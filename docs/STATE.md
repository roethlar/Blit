# STATE — single entry point for "what is true right now"

Last updated: 2026-07-05 (**otp-3 TransferSession core landed +
graded** — the unified session moves real bytes in-process and the
role suite pins the owner's invariance property; ONE_TRANSFER_PATH
otp-1 + otp-3 `[x]`, current slice otp-4. SMALL_FILE_CEILING stays
paused, D-2026-07-05-1.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 flip: "flip the plan and go") — otp-3 landed**
  — owner directive 2026-07-05, verbatim in the plan doc: ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing, impossible by construction
  because the per-direction drivers and the `Push`/`PullSync` RPCs
  are deleted. One `TransferSession` (roles SOURCE/DESTINATION), one
  `Transfer` RPC, one choreography (streaming source manifest,
  destination diffs, sf-2 shape-corrected dial as the only stream
  policy); gRPC fallback becomes a byte-carrier option; delegated =
  daemon-initiated session; local rides an in-process transport.
  Slices otp-1..13; converge-up constraint (unified path must match
  the better direction per cell ±10%); benchmark verdict cells must
  be symmetric-fs disk-to-disk, tmpfs = wire-reference rows only.
  **D-2026-07-05-2: no version compatibility, EVER — same-build
  peers only, refusal at session open; REV4's negotiate-down clause
  void.** **otp-1 `[x]`** (`a3e2acb`+`f861579`; contract:
  `docs/TRANSFER_SESSION.md`). **otp-3 `[x]`** (`ef9ffa1` + review
  fix `d5796a1`, codex 2/2 accepted+fixed: non-collapsing build-id
  forms + early-NeedComplete gate; suite 1484 → **1501/0**; the role
  suite runs every fixture under both initiator layouts and pins
  identical need sets/summaries/byte-identical trees — the owner's
  invariance property is now executable). Current slice: **otp-4
  daemon serves `Transfer`, client initiates as SOURCE** (otp-2
  symmetric baseline is rig-gated; must land before otp-10).
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1 `[x]`
  sf-2 `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`, codex 1/1,
  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Its principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence at
  `docs/bench/10gbe-2026-07-05/`; binaries staged at `blit-bin/`.
- **10 GbE benchmark session DONE (2026-07-04/05)** — REV4 sign-off
  data in (push/pull ≈ 9.5 of 9.88 Gbit/s; **ue-1 band holds**; no
  organic resize → ue-2 call); tool comparison: blit fastest on
  large/pull/local cells, rsyncd faster on small/mixed push (the
  paused plan's cells). Owner declarations pending (Blocked).
  Evidence `docs/bench/10gbe-2026-07-05/`; DEVLOG 00:34 + 00:51.
- **Earlier 2026-07-04: w9-3 + eleven review-queue rows all `[x]`**
  — DEVLOG 2026-07-04 entries; commit map in REVIEW.md.
- **REV4 code-complete**; measurement gates DATA-COMPLETE — only the
  owner declarations remain. Residue: Queue item 4. Windows: suite
  green on the owner's machine (erratum D-2026-07-04-2 settled).
- **Active context**: REV4 plan Active (D-2026-06-20-5); codex loop
  governs all code + plan changes (D-2026-07-04-1); REVIEW.md is the
  queue/status index.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3 `[x]`.
   Current: otp-4 (daemon serves `Transfer`, client initiates as
   SOURCE; A/B parity pins vs old push — the SizeMtime semantic
   divergence recorded in the otp-3 finding doc gets decided here).
   otp-2 (symmetric baseline) is RIG-GATED — runs when the 10 GbE
   rig is available, must land before otp-10 cutover.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2,
   REV4 → Shipped (zero-copy resolved — D-2026-07-05-3). Optional
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
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: revisit gate
   declared met (UNAS 8 Pro daemon CPU-bound below 10 GbE from SSD
   cache). Executes AFTER ONE_TRANSFER_PATH cutover as a
   runtime-selected write strategy in the unified receive sink
   (design input: eval doc §If-FAST-evidence; dead module still
   deletes in w8-1). UNAS is the measurement rig; symmetric-endpoint
   methodology applies. **Rig `zoey` (verified 2026-07-05)**: UNAS 8
   Pro, 4×Cortex-A57 aarch64, Debian 11 userland (glibc 2.31), kernel
   5.10, 15 GiB; test dir `root@zoey:/volume/a595ddbf-…/.srv/
   .unifi-drive/michael/.data/blit-temp/`. **Build recipe** (static
   musl — sidesteps the old glibc): rustup target
   `aarch64-unknown-linux-musl` + `aarch64-linux-gnu-gcc` as
   LINKER/CC/AR for that target, `RUSTFLAGS="-C
   target-feature=+crt-static -C link-self-contained=yes"`, `cargo
   build --release --target aarch64-unknown-linux-musl -p blit-daemon
   -p blit-cli`. Binaries verified executing on zoey 2026-07-05.
   **Owner constraints (2026-07-05, standing)**: ALL activity on
   zoey is restricted to that blit-temp folder — test daemon module
   roots there, test data there, nothing written outside it, ever.
   Zero-copy is to be TESTED on this rig when the post-cutover slice
   set reaches it (standing owner authorization for that test, within
   the folder restriction); no daemon runs on zoey before then
   without a fresh go.
6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**.
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
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
  recorded in DEVLOG + DIAGNOSIS.md.)
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
  session delivered the measurement evidence; flip awaits the three
  declarations in Blocked (was four — zero-copy resolved,
  D-2026-07-05-3).
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

- **2026-07-05 (26th)** @ `85bf611` — **otp-3 landed and graded**:
  `blit-core/src/transfer_session/` (details: Now bullet 1, DEVLOG
  18:30, finding doc). Codex FAIL 2/2 accepted+fixed (`ef9ffa1`, fix
  `d5796a1`): non-collapsing build-id forms + early-NeedComplete
  gate (guard proven by revert). Suite **1501/0**. In-flight: none.
  **Exact first action next session**: otp-4 (daemon serves
  `Transfer`, client initiates as SOURCE; A/B parity pins vs old
  push — settle the SizeMtime divergence flagged in the otp-3
  finding doc) through the codex loop. otp-2 stays rig-gated (before
  otp-10). Owner declarations: three 10 GbE gates + push go remain
  in Blocked.
- **2026-07-05 (25th)** @ `cb96e91`+records — plan Active
  (D-2026-07-05-4) + otp-1 landed/graded (`a3e2acb` → `f861579`,
  contract `docs/TRANSFER_SESSION.md`); D-2026-07-05-2 (same-build
  only); D-2026-07-05-3 (zero-copy unparked; zoey rig + musl recipe:
  queue item 5). Details: DEVLOG 2026-07-05 10:00.
- (older entries pruned — see DEVLOG 2026-07-05 06:45 and earlier)
