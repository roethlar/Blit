# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w4-5` landed and graded through the codex
loop — **CancelJob now works on attached Push/PullSync transfers**,
D-2026-07-04-3 executed); local HEAD `1708075`+records, **not yet
pushed** to either remote across these sessions.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **w4-5 DONE — `supports_cancellation` flipped for Push/PullSync**
  (`05a8b39`+fix `1708075`; finding
  `.review/findings/w4-5-supports-cancellation-flip.md`). One-predicate
  flip in `active_jobs.rs` (only history-only `Pull` stays gated — its
  RPC died at ue-r2-1h): `blit jobs cancel` and the TUI `K`/`Shift+X`
  now fire the row token for attached transfers; the w4-3 race tears
  down and sends the still-connected client `Status::cancelled`. CLI
  contract for those kinds: exit 2/FailedPrecondition → exit 0
  (mapping itself unchanged; Unsupported arm survives end-to-end as
  escape hatch). Workflow-swept every old-policy comment surface
  (active_jobs, core, blit-app outcome doc, proto wire-contract
  comment, jobs_lifecycle header); TUI/CLI needed zero logic changes
  (no kind gating exists). Tests: policy pin flipped, dispatch + RPC
  success pinned per kind, authz covers flipped kinds — the four new
  pins mutation-verified (revert → exactly those 4 fail). Codex:
  NEEDS FIXES 1 Low (module scope-log doc drift → `1708075`).
  blit-daemon 168 → 170; workspace 1448/0/2 across 37 suites.
- **Earlier 2026-07-04: W1 family + w4-1 + w4-3 all `[x]`** (details:
  DEVLOG 2026-07-04 entries; findings `.review/findings/w1-*.md`,
  `w4-*.md`): shared data-socket policy helper w1-2 `16237e2`, real
  keepalive timing w1-3 `865fc1e`, shared accept/token bounds w1-4
  `6a19e1d`+`d17b089`; AbortOnDrop family w4-1; daemon disconnect
  racing w4-3 `37d7f91`.
- **Process (D-2026-07-04-1)**: the codex loop now governs **all code
  and plan changes** — no exceptions; codex is the **only** reviewer
  (no same-model panels; owner correction this session). Async
  sentinel loop retired. Decision + propagation `3ebcc37`, its own
  codex round fixed `10866e4`.
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
  backslash echo). Erratum **settled (D-2026-07-04-2)**: those two
  commits don't build in isolation (staging slip); they stay as
  pushed, no history rewrite — bisect must skip them; HEAD fully
  gated.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5), code-complete; flipping to Shipped is an
    owner call after the 10 GbE benchmark session.
  - Process: the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`) now
    governs **all code and plan changes** (D-2026-07-04-1, owner: "no
    exceptions"); the `.review/README.md` async sentinel loop is
    retired. REVIEW.md stays the queue/status index.

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. w4-5 closed
   `[x]` 2026-07-04 (see Now), following w4-1, w4-3, and the W1
   family the same day. Strict row order now gives **w2-2**
   (stream-ladder owner) as the topmost open row; **design-3**
   (data-plane connect timeouts, filed-findings section) is trivially
   placeable after W1 (two client connect sites, bound imports the
   shared `DATA_PLANE_ACCEPT_TIMEOUT`) — w2-2 vs design-3 sequencing
   stays the coder's pick unless the owner orders otherwise. Open Low
   rows: `relay-1-subpath-double-join`.
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

- **(RESOLVED 2026-07-04, owner Q&A session)** Four standing items
  answered one-at-a-time: commit erratum → **leave as-is**
  (D-2026-07-04-2); 10 GbE → **soon, keep coding first** (see
  Blocked); D-2026-06-20-1 stale wording → **follow the existing
  pattern** (edited in place, D-2026-06-20-3/-6 style);
  `supports_cancellation` → **flip it** (D-2026-07-04-3 — **executed**,
  landed as w4-5 `05a8b39`+`1708075`).
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(OPEN)** REV4 → Shipped flip: after the 10 GbE session, or now
  with the measurement gates tracked separately? Owner call (10 GbE
  now "soon" — likely resolves with it).
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed); the
  daemon-spawn e2e family shows load-flakiness under full-parallel
  runs (w9-3 territory). windows-latest CI on the next push should be
  meaningfully green.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-04 (12th)** @ `1708075`+records+docs —
  **w4-5-supports-cancellation-flip landed and graded through the
  codex loop** (owner go: "continue" → topmost open row per the 11th
  handoff). D-2026-07-04-3 executed: CancelJob dispatch flipped on for
  attached Push/PullSync (one predicate; Pull history-only stays
  gated), every old-policy comment surface updated including the
  proto/blit.proto wire-contract comment (a 3-agent workflow sweep
  enumerated the surfaces; TUI/CLI have no kind gating — zero logic
  changes there). Codex: NEEDS FIXES 1 Low (module scope-log rustdoc
  still claimed Pull wired at dispatch → fixed `1708075`); verdict +
  trimmed review recorded, REVIEW.md row `[x]`. Four new contract pins
  mutation-verified (revert flips exactly those 4 red). blit-daemon
  168 → 170; workspace 1448/0/2 across 37 suites, fmt/clippy clean
  (macOS host). Known gap (finding doc): no e2e drives a live
  mid-flight attached cancel — deterministic parking needs a test
  seam; same evidence shape w4-3 was graded on. In-flight: none.
  **Exact first action next session**: on owner "continue", pick up
  **w2-2** (stream-ladder owner, topmost open row by strict order;
  design-3 remains the sanctioned smaller alternative) through the
  codex loop. Nothing pushed — push stays owner-gated.
- **2026-07-04 (11th)** @ `d17b089`+records+docs — **w1-2, w1-3, w1-4
  landed and graded through the codex loop in one session** (owner go:
  "continue. use /playbook reviewloop with codex for each commit" —
  the named playbook doesn't exist; mapped to standing policy
  D-2026-07-04-1 / `docs/agent/GPT_REVIEW_LOOP.md`). Codex verdicts:
  PASS 0 / PASS 0 / NEEDS FIXES 1 Low (stall_guard doc drift → fixed
  `d17b089`). W1 transport-policy family complete: shared socket
  policy helper (pull direction de-Nagled, dial buffers wired, daemon
  twin deleted), real keepalive timing, shared accept/token bounds.
  blit-core 414 → 418; workspace 1446/0/2 across 37 suites (macOS
  host; Windows coverage rides the next push's CI). Environment note:
  codex hangs if invoked in a chained command — run it standalone
  with stdin closed (`< /dev/null`). Same session, owner Q&A: all
  four standing questions answered one-at-a-time — erratum leave
  (D-2026-07-04-2), 10 GbE soon-keep-coding, D-2026-06-20-1 edited
  in place per existing pattern, cancellation flip authorized
  (D-2026-07-04-3 → new row w4-5). In-flight: none. **Exact first
  action next session**: on owner "continue", pick up **w4-5**
  (cancel-policy flip, topmost open row) through the codex loop.
  Nothing pushed — push stays owner-gated.
- **2026-07-04 (10th)** @ `37d7f91`+records+docs —
  `w4-3-daemon-disconnect-racing` landed and graded through the codex
  loop: **PASS, zero findings**, no fix commit; row `[x]`, verdict +
  trimmed review recorded. Select arms mutation-verified. blit-daemon
  162 → 167 (Windows-host count; macOS baseline measures 168); all 37
  suites green (gates run in PowerShell — Git Bash can't link on that
  host, coreutils `link` shadows MSVC). New open question: flip
  `supports_cancellation` for Push/PullSync (now policy-only).
