# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`ue-r2-2` complete — **REV4 is complete**:
all nine slices landed through the code→review→fix loop; the three
static ladders are gone and stream resize is live end-to-end);
unpushed to `origin`/gitea: everything after `7603177`.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-2` COMPLETE — REV4 COMPLETE** (details: DEVLOG 2026-07-04).
  Stream resize: engine-owned policy (`resize_tick`: ±1 stream/epoch,
  cheap-dials-first escalation, sustain 2 / cooldown 4 busy ticks,
  bounded by the receiver profile), elastic pipeline
  (`SinkControl::Add`/`RetireOne`; a retired worker's END record IS the
  receiver-side teardown), push (client controller/dialer + daemon
  armed-only acceptor; sockets identified by which sub-token they
  echo), pull (daemon controller + client growable JoinSet receiver;
  delegated inherits via `dst_capabilities`). Commits
  `042ca4b`..`0788e83` + review fix `ec4a3fe`; tests **1405 / 0 / 3
  (Windows host)**. Review: codex NEEDS FIXES (3) + 3-lens panel (4) →
  9 fixed, 1 deferred — headline catches: pull resize was DEAD on the
  CLI path (a dropped edit; hand-built-spec wire tests masked it), no
  cumulative stream bound at the acking ends, the pull controller's
  inline handshake freeze. Post-REV4 residue, recorded in the finding
  doc: pull 1s-start (needs pull negotiation ahead of enumeration — a
  restructuring); epoch-0-loop-vs-early-ADD hardening (Low, deferred);
  BufferPool live growth (W3.1); telemetry-triggered resize behavior
  is measured at the 10 GbE sign-off per the plan.
- **`ue-r2-1b`..`ue-r2-1h` COMPLETE** — wire contract, engine shell,
  streaming plan, live dials, push converge, pull multistream, Pull-RPC
  deletion (details: DEVLOG 2026-07-03/04 entries; REVIEW.md has the
  commit map). All three static ladders retired (1e/1f/1h).
- **Windows-host session artifacts (2026-07-04)**: first fully-gated
  sessions on the owner's Windows machine — fixed pre-existing
  `9f37a7a` (clippy baseline) + `48c5a11` (win-1 High: push need-list
  backslash echo). Erratum: `9f37a7a`/`48c5a11` don't build in
  isolation (a staging slip put the pull.rs deletion in the wrong
  commit; bisect must skip them; HEAD fully gated) — owner may
  authorize a history fix of the unpushed stack or leave it.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5) and is now **code-complete**; its remaining
    acceptance items are measurement gates (loopback parity band,
    1s-start verification, 10 GbE sign-off `ue-1`/`ue-2`) owned by the
    owner's benchmark session. Flipping REV4 to Shipped is an owner
    call after that session.
  - Process (D-2026-06-20-6, `docs/agent/GPT_REVIEW_LOOP.md`) governed
    `ue-r2-*`; with the slices done, the `.review/README.md` async loop
    + REVIEW.md queue govern what's next. Owner gates remaining:
    **push**, **10 GbE sign-off**.

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. Highest open
   ratified row is **w4-1** (AbortOnDrop family, High; design-2 now
   scopes to `push/control.rs` only). Then w4-3, W1 socket-policy /
   timeout rows (note: ue-r2-2's armed-only accepts re-ratified the 1g
   W1 deferral premise; the constants/policy consolidation still
   belongs to W1). Open Low rows from the ue-r2 reviews:
   `relay-1-subpath-double-join`.
2. **10 GbE benchmark session — owner-gated** (env:
   `admin@skippy:/mnt/generic-pool/video/test`, scp/ssh open; ping the
   owner if a daemon can't run on skippy). This is the REV4 sign-off:
   `ue-1` loopback parity band, `ue-2` continuous/resize behavior
   under real load, zero-copy revisit gate (D-2026-06-12-1). After
   `ue-1`: audit Round 1, TUI rework, H10b planner.
3. **Post-REV4 residue** (unowned until the owner slots them): pull
   1s-start restructuring; epoch-0/early-ADD hardening; remote
   perf-history lanes (1e gap); dead `derive_local_plan_tuning`
   fold-or-retire (w2-2).

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
  code-complete; measurement gates remain (see Active context).
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `.review/README.md` async loop for queue work;
  `docs/agent/GPT_REVIEW_LOOP.md` (Active) applied to the now-finished
  `ue-r2-*` slices.
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified
  D-2026-06-12-1, executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (loop).

## Blocked / waiting

- **Owner**: push approval for `origin..master` (everything after
  `7603177` — the 1f/1g/1h/2 stacks + Windows fixes + records).
- **Owner call on the commit erratum** (`9f37a7a`/`48c5a11` unbuildable
  in isolation): leave as-is (default) or authorize a rewrite of the
  unpushed stack.
- **Owner**: 10 GbE session scheduling (REV4 sign-off + zero-copy
  revisit + resize behavior measurement).
- `Cargo.lock`: the pre-existing dependency-refresh drift was
  committed at `04c9c6d` out of necessity (blit-core gained `rand`,
  which cannot land without its lockfile edge; every gate this session
  ran against the drifted lockfile). The owner's pending
  commit-or-regenerate question is thereby answered "committed" —
  revert selectively if unwanted.

## Open questions

- **(OPEN)** Edit D-2026-06-20-1 to strip its superseded warmup/size-gate
  wording? Owner: not sure. (Agent rec: edit with a one-line note → -2/-5.)
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(OPEN)** REV4 → Shipped flip: after the 10 GbE session, or now
  with the measurement gates tracked separately? Owner call.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed); the
  daemon-spawn e2e family shows load-flakiness under full-parallel
  runs (w9-3 territory). windows-latest CI on the next push should be
  meaningfully green.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (8th)** @ `8d62afc` — `ue-r2-2` landed end-to-end
  (`042ca4b`..`0788e83`; codex NEEDS FIXES 3 + panel 4 → 9 fixed
  `ec4a3fe`, 1 deferred; records `8d62afc`). **REV4 code-complete.**
  fmt/clippy clean; tests 1405/0/3 on Windows. In-flight: none — REV4
  boundary. **Exact first action next session**: on owner "continue",
  pick up the design-review queue at **w4-1** (AbortOnDrop family)
  through the `.review/` loop; else owner pushes the stack / schedules
  the 10 GbE sign-off / decides the erratum + D-2026-06-20-1
  questions.
- **2026-07-04 (7th)** @ `f6f52d7`+docs — `ue-r2-1h` landed end-to-end
  (`2a13f53` deletion+port; codex NEEDS FIXES 3 + panel → 5 fixed
  `f6f52d7`, 1 deferred relay-1, 1 rejected; plus Windows-host
  pre-existing fixes `9f37a7a` clippy baseline + `48c5a11` win-1
  push-separator). fmt/clippy clean; tests 1393/0/3 on Windows.
- **2026-07-03 (6th)** @ `4a2e58d`+docs — `ue-r2-1g` landed end-to-end
  (`48e583e` multistream + engine proposal; codex NEEDS FIXES → 2
  fixed + self-review panel 2 fixed / 1 deferred, `4a2e58d`).
  fmt/clippy clean; tests 1413/0/2.
