# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w6-1`, `w6-2`, `w4-4`, and `design-3` all
landed and graded through the codex loop in one session —
ProgressEvent contract in blit-core; §1.6 residue filed as
w6-2a/-2b/-2c; blocking filesystem work off the tokio runtime;
**bounded data-plane dials**). **Owner pushed `master` → `github` at
`10d89e0`** (2026-07-04); the `origin` gitea mirror was not pushed
and lags. windows-latest CI on this push is the "meaningfully green"
check the Open questions entry anticipates.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **design-3 DONE — bounded data-plane dials** (`49dcec6`; finding
  `.review/findings/design-3-unbounded-data-plane-connects.md`, impl
  record appended). Shared `socket::dial_data_plane`: connect bounded
  by `DATA_PLANE_ACCEPT_TIMEOUT`, w1-2 policy, handshake write
  bounded by `DATA_PLANE_TOKEN_TIMEOUT`; TimedOut in the chain →
  `is_retryable` transient. Both client sites collapsed (pull
  `connect_pull_stream` incl. resize-ADD; push `connect_with_probe`
  incl. elastic). Was: kernel SYN-timeout hangs (60–127 s) on
  black-holed ephemeral data ports. Codex: **PASS, 0 findings**. +3
  tests (deterministic stalled-handshake shape pin,
  mutation-verified); workspace 1476 → 1479/0/2.
- **w4-4 DONE — blocking work off the runtime** (`0feca34`+fix
  `768e7e3`; finding
  `.review/findings/w4-4-blocking-work-off-runtime.md`). Push
  manifest requires-upload checks (canonical containment walk + stat
  — ~3M+ blocking syscalls per 1M-file push, previously inline on a
  tokio worker) now run in chunked `spawn_blocking` batches
  (chunk=128 = need-list early-flush threshold; lexical-containment
  alternative rejected — weakens F2); need-list order kept,
  mid-manifest TCP spin-up post-chunk-drain, design-4 untouched.
  `collect_pull_entries_with_checksums` runs entirely on one
  `spawn_blocking` thread (single-file `--checksum` branch was
  hashing whole files inline on a worker). Codex: **NEEDS FIXES (1
  Medium, accepted — chunk-only draining muted the batcher's 5 ms
  early-flush for trickling manifests)** → `manifest_drain_due`
  chunk-or-delay trigger, fixed `768e7e3`. +4 tests,
  mutation-verified; workspace 1472 → 1476/0/2.
- **Earlier same day: w6-2 verify-then-file** (`0aba593`+`8b7829d`):
  all three §1.6 residue claims CONFIRMED (delegated live progress
  wire-dead; daemon counters 0 for push/pull_sync; denominators
  hardcoded 0); filed as independent rows **w6-2a/-2b/-2c**
  (pending-review section). Codex: 2 Low doc-coherence, fixed.
- **w6-1 DONE — ProgressEvent contract** (`8fd8978`; finding
  `.review/findings/w6-1-progress-event-contract.md`). Contract on
  the enum in blit-core: **bytes ride `Payload` only —
  `FileComplete.bytes` deleted** (design-1's class unrepresentable);
  files count once via byteless `FileComplete{wire path}` or
  `Payload.files` (aggregate lane); `ManifestBatch` = documented
  denominator. All producers normalized (double-emit, tar-shard +
  resume gaps, absolute-path leak, dead emitters); consumers
  collapsed onto shared `ProgressTotals` (CLI + 3 TUI forwarders;
  TUI's 3 rules deleted). **design-1 closed alongside.** Codex:
  **PASS, 0 findings**. +12 tests, 2 mutation checks; 1460 → 1472.
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

1. **Design-review queue** — `REVIEW.md` order governs. w6-1,
   design-1, w6-2, w4-4, and design-3 all closed `[x]` 2026-07-04
   (see Now). Strict row order gives **w9-3** (test-harness builder,
   Medium — `TestContext::builder()` consolidating 5 harness clones +
   5 cli_bin copies, OnceLock daemon build, fake-server keepalive
   parity; also the home of the daemon-spawn e2e load-flakiness) as
   the topmost ratified open row — sized right for a fresh session.
   Filed alternatives (pending-review section, coder's pick):
   **w6-2a/-2b/-2c** (daemon progress residue — independent slices,
   2b→2a→2c smallest-first suggestion) and Low
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

- **2026-07-04 (18th)** @ `49dcec6`+records —
  **design-3-unbounded-data-plane-connects landed and graded** (same
  session, fourth slice; coder's pick of the sanctioned smaller
  alternative over the large w9-3). Shared `dial_data_plane`
  (bounded connect + policy + bounded handshake write, TimedOut →
  retryable); both client sites collapsed. Codex: **PASS 0
  findings**. +3 tests, mutation-verified; workspace 1476 → 1479/0/2
  across 37 suites, fmt/clippy clean (macOS host). Session closed
  w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3.
  In-flight: none. **Exact first action next session**: standing
  "reviewloop" go → pick up **w9-3** (test-harness builder, topmost
  ratified open row, sized for a fresh session; w6-2a/b/c + relay-1
  are the filed coder's-pick alternatives) through the codex loop.
  Nothing pushed — push stays owner-gated.
- **2026-07-04 (17th)** @ `768e7e3`+records —
  **w4-4-blocking-work-off-runtime landed and graded**. Push manifest
  checks → chunked spawn_blocking (design-4 untouched, F2 canonical);
  pull_sync enumeration fully off-runtime. Codex: NEEDS FIXES 1
  Medium (chunk-only draining muted the 5 ms early-flush for
  trickling manifests) → chunk-or-delay `manifest_drain_due`, fixed
  `768e7e3`. +4 tests mutation-verified; 1472 → 1476/0/2. Nothing
  pushed.
- **2026-07-04 (16th)** @ `8b7829d`+records —
  **w6-2-progress-residue-verify landed and graded**. All three §1.6
  claims CONFIRMED; filed as w6-2a/-2b/-2c. Codex: NEEDS FIXES 2 Low
  (doc-coherence) → fixed `8b7829d`. No code; 1472/0/2.
