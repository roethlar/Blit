# STATE — single entry point for "what is true right now"

Last updated: 2026-07-04 (**SMALL_FILE_CEILING flipped Active**,
D-2026-07-04-4 — sf-1 is the active slice. Prior session recorded the
owner principle: perf goals are ceiling-driven, never
competitor-relative — and drafted the plan. Same
session: 10 GbE benchmark ran end-to-end (wire-ceiling push/pull,
ue-1 band holds), blit/rsync/rclone comparison measured (21/24 wire
cells won; small-file/mixed push are the ceiling gaps the plan
closes), zero-copy revisit-gate CPU data collected, w9-3 landed).
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **SMALL_FILE_CEILING Active (D-2026-07-04-4) — sf-1 in progress** —
  owner correction governing all perf work: FAST/SIMPLE/RELIABLE
  gate every change; goals are **ceiling-driven, never
  competitor-relative** (a "beat X by N%" bar embeds a stopping
  condition; a ≥25% margin answer was explicitly retracted — do not
  re-litigate). Plan `docs/plan/SMALL_FILE_CEILING.md` (**Active**,
  `78eabfd`+`811a3f2`, codex 5/5 accepted+fixed, records `219cecf`):
  small-file/mixed cells to a NAMED hardware limiter, tools as
  tripwires only; evidence durable at `docs/bench/10gbe-2026-07-05/`
  (DIAGNOSIS.md: one-stream-for-10k-files dial gap, 215 µs/file
  daemon cost vs 34 ms wire, CPU gate data). Next slice: **sf-1
  tripwire harness** (no production code); sf-6 keeps its own wire
  owner gate. skippy torn down (daemons stopped, payloads
  removed; binaries staged at `blit-bin/` for sf-4).
- **Tool comparison measured (2026-07-05)** — blit vs rsyncd /
  rsync-ssh / rclone (sftp, webdav, no-hash fairness cells): blit
  fastest on all large/pull/local cells at the wire ceiling; rsyncd
  faster on small push (1.5 s vs 2.4–3.3 s), small pull (0.37 vs
  0.45 s), mixed push — exactly the plan's target cells. rclone has
  no LAN config that competes (webdav smalls catastrophic: 315 s).
  CSVs tracked in `docs/bench/10gbe-2026-07-05/`.
- **10 GbE benchmark session DONE (2026-07-04/05)** — the REV4
  sign-off data is in; owner declarations pending (see Blocked).
  Headlines (digest: DEVLOG 2026-07-05 00:34; durable evidence:
  `docs/bench/10gbe-2026-07-05/`): push/pull 1 GiB ≈ 9.5 Gbit/s
  against a 9.88 iperf3 ceiling @ MTU 9000, first payload 14.5 ms;
  **ue-1 loopback parity band holds** (worst spread 1.8×); reverse
  direction validated; no organic resize anywhere (one stream
  saturates 10 GbE) — ue-2 is an interpretation call; zero-copy
  0 bytes at wire saturation. Bench script repaired through the
  codex loop en route (`b9befb8`+`92d6326`, 2 High accepted;
  methodology + disk-path follow-ups recorded in DEVLOG).
- **w9-3 DONE — test-harness consolidation** (`f6e592e`+`8641bc6`+
  `c62d15b`; finding `.review/findings/w9-3-test-harness-builder.md`;
  DEVLOG 2026-07-04 23:35). One harness (builder, second-daemon,
  OnceLock build, keepalive parity via
  `blit_core::remote::grpc_server`); daemon-spawn port-TOCTOU flake
  root-caused + fixed. Codex 1 Medium accepted → fixed. Tests
  1478 → 1479/0/2 same-method A/B.
- **Earlier 2026-07-04: design-3, w4-4, w6-2 (filed w6-2a/b/c), w6-1
  (+design-1), w3-1, w2-2, w4-5, W1 family, w4-1, w4-3 all `[x]`** —
  DEVLOG 2026-07-04 entries; `.review/`; commit map in REVIEW.md.
- **REV4 code-complete**; measurement gates DATA-COMPLETE — only the
  owner declarations remain. Residue: Queue item 4. Windows: suite
  green on the owner's machine (erratum D-2026-07-04-2 settled).
- **Active context**: REV4 plan Active (D-2026-06-20-5); codex loop
  governs all code + plan changes (D-2026-07-04-1); REVIEW.md is the
  queue/status index.

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
2. **10 GbE session ran (see Now) — owner declarations pending**:
   ue-1 (evidence: band holds), ue-2 (no organic resize at 10 GbE —
   interpretation call), zero-copy revisit verdict (D-2026-06-12-1;
   evidence: wire-saturated with 0 spliced bytes), REV4 → Shipped.
   Optional measurement follow-ups (owner-gated): Win 11 bare-metal
   datapoint on the dual-boot client (same hardware window,
   deployment parity, not a gate); disk-path variants (post-push
   `zpool sync` column, cold-ARC pulls via `primarycache`);
   sustained >ARC-size push for the pool floor. Env note: bench area
   is now `skippy:/mnt/generic-pool/video/blit-bin/` (binaries +
   bench.toml staged; /tmp and /home on skippy are noexec). After
   the declarations: audit Round 1, TUI rework, H10b planner.
