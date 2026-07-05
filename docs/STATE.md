# STATE — single entry point for "what is true right now"

Last updated: 2026-07-05 (**the 10 GbE benchmark session ran** —
owner-called, TrueNAS↔Arch @ MTU 9000, iperf3 ceiling 9.88/9.91
Gbit/s: blit TCP push/pull 1 GiB both ≈ 9.5 Gbit/s at the wire
ceiling, ue-1 loopback parity band HOLDS (worst spread 1.8×), both
directions validated, zero-copy datapoint = saturation with 0 spliced
bytes. Full digest: DEVLOG 2026-07-05 entry + logs/bench_10gbe_*.
Same day: w9-3 harness consolidation landed and graded).
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check still queued behind
the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **10 GbE benchmark session DONE (2026-07-04/05)** — the REV4
  sign-off data is in; **four owner declarations now pending** (see
  Blocked). Headlines (full: DEVLOG 2026-07-05; evidence:
  `logs/bench_10gbe_20260704T20*/`): TCP push/pull 1 GiB ≈ 9.5
  Gbit/s against a 9.88 iperf3 ceiling, first payload 14.5 ms;
  **ue-1 loopback parity band holds** (local/push/pull worst spread
  1.8×, no 10×/2× gap); reverse direction validated (7.25/9.75
  Gbit/s); gRPC fallback wire-competitive on large (8.0 Gbit/s);
  no organic mid-transfer resize anywhere — one stream saturates
  10 GbE (clean idle-stream teardown, no wedges, incl. under
  2-concurrent-push contention) — ue-2 is therefore an owner
  interpretation call (deterministic resize coverage = ue-r2-2
  suite); zero-copy: `zero-copy 0 bytes` on every transfer AT wire
  saturation (splice buys nothing at 10 GbE). Bench script repaired
  en route through the codex loop (`b9befb8` grammar/flag +
  `92d6326` matrix validity, 2 High accepted;
  `.review/results/bench-script-fix.*`). Methodology: engine-vs-wire
  isolation (tmpfs ends, async ZFS writes, ARC-warm re-reads, no
  sync between runs — deliberate, recorded); disk-path variants
  (post-push `zpool sync` timing, cold-ARC pulls) are owner-gated
  follow-ups. Pool cleaned; binaries + config staged at
  `skippy:/mnt/generic-pool/video/blit-bin/` for future sessions.
- **w9-3 DONE — test-harness consolidation** (`f6e592e` + review fix
  `8641bc6` + records `c62d15b`; finding
  `.review/findings/w9-3-test-harness-builder.md`; full story: DEVLOG
  2026-07-04 23:35 entry). One daemon-spawn harness in `tests/common`
  (builder + spawn_second_daemon absorbed SEVEN clones + the pasted
  cli_bin/run_with_timeout/ChildGuard copies); OnceLock'd daemon
  build (R16-F1 kept); `blit_core::remote::grpc_server` = single
  keepalive owner for daemon + all five fake servers. Daemon-spawn
  load-flakiness root-caused (port TOCTOU) and fixed two-layer.
  Codex: 1 Medium accepted (fake-server bind bypassed the claimed
  set) → fixed. Tests 1478 → 1479/0/2 by same-method A/B.
- **Earlier 2026-07-04 (four sessions): design-3, w4-4, w6-2 (filed
  w6-2a/b/c), w6-1 (+design-1), w3-1, w2-2, w4-5, W1 family, w4-1,
  w4-3 all `[x]`** — details in DEVLOG 2026-07-04 entries; findings
  + verdicts in `.review/`; commit map in REVIEW.md.
- **REV4 code-complete** (all nine `ue-r2-*` slices; stream resize
  live; all three static ladders retired). The measurement gates are
  now DATA-COMPLETE (see the 10 GbE item above) — only the owner
  declarations remain. Residue: Queue item 3.
- **Windows-host sessions (2026-07-04)**: suite fully green on the
  owner's Windows machine. Erratum settled (D-2026-07-04-2):
  `9f37a7a`/`48c5a11` don't build in isolation; bisect skips them.
- **Active context**: REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4
  .md`) Active (D-2026-06-20-5); the codex loop governs all code and
  plan changes (D-2026-07-04-1); `.review/README.md` async loop
  retired; REVIEW.md stays the queue/status index.

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
3. **`docs/plan/SMALL_FILE_CEILING.md` (Draft — awaiting owner
   Active flip)**: close the measured small-file/mixed gaps to the
   hardware ceiling. Owner principle recorded in the doc
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

## Blocked / waiting

- **Four owner declarations from the completed 10 GbE session**
  (checkpoints are owner-only): ue-1 pass/fail, ue-2 pass/fail or
  re-scope, zero-copy revisit verdict, REV4 → Shipped flip. Evidence
  is in (Now + DEVLOG 2026-07-05); agent reads it as: band holds,
  wire saturated, resize unexercisable at this wire speed, splice
  unnecessary at 10 GbE.
- **Push go** (always owner-gated): local commits `f6e592e`..HEAD
  await the ref-listing + approval flow; windows-latest CI on the
  w9-3 harness fix rides on it.
- `Cargo.lock`: dependency-refresh drift committed at `04c9c6d` (was
  unavoidable — blit-core gained `rand`); revert selectively if
  unwanted, otherwise settled.

## Open questions

- **(OPEN)** Historical audit/finding docs still embed `/Users/...`
  in recorded evidence — scrub or leave? Agent rec: leave.
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
- **2026-07-04 (19th)** @ `c609192`+docs — push recorded (`10d89e0`
  → GitHub) + 10 GbE host plan settled (TrueNAS ↔ Arch; details in
  DEVLOG 2026-07-04 22:09). Executed by the 20th/21st entries.
