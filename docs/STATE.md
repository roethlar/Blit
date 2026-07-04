# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w4-1` landed via the `.review/` coder loop —
AbortOnDrop hoisted, remaining detach-on-drop sites closed, closes
`design-2` as a byproduct); local HEAD `44bf416`, **not yet pushed** to
either remote this session.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`w4-1-abortondrop-family` landed, pending review** (details: DEVLOG
  2026-07-04T05:00:00Z; finding: `.review/findings/w4-1-abortondrop-family.md`).
  Hoisted `AbortOnDrop` from `blit-core/src/remote/pull.rs` (`pub(crate)`)
  to `blit-core::remote::transfer::abort_on_drop` (`pub`); wrapped the
  remaining detach-on-drop sites: daemon push `data_plane_handle`
  (design-2's last site), push client `pipeline_handle` +
  `response_task`, and converted the daemon's per-stream push worker
  `Vec<JoinHandle>` to a `JoinSet` (mirrors the resizable path's
  existing `ue-r2-2` fix). Commits `65ecb93` (fix) + `44bf416` (finding
  doc/REVIEW.md/sentinel); fmt/clippy clean; `cargo test --workspace`
  green (blit-core 348, blit-daemon 162). Sentinel written to
  `.review/ready/w4-1-abortondrop-family.json` — **awaiting reviewer
  verdict**; also closes `design-2-orphaned-daemon-data-planes`'s
  remaining scope (same commit).
- **REV4 code-complete** (`ue-r2-1b`..`ue-r2-2`, all nine slices; details:
  DEVLOG 2026-07-03/04 entries, REVIEW.md commit map). Stream resize is
  live end-to-end (engine-owned `resize_tick` policy, elastic pipeline,
  push armed-only acceptor, pull growable JoinSet receiver); all three
  static stream-count ladders retired. Remaining acceptance items are
  measurement gates (loopback parity band, 1s-start verification, 10
  GbE sign-off `ue-1`/`ue-2`) owned by the owner's benchmark session.
  Post-REV4 residue: pull 1s-start restructuring; epoch-0-loop-vs-early-ADD
  hardening (Low, deferred); BufferPool live growth (W3.1).
- **Windows-host session artifacts (2026-07-04)**: first fully-gated
  sessions on the owner's Windows machine — fixed pre-existing
  `9f37a7a` (clippy baseline) + `48c5a11` (win-1 High: push need-list
  backslash echo). Erratum: `9f37a7a`/`48c5a11` don't build in
  isolation (a staging slip put the pull.rs deletion in the wrong
  commit; bisect must skip them; HEAD fully gated) — owner may
  authorize a history fix of the now-pushed stack or leave it.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5), code-complete; flipping to Shipped is an
    owner call after the 10 GbE benchmark session.
  - Process (D-2026-06-20-6, `docs/agent/GPT_REVIEW_LOOP.md`) governed
    `ue-r2-*`; with the slices done, the `.review/README.md` async loop
    + REVIEW.md queue govern what's next (current example: w4-1 above).

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. w4-1 landed
   2026-07-04, pending review (see Now). Highest still-open ratified
   row is **w4-3** (daemon disconnect racing). Then W1 socket-policy /
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

- **Reviewer verdict on `w4-1-abortondrop-family`**: sentinel written to
  `.review/ready/w4-1-abortondrop-family.json` (branch `master`, sha
  `65ecb93`); needs a reviewer pass per `.review/README.md` before the
  REVIEW.md row flips `[~]` → `[x]`.
- **Owner call on the commit erratum** (`9f37a7a`/`48c5a11` unbuildable
  in isolation, now pushed to both remotes): leave as-is (default) or
  authorize a history rewrite to fix it.
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

- **2026-07-04 (9th)** @ `44bf416` — `w4-1-abortondrop-family` landed via
  the `.review/` coder loop (fix `65ecb93`; finding/REVIEW.md/sentinel
  `44bf416`). Hoisted `AbortOnDrop`; closed the daemon push
  `data_plane_handle`, push-client `pipeline_handle`/`response_task`,
  and daemon per-stream-worker `Vec→JoinSet` detach-on-drop sites;
  closes `design-2` as a byproduct. fmt/clippy clean; tests green
  (blit-core 348, blit-daemon 162). In-flight: none — sentinel out,
  awaiting reviewer verdict. **Exact first action next session**: if
  the reviewer has graded it, act on the verdict (merge/close on
  Accepted, or address `.review/results/w4-1-abortondrop-family.reopened.md`
  on Reopened); otherwise pick up the next design-review queue row,
  **w4-3** (daemon disconnect racing). Not pushed to either remote this
  session — confirm with the owner before pushing.
- **2026-07-04 (8th)** @ `8d62afc` — `ue-r2-2` landed end-to-end
  (`042ca4b`..`0788e83`; codex NEEDS FIXES 3 + panel 4 → 9 fixed
  `ec4a3fe`, 1 deferred; records `8d62afc`). **REV4 code-complete.**
  fmt/clippy clean; tests 1405/0/3 on Windows. (Stack pushed to both
  remotes 2026-07-04, after this entry was written.)
- **2026-07-04 (7th)** @ `f6f52d7`+docs — `ue-r2-1h` landed end-to-end
  (`2a13f53` deletion+port; codex NEEDS FIXES 3 + panel → 5 fixed
  `f6f52d7`, 1 deferred relay-1, 1 rejected; plus Windows-host
  pre-existing fixes `9f37a7a` clippy baseline + `48c5a11` win-1
  push-separator). fmt/clippy clean; tests 1393/0/3 on Windows.
