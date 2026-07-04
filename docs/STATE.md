# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (**w9-3 test-harness consolidation landed
and graded** — one daemon-spawn harness, OnceLock'd build,
fake-server keepalive parity, and the daemon-spawn port-collision
flake root-caused and fixed). **Owner pushed `master` → GitHub
(`origin`) at `10d89e0`** (2026-07-04); the `gitea` LAN mirror is
also at `10d89e0`. windows-latest CI on that push is the
"meaningfully green" check the Open questions entry anticipates;
`f6e592e`..`c62d15b` are local on top of it, unpushed.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **w9-3 DONE — test-harness consolidation** (`f6e592e` + review fix
  `8641bc6` + records `c62d15b`; finding
  `.review/findings/w9-3-test-harness-builder.md`). One daemon-spawn
  harness in `tests/common`: `TestContext::builder()`
  (read_only/delegation/extra_daemon_args) + `spawn_second_daemon`
  absorbed the SEVEN clones at HEAD (audit counted 5; w9-4/w9-5 had
  each added one) plus 5 cli_bin / 7 run_with_timeout / 4 ChildGuard
  copies; daemon build OnceLock'd per binary (R16-F1 kept, ~75 nested
  cargo builds → ≤1/binary); new `blit_core::remote::grpc_server`
  owns the audit-1 keepalive (30s/20s) — daemon + all FIVE fake tonic
  servers route through `production_server_builder()`, zero bare
  `Server::builder()` left. **Daemon-spawn load-flakiness root-caused
  live mid-slice** (port TOCTOU: probe-drop-to-bind gap, previously
  masked by per-test builds serializing bring-ups) and fixed two-layer
  (process-global claimed-port set + child-death readiness check).
  Codex: **NEEDS FIXES, 1 Medium accepted** (fake-server `:0` bind
  bypassed the claimed set — same race class, missed path) → fixed.
  Net −1,251 test-tree lines; count gate by same-method A/B:
  1478 → 1479/0/2 (+1 mutation-verified keepalive pin; the previously
  recorded "1479" baseline was a different aggregation).
- **Earlier 2026-07-04 (four sessions): design-3, w4-4, w6-2, w6-1
  (+design-1), w3-1, w2-2, w4-5, W1 family, w4-1, w4-3 all `[x]`**
  (details: DEVLOG 2026-07-04 entries; findings + verdicts in
  `.review/`): bounded data-plane dials design-3 `49dcec6` (codex
  PASS 0); blocking work off the runtime w4-4 `0feca34`+`768e7e3`;
  §1.6 residue filed as **w6-2a/-2b/-2c** (w6-2 `0aba593`);
  ProgressEvent contract in blit-core w6-1 `8fd8978` (design-1 closed
  alongside); memory-aware BufferPool + sysinfo 1024× bug w3-1
  `f49f8f6`; dial = single stream/chunk owner w2-2; w4-5
  cancellation flip (D-2026-07-04-3); w1-2/-3/-4 socket policy;
  w4-1 AbortOnDrop; w4-3 disconnect racing.
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

1. **Design-review queue** — `REVIEW.md` order governs. w9-3 closed
   `[x]` 2026-07-04 (see Now). Strict row order gives **w7-1**
   (mirror-executor consolidation, Medium — one mirror/purge deletion
   executor + parallel enumerate_local_manifest in blit-core, R58-F3
   class closure; four diverged copies today, only two clear Windows
   read-only) as the topmost ratified open row. Filed alternatives
   (pending-review section, coder's pick): **w6-2a/-2b/-2c** (daemon
   progress residue — independent slices, 2b→2a→2c smallest-first
   suggestion) and Low `relay-1-subpath-double-join`.
2. **10 GbE benchmark session — owner-gated** (env:
   `admin@skippy:/mnt/generic-pool/video/test`, scp/ssh open; ping the
   owner if a daemon can't run on skippy). This is the REV4 sign-off:
   `ue-1` loopback parity band, `ue-2` continuous/resize behavior
   under real load, zero-copy revisit gate (D-2026-06-12-1).
   **Host plan (owner, 2026-07-04)**: sign-off pair = TrueNAS
   (skippy) ↔ **Arch client**, all-Linux — the zero-copy/splice gate
   needs a Linux consumer, and the parity band should measure the
   engine, not Windows I/O quirks. The client box dual-boots
   Win 11/Arch (identical hardware → clean Win-vs-Linux delta):
   after the Linux gates close, boot Win 11 bare-metal for a
   TrueNAS→Win pull datapoint in the same window (deployment parity,
   not a gate). The Win VM on the Arch install is for
   Windows-specific *functional* checks only — never perf numbers
   (virtio/NAT skews throughput). iperf3 baseline per pair before
   any Blit numbers (the parity band is defined against it). After
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
  locally across three sessions (clippy baseline + win-1 fixed). The
  daemon-spawn e2e load-flakiness is now root-caused and fixed on
  Linux (w9-3: port-TOCTOU wrong-daemon race + cargo-lock contention;
  claimed-port set + OnceLock build + child-death check). Remaining
  check: windows-latest CI on the next push (10d89e0 predates the
  w9-3 fix, so daemon-spawn flakes there would not be news).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (20th)** @ `c62d15b` —
  **w9-3-test-harness-builder landed and graded** (owner go:
  "continue, use /playbook reviewloop codex" — no playbooks in this
  repo; resolved to `slice`). One harness (builder + second-daemon +
  OnceLock build + keepalive parity via new
  `blit_core::remote::grpc_server`); the daemon-spawn port-collision
  flake was caught live during validation and fixed (claimed-port set
  + child-death check). Codex: NEEDS FIXES 1 Medium (fake-server bind
  bypassed the claimed set) → fixed `8641bc6`; records `c62d15b`.
  Gate: fmt/clippy clean; 1478 → 1479/0/2 same-method A/B; full suite
  ×2 + admin_verbs ×10 green. In-flight: none. **Exact first action
  next session**: standing "reviewloop" go → **w7-1**
  (mirror-executor consolidation, topmost ratified open row) through
  the codex loop; alternatives w6-2a/b/c + relay-1 (coder's pick).
  Nothing pushed — push stays owner-gated.
- **2026-07-04 (19th)** @ `c609192`+docs — **push recorded + 10 GbE
  host plan settled** (owner Q&A). Owner pushed `master` → `github`
  at `10d89e0`; gitea mirror lags. Benchmark sign-off pair decided:
  TrueNAS ↔ Arch (all-Linux; splice gate + clean parity band), Win 11
  bare-metal datapoint after on the same dual-boot hardware, Win VM
  for functional checks only — recorded in Queue item 2. No code.
  In-flight: none. **Exact first action next session**: standing
  "reviewloop" go → **w9-3** (test-harness builder) through the codex
  loop; the owner will call "benchmark" for the 10 GbE session.
- **2026-07-04 (18th)** @ `49dcec6`+records —
  **design-3-unbounded-data-plane-connects landed and graded** (same
  session, fourth slice; coder's pick of the sanctioned smaller
  alternative over the large w9-3). Shared `dial_data_plane`
  (bounded connect + policy + bounded handshake write, TimedOut →
  retryable); both client sites collapsed. Codex: **PASS 0
  findings**. +3 tests, mutation-verified; workspace 1476 → 1479/0/2
  across 37 suites, fmt/clippy clean (macOS host). Session closed
  w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3.
  In-flight: none. Next action was w9-3 — done, see the 20th entry.
  Nothing pushed — push stays owner-gated.
