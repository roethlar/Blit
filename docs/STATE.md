# STATE — single entry point for "what is true right now"

Last updated: 2026-07-05 (**owner directive D-2026-07-05-1: ONE
transfer path, direction-invariant by construction** — plan
`docs/plan/ONE_TRANSFER_PATH.md` drafted, in codex review, awaiting
the owner's Active flip. **All SMALL_FILE_CEILING work is paused**
(sf-2 landed + graded earlier this date; sf-3a+ blocked). Earlier:
sf-1/sf-2 landed, 10 GbE benchmark session complete, w9-3 landed.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH (D-2026-07-05-1) — Draft, codex review, then
  owner Active flip** — owner directive 2026-07-05, verbatim in the
  plan doc: ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing, impossible by construction
  because the per-direction drivers and the `Push`/`PullSync` RPCs
  are deleted. One `TransferSession` (roles SOURCE/DESTINATION), one
  `Transfer` RPC, one choreography (streaming source manifest,
  destination diffs, sf-2 shape-corrected dial as the only stream
  policy); gRPC fallback becomes a byte-carrier option; delegated =
  daemon-initiated session; local rides an in-process transport.
  Slices otp-1..13; converge-up constraint (unified path must match
  the better direction per cell ±10%); benchmark verdict cells must
  be symmetric-fs disk-to-disk (owner: "tmp on one side, spinning
  rust on the other is not a valid test"), tmpfs = wire-reference
  rows only. **No code until the owner flips Active.**
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1 `[x]`
  sf-2 `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`, codex 1/1,
  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Its principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence at
  `docs/bench/10gbe-2026-07-05/`; binaries staged at `blit-bin/`.
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
- **Earlier 2026-07-04: w9-3 test-harness consolidation (port-TOCTOU
  flake root-caused; tests 1478 → 1479), design-3, w4-4, w6-2 (filed
  w6-2a/b/c), w6-1 (+design-1), w3-1, w2-2, w4-5, W1 family, w4-1,
  w4-3 all `[x]`** — DEVLOG 2026-07-04 entries; `.review/`; commit
  map in REVIEW.md.
- **REV4 code-complete**; measurement gates DATA-COMPLETE — only the
  owner declarations remain. Residue: Queue item 4. Windows: suite
  green on the owner's machine (erratum D-2026-07-04-2 settled).
- **Active context**: REV4 plan Active (D-2026-06-20-5); codex loop
  governs all code + plan changes (D-2026-07-04-1); REVIEW.md is the
  queue/status index.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` — the only work item until it
   ships (owner directive: "do not do ANYTHING else")**: Draft
   written 2026-07-05, codex plan review + adjudication, then STOP
   for the owner's Active flip. After the flip: slices otp-1..13
   through the codex loop, starting with otp-1 (wire+session
   contract, doc+proto, no behavior).
2. **10 GbE owner declarations (unchanged, still pending)**: ue-1,
   ue-2, zero-copy a/b/c (D-2026-06-12-1), REV4 → Shipped. Optional
   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
   disk-path variants; >ARC-size push) — note the disk-path items
   are largely absorbed by otp-2/otp-12's symmetric-rig matrices. Env: bench
   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
   (/tmp and /home on skippy are noexec).
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
   (mirror-executor consolidation) likely lands for free inside
   otp-6's one-delete-rule slice; re-check before picking it up.
5. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (Draft — governs all work; no
  code until Active, D-2026-07-05-1)**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2, D-2026-07-05-1) and
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

- **ONE_TRANSFER_PATH Draft → Active flip** (owner; after the codex
  plan review is adjudicated). Until then no implementation anywhere
  — the directive blocks all other work too.
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

- **2026-07-05 (25th)** @ ONE_TRANSFER_PATH records — **owner
  directive D-2026-07-05-1** (one transfer path,
  direction-invariance by construction; verbatim quotes in the plan
  doc) after the owner rejected the push/pull disparity and the
  mixed-fs benchmark methodology. Plan drafted through the plan
  procedure; SMALL_FILE_CEILING + design queue paused. In-flight:
  codex plan review adjudication. **Exact first action next
  session**: finish the plan-review adjudication if incomplete, then
  STOP for the owner's Active flip — no implementation anywhere
  until it lands (then otp-1: wire+session contract, doc+proto).
- **2026-07-05 (24th)** @ `7627e7b`+records — **sf-2 landed and
  graded** (shape-correction stream resize `c70c2ac`, codex 1/1
  accepted → `7627e7b`; e2e guard proven by revert; suite
  1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none.
  (Its "next: sf-3a" is superseded by the 25th entry above.)
- (older entries pruned — see DEVLOG 2026-07-05 03:03 and earlier)