3. **`docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4 —
   sf-1 is the current slice; see Now)**: close the measured
   small-file/mixed gaps to the hardware ceiling. Owner principle
   recorded in the doc
   (2026-07-05): goals are ceiling-driven, never competitor-relative
   — tools like rsync are tripwires, not targets. Slices sf-1..7;
   sf-6 (wire-visible tar-shard lane) carries its own owner gate.
4. **Post-REV4 residue** (unowned until the owner slots them): pull
   1s-start restructuring; epoch-0/early-ADD hardening; remote
   perf-history lanes (1e gap); `derive_local_plan_tuning`
   fold-or-retire (statically live on the local engine path but
   dynamically dead — nothing fills the tar/raw telemetry buckets
   since `4ce4898`, 2026-04-07; verified during the w2-2 audit,
   design decision not review-queue material); receive-side dial
   tuning (rest of constants-receive-chunk-1mib-asymmetry — w3-1
   scoped it out, wire needs no change; separate slice if wanted).

## Authoritative docs right now

- **Active plans: `docs/plan/SMALL_FILE_CEILING.md`**
  (D-2026-07-04-4; sf-1 current) and
  **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
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

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Four 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE —
  sf-5 would give it a real trigger), zero-copy revisit verdict,
  REV4 → Shipped.
- **Zero-copy option a/b/c** (from the 2026-07-05 exchange): (a) keep
  deletion + append measured CPU data and regeneralize the rig-bound
  revisit gate in the eval doc, (b) amend D-2026-06-12-1 to keep the
  module, (c) leave as-is (data stays in DEVLOG +
  docs/bench/10gbe-2026-07-05/DIAGNOSIS.md). Measured: 1.43 cores
  daemon-receive / 0.45 client at 9.5 Gbit/s — gate not met on this
  rig, but "fraction of one core" was optimistic.
- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
  approval flow; windows-latest CI on the w9-3 harness fix rides it.
- `Cargo.lock`: dependency-refresh drift committed at `04c9c6d` (was
  unavoidable — blit-core gained `rand`); revert selectively if
  unwanted, otherwise settled.

## Open questions

- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
- **(OPEN, new 2026-07-04)** `725aa07` tracked a 236-file stale
  worktree snapshot (`.claude/worktrees/vigilant-mayer/`, incl. a
  full `crates/` copy). Keep or `git rm -r`? Agent rec: remove;
  deletion awaits an owner go.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
  session delivered the measurement evidence; flip awaits the four
  declarations in Blocked.
- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
  `blit copy src_large dst` with an existing local dir, no `./`,
  parses the bare name as an mDNS discovery endpoint and errors
  "remote source must include a module or root"
  (blit-app endpoints.rs). Should local-path existence win over the
  discovery interpretation, or at least improve the error? Candidate
  review-queue row; owner to slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed). The
  daemon-spawn e2e load-flakiness is now root-caused and fixed on
  Linux (w9-3: port-TOCTOU wrong-daemon race + cargo-lock contention;
  claimed-port set + OnceLock build + child-death check). Remaining
  check: windows-latest CI on the next push (10d89e0 predates the
  w9-3 fix, so daemon-spawn flakes there would not be news).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-05 (22nd)** @ `219cecf`+handoff — **ceiling principle
  recorded + SMALL_FILE_CEILING drafted through the plan procedure**
  (owner interview → correction → reframe; codex plan review 5/5
  accepted+fixed; evidence bundle committed to
  `docs/bench/10gbe-2026-07-05/`). Tool comparison + zero-copy CPU
  data measured and recorded same session. skippy fully torn down;
  tree clean. In-flight: none. **Exact first action next session**:
  the owner declarations in Blocked — above all the
  SMALL_FILE_CEILING Active flip (then sf-1); coding queue's w7-1
  remains the fallback if the owner defers everything.
- **2026-07-05 (21st)** @ `92d6326`+records — **10 GbE benchmark
  session ran end-to-end** (owner-called and owner-attended: MTU
  9000 set on the client mid-session, ufw confirmed, bench area
  designated). All REV4 measurement evidence banked — wire-ceiling
  push/pull, ue-1 band holds, both directions, resize/zero-copy
  datapoints (Now + DEVLOG 2026-07-05). Bench script repaired
  through the codex loop en route (`b9befb8`+`92d6326`, 2 High
  accepted+fixed). In-flight: none. **Exact first action next
  session**: the four owner declarations in Blocked (ue-1/ue-2/
  zero-copy/REV4→Shipped); coding queue resumes at **w7-1** after
  that (or immediately if the owner defers the declarations).
  Nothing pushed — push stays owner-gated.
