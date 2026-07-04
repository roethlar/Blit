# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (`w1-2` + `w1-3` + `w1-4` landed and graded
through the codex loop — **the W1 transport-policy family is
complete**); local HEAD past `6a38810`, **not yet pushed** to either
remote across these sessions.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **W1 family DONE — w1-2, w1-3, w1-4 all landed and graded `[x]`**
  (details: DEVLOG 2026-07-04 T12:54Z/T13:06Z/T13:36Z; findings under
  `.review/findings/w1-*.md`). w1-2 (`16237e2`): one shared
  `configure_data_socket(&TcpStream, Option<usize>)` in
  `blit_core::remote::transfer::socket` (SockRef in-place; nodelay
  hard, keepalive/buffers logged); every data-plane socket routes
  through it — push client connect, pull client connect (the pull
  Nagle fix), daemon push accepts (silently-swallowing twin + daemon
  socket2 dep deleted), pull_sync accepts (dial's `tcp_buffer_bytes`
  now applied — snapshot at epoch-0/resume, LIVE at the resize
  accept). w1-3 (`865fc1e`): real `TcpKeepalive` timing at that single
  site — 60 s/10 s/5, dead idle peer detected in ~2 min not ~2 h;
  socket2 → `features=["all"]`. w1-4 (`6a19e1d`+fix `d17b089`): shared
  `DATA_PLANE_ACCEPT_TIMEOUT`/`DATA_PLANE_TOKEN_TIMEOUT` pair in the
  same module, three local declarations deleted. Codex: PASS/PASS/
  NEEDS-FIXES-1-Low(doc drift, fixed). All mutation-verified where
  tests shipped; fmt/clippy clean; workspace 1446/0/2 (blit-core 414 →
  418). Earlier same day: w4-3 + w4-1 closed `[x]`.
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
  backslash echo). Erratum: `9f37a7a`/`48c5a11` don't build in
  isolation (a staging slip put the pull.rs deletion in the wrong
  commit; bisect must skip them; HEAD fully gated) — owner may
  authorize a history fix of the now-pushed stack or leave it.
- **Active context** (settled background):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the Active
    plan (D-2026-06-20-5), code-complete; flipping to Shipped is an
    owner call after the 10 GbE benchmark session.
  - Process: the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`) now
    governs **all code and plan changes** (D-2026-07-04-1, owner: "no
    exceptions"); the `.review/README.md` async sentinel loop is
    retired. REVIEW.md stays the queue/status index.

## Queue (ordered)

1. **Design-review queue** — `REVIEW.md` order governs. w4-1, w4-3,
   and the whole W1 family (w1-2/w1-3/w1-4) closed `[x]` 2026-07-04
   (see Now). Next in REVIEW.md row order: **w2-2** (stream-ladder
   owner) — but **design-3** (data-plane connect timeouts, filed
   Medium, `.review/findings/design-3-unbounded-data-plane-connects.md`)
   is now trivially placeable after W1: two client connect sites
   (`connect_with_probe`, `connect_pull_stream`), bound can import the
   shared `DATA_PLANE_ACCEPT_TIMEOUT`. Sequencing is the coder's pick
   unless the owner orders otherwise. Open Low rows:
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
- **(OPEN, new w4-3)** Flip `supports_cancellation` for Push/PullSync
  so `CancelJob` (and the TUI F2 cancel) works on attached transfers?
  The handlers now race the row token, so the flip is policy-only —
  but it changes the CancelJob contract (exit-code 2 → 0, TUI
  Unsupported surfaces). Agent rec: flip in a small follow-up slice;
  the "disconnect is the cancel" rationale no longer requires the gate.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed); the
  daemon-spawn e2e family shows load-flakiness under full-parallel
  runs (w9-3 territory). windows-latest CI on the next push should be
  meaningfully green.

## Handoff log (newest first, keep ≤ 3)

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
  with stdin closed (`< /dev/null`). In-flight: none. **Exact first
  action next session**: on owner "continue", pick up **design-3**
  (data-plane connect timeouts — now two consolidated call sites +
  the shared bound; or w2-2 if the owner prefers strict row order)
  through the codex loop; else the owner schedules the 10 GbE
  sign-off / decides the erratum + D-2026-06-20-1 +
  supports_cancellation questions. Nothing pushed — push stays
  owner-gated.
- **2026-07-04 (10th)** @ `37d7f91`+records+docs —
  `w4-3-daemon-disconnect-racing` landed and graded through the codex
  loop: **PASS, zero findings**, no fix commit; row `[x]`, verdict +
  trimmed review recorded. Select arms mutation-verified. blit-daemon
  162 → 167 (Windows-host count; macOS baseline measures 168); all 37
  suites green (gates run in PowerShell — Git Bash can't link on that
  host, coreutils `link` shadows MSVC). New open question: flip
  `supports_cancellation` for Push/PullSync (now policy-only).
- **2026-07-04 (9th)** @ `6a38810`+docs — `w4-1-abortondrop-family`
  landed (`65ecb93`+`44bf416`) **and graded through the codex loop**:
  NEEDS FIXES 1 Low (vacuous relocated drop-test) → fixed `bedfa52`,
  mutation-verified; w4-1 + design-2 `[x]`, sentinel deleted
  (`6a38810`). Same session: **D-2026-07-04-1** — codex loop for ALL
  code and plan changes, codex the only reviewer (no same-model
  panels); decision+propagation `3ebcc37`, its codex round fixed
  `10866e4`. fmt/clippy clean; workspace tests green.
