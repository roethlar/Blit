# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w6-1` landed and graded through the codex
loop — **the ProgressEvent contract lives in blit-core**: bytes ride
`Payload` only, `FileComplete` is byteless, one shared `ProgressTotals`
fold; closes design-1 structurally); local HEAD `8fd8978`+records,
**not yet pushed** to either remote across these sessions.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **w6-1 DONE — ProgressEvent contract** (`8fd8978`; finding
  `.review/findings/w6-1-progress-event-contract.md`). The contract is
  defined ON the enum in blit-core: **bytes ride `Payload` only —
  `FileComplete`'s `bytes` field is deleted**, so the design-1
  double-count class is unrepresentable at the type level; files count
  exactly once via either one byteless `FileComplete{wire-relative
  path}` (per-file lane) or `Payload.files` deltas (aggregate lane:
  delegated bridge, gRPC tar-shard applier); `ManifestBatch` is the
  documented direction-flavored denominator. All producers normalized
  (TCP receive double-emit fixed; tar-shard members + TCP/gRPC resume
  lanes gain their missing events; send side moves planned bytes onto
  `Payload`; gRPC pull's absolute-path leak fixed; both dead emitters
  conformed pending w8 deletion). Consumers collapsed onto shared
  `ProgressTotals`: CLI monitor (fixes design-1's ~2× bytes on TCP
  pulls; `--json` `file_complete` keeps its shape with `bytes:0`) +
  all three TUI forwarders; the TUI's three `accumulate_*` rules
  deleted. **design-1 closed `[x]` alongside**, graded in the same
  round. Codex: **PASS, 0 findings** (checked the W6.1/W6.2 split —
  daemon/ByteProgressSink counters deliberately untouched). +12
  blit-core tests incl. exact-sequence emission pins, 2 mutation
  checks; workspace 1460 → 1472/0/2 across 37 suites.
- **Earlier 2026-07-04: w3-1, w2-2, w4-5, W1 family, w4-1, w4-3 all
  `[x]`** (details: DEVLOG 2026-07-04 entries; findings
  `.review/findings/`): memory-aware BufferPool + sysinfo 1024× bug
  w3-1 `f49f8f6`; dial = single stream/chunk owner w2-2
  `01209bc`+`27f53a0`; `supports_cancellation` flipped w4-5
  `05a8b39`+`1708075` (D-2026-07-04-3); socket policy helper w1-2
  `16237e2`; real keepalive timing w1-3 `865fc1e`; shared accept/token
  bounds w1-4 `6a19e1d`+`d17b089`; AbortOnDrop family w4-1;
  disconnect racing w4-3 `37d7f91`.
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

1. **Design-review queue** — `REVIEW.md` order governs. w6-1 (and
   design-1 with it) closed `[x]` 2026-07-04 (see Now), the
   ninth/tenth rows that day. Strict row order now gives **w6-2**
   (progress-residue verify-then-fix, Medium — the §1.6 residue:
   delegated zero live progress since `BytesProgress` is wire-dead,
   daemon counters 0 for push/pull_sync rows, no denominators
   end-to-end; verification is step 1, each confirmed item becomes its
   own follow-on slice) as the topmost open row; **design-3**
   (data-plane connect timeouts, filed-findings section) remains the
   sanctioned smaller alternative (two client connect sites, bound
   imports the shared `DATA_PLANE_ACCEPT_TIMEOUT`) — sequencing stays
   the coder's pick unless the owner orders otherwise. Open Low rows:
   `relay-1-subpath-double-join`.
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

- **2026-07-04 (15th)** @ `8fd8978`+records+docs —
  **w6-1-progress-event-contract landed and graded through the codex
  loop** (owner go: "continue. reviewloop with codex as each slice
  lands" → topmost open row per the 14th handoff). ProgressEvent
  contract defined on the enum in blit-core: bytes ride `Payload`
  only (`FileComplete.bytes` DELETED — design-1's class
  unrepresentable), files count once via byteless
  `FileComplete{wire path}` or `Payload.files` (aggregate lane),
  `ManifestBatch` = documented denominator. All producers normalized
  (double-emit, tar-shard/resume gaps, absolute-path leak, dead
  emitters); consumers collapsed onto shared `ProgressTotals` (CLI +
  3 TUI forwarders; TUI's 3 rules deleted). design-1 closed `[x]` in
  the same round. Codex: **PASS 0 findings**. +12 blit-core tests, 2
  mutation checks; workspace 1460 → 1472/0/2 across 37 suites,
  fmt/clippy clean (macOS host). In-flight: none. **Exact first
  action next session**: owner's standing "reviewloop with codex as
  each slice lands" go → pick up **w6-2** (progress-residue
  verify-then-fix, topmost open row; design-3 remains the sanctioned
  smaller alternative) through the codex loop.  Nothing pushed — push
  stays owner-gated.
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
  **w2-2-stream-ladder-owner landed and graded** (owner go:
  "continue"). Three stream ladders were already gone post-REV4; the
  slice deleted the planner's dead chunk lane (ladder,
  `Plan`/`PlannedPayloads`, `chunk_bytes_override`, `TuningParams`);
  dial = single chunk owner; byte-identical wire behavior. Codex:
  NEEDS FIXES 1 Low (comment-truth → `27f53a0`). 4 transfer_plan
  pins. Workspace 1448 → 1452/0/2. Discoveries → Open questions
  (worktree snapshot `725aa07`; WHITEPAPER drift). Nothing pushed.
