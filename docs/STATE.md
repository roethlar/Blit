# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w3-1` landed and graded through the codex
loop — **`BufferPool::for_data_plane` owns the pool formula + the
available-memory cap**; sysinfo 1024× units bug fixed); local HEAD
`f49f8f6`+records, **not yet pushed** to either remote across these
sessions.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **w3-1 DONE — memory-aware BufferPool** (`f49f8f6`; finding
  `.review/findings/w3-1-memory-aware-buffer-pool.md`).
  `BufferPool::for_data_plane(chunk_bytes, streams)` owns the formula
  (streams×2+4, shared 64 KiB `DATA_PLANE_BUFFER_FLOOR`) plus an
  available/4 memory cap with a **2-buffers-per-stream liveness
  floor** (the double-buffered sender holds 2, acquired sequentially —
  a tighter cap shrinks buffer size, never concurrency, so the cap can
  never deadlock a sender). Replaces the 3 pasted sites (push client,
  pull_sync multistream, pull_sync resume — whose pool is inert at
  runtime, verified). Elastic paths authorize
  `dial.ceiling_max_streams()` up front (lazy allocation → free until
  resize ADDs streams), closing both sites' "growing the pool live is
  a W3.1 concern" deferral. **Bonus bug fixed**: the old
  available-memory helper multiplied sysinfo by 1024, but sysinfo 0.38
  returns bytes — memory was over-reported 1024×, making every
  downstream cap vacuous (incl. BufferSizer's /10). Comment-truth:
  `RECEIVE_CHUNK_SIZE`'s false "matches the send side" claim
  rewritten. Codex: **PASS, 0 findings**. 8 params-layer pins,
  mutation-verified; workspace 1452 → 1460/0/2 across 37 suites.
- **Earlier 2026-07-04: w2-2, w4-5, W1 family, w4-1, w4-3 all `[x]`**
  (details: DEVLOG 2026-07-04 entries; findings
  `.review/findings/w2-2-*.md`, `w4-5-*.md`, `w1-*.md`, `w4-*.md`):
  dial = single stream/chunk owner w2-2 `01209bc`+`27f53a0` (planner's
  dead chunk lane deleted; tuning owner = `engine::TransferDial`);
  `supports_cancellation` flipped for Push/PullSync w4-5
  `05a8b39`+`1708075` (D-2026-07-04-3 executed); socket policy helper
  w1-2 `16237e2`; real keepalive timing w1-3 `865fc1e`; shared
  accept/token bounds w1-4 `6a19e1d`+`d17b089`; AbortOnDrop family
  w4-1; disconnect racing w4-3 `37d7f91`.
- **REV4 code-complete** (`ue-r2-1b`..`ue-r2-2`, all nine slices;
  details: DEVLOG 2026-07-03/04 entries, REVIEW.md commit map). Stream
  resize live end-to-end; all three static stream-count ladders
  retired. Remaining acceptance items are measurement gates (loopback
  parity band, 1s-start verification, 10 GbE sign-off `ue-1`/`ue-2`)
  owned by the owner's benchmark session. Residue: see Queue item 3.
- **Windows-host sessions (2026-07-04)**: suite fully green on the
  owner's Windows machine (`9f37a7a` clippy baseline + `48c5a11`
  win-1). Erratum settled (D-2026-07-04-2): those two commits don't
  build in isolation (staging slip); stay as pushed, bisect skips
  them; HEAD fully gated.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5), code-complete; flipping to Shipped is an
    owner call after the 10 GbE benchmark session.
  - Process: the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`) now
    governs **all code and plan changes** (D-2026-07-04-1, owner: "no
    exceptions"); the `.review/README.md` async sentinel loop is
    retired. REVIEW.md stays the queue/status index.

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. w3-1 closed
   `[x]` 2026-07-04 (see Now), the eighth row that day. Strict row
   order now gives **w6-1** (ProgressEvent contract, Medium — defines
   semantics in blit-core, normalizes producers, shared accumulator,
   incorporates design-1's byte double-count) as the topmost open row;
   **design-3** (data-plane connect timeouts, filed-findings section)
   remains the sanctioned smaller alternative (two client connect
   sites, bound imports the shared `DATA_PLANE_ACCEPT_TIMEOUT`) —
   sequencing stays the coder's pick unless the owner orders
   otherwise. Open Low rows: `relay-1-subpath-double-join`.
2. **10 GbE benchmark session — owner-gated** (env:
   `admin@skippy:/mnt/generic-pool/video/test`, scp/ssh open; ping the
   owner if a daemon can't run on skippy). This is the REV4 sign-off:
   `ue-1` loopback parity band, `ue-2` continuous/resize behavior
   under real load, zero-copy revisit gate (D-2026-06-12-1). After
   `ue-1`: audit Round 1, TUI rework, H10b planner.
3. **Post-REV4 residue** (unowned until the owner slots them): pull
   1s-start restructuring; epoch-0/early-ADD hardening; remote
   perf-history lanes (1e gap); `derive_local_plan_tuning`
   fold-or-retire (statically live on the local engine path but
   dynamically dead — nothing fills the tar/raw telemetry buckets
   since `4ce4898`, 2026-04-07; verified during the w2-2 audit,
   design decision not review-queue material); receive-side dial
   tuning (rest of constants-receive-chunk-1mib-asymmetry — w3-1
   scoped it out, wire needs no change; separate slice if wanted).

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  code-complete; measurement gates remain (see Active context).
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (loop).

## Blocked / waiting

- **10 GbE session** (REV4 sign-off + zero-copy revisit + resize
  behavior measurement). Owner 2026-07-04: **"soon, but keep coding
  first"** — keep working the review queue; the owner will call
  "benchmark" when the hardware session is on. Not a daily blocker.
- `Cargo.lock`: the pre-existing dependency-refresh drift was
  committed at `04c9c6d` out of necessity (blit-core gained `rand`,
  which cannot land without its lockfile edge; every gate this session
  ran against the drifted lockfile). The owner's pending
  commit-or-regenerate question is thereby answered "committed" —
  revert selectively if unwanted.

## Open questions

- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(OPEN, new 2026-07-04)** `725aa07` ("chore: track claude
  worktrees?") committed 236 files of a stale worktree snapshot at
  `.claude/worktrees/vigilant-mayer/` into the repo — including a full
  copy of `crates/` sources. Keep or `git rm -r`? Agent rec: remove
  (it's a stale duplicate that pollutes grep/audit sweeps); deletion
  awaits an owner go since the tracking commit looks deliberate-ish.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
- **(OPEN)** REV4 → Shipped flip: after the 10 GbE session, or now
  with the measurement gates tracked separately? Owner call (10 GbE
  now "soon" — likely resolves with it).
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed); the
  daemon-spawn e2e family shows load-flakiness under full-parallel
  runs (w9-3 territory). windows-latest CI on the next push should be
  meaningfully green.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (14th)** @ `f49f8f6`+records+docs —
  **w3-1-memory-aware-buffer-pool landed and graded through the codex
  loop** (owner go: "continue" → topmost open row per the 13th
  handoff). `BufferPool::for_data_plane(chunk_bytes, streams)`:
  formula + 64 KiB floor + available/4 cap + 2-buffers-per-stream
  liveness floor (cap shrinks buffers, never concurrency — deadlock-
  proof by construction); 3 pasted sites replaced; elastic paths
  authorize `ceiling_max_streams()` up front. Sysinfo 1024× units bug
  fixed (0.38 returns bytes; old helper's ×1024 made every memory cap
  vacuous). Design assumptions verified by a 5-agent audit workflow
  before coding (two-buffer hold-and-wait, wire tolerance of shrunk
  buffers, hard resize ceiling, resume pool inert). Codex: **PASS 0
  findings** (first invocation killed by a session restart before
  output; record is the complete re-run). 8 params pins mutation-
  verified. Workspace 1452 → 1460/0/2 across 37 suites, fmt/clippy
  clean (macOS host). In-flight: none. **Exact first action next
  session**: on owner "continue", pick up **w6-1** (ProgressEvent
  contract, topmost open row; design-3 remains the sanctioned smaller
  alternative) through the codex loop. Nothing pushed — push stays
  owner-gated.
- **2026-07-04 (13th)** @ `27f53a0`+records+docs —
  **w2-2-stream-ladder-owner landed and graded through the codex
  loop** (owner go: "continue" → topmost open row per the 12th
  handoff). Row re-scoped to post-REV4 reality before coding: the
  three stream ladders were already gone (ue-r2-1e/-1f/-1h,
  D-2026-06-20-1), so the slice deleted the remaining leg — the
  planner's dead chunk lane (ladder, `Plan`/`PlannedPayloads`
  wrappers, `chunk_bytes_override` + 5 refresh sites, never-called
  `plan_to_daemon_format`, orphaned `TuningParams`); dial =
  single chunk owner; byte-identical wire behavior. Codex: NEEDS
  FIXES 1 Low (comment-truth → `27f53a0`). 4 new transfer_plan pins.
  Workspace 1448 → 1452/0/2. New discoveries → Open questions:
  tracked stale worktree snapshot `725aa07`; WHITEPAPER tuning drift.
  Nothing pushed.
- **2026-07-04 (12th)** @ `1708075`+records+docs —
  **w4-5-supports-cancellation-flip landed and graded** (owner go:
  "continue"). D-2026-07-04-3 executed: CancelJob dispatch flipped on
  for attached Push/PullSync (one predicate; Pull history-only stays
  gated), every old-policy comment surface updated (3-agent workflow
  sweep; TUI/CLI have no kind gating — zero logic changes). Codex:
  NEEDS FIXES 1 Low (module scope-log rustdoc → `1708075`). Four new
  contract pins mutation-verified. blit-daemon 168 → 170; workspace
  1448/0/2 across 37 suites. Known gap: no e2e drives a live
  mid-flight attached cancel (needs a test seam; w4-3 evidence
  shape). Nothing pushed.
