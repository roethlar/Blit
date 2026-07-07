Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f3031-c48f-7e50-a6e0-c90771d7e2c3
--------
user
Review the diff of commit 7202c1a (run: git show 7202c1a). It implements sf-1 of docs/plan/SMALL_FILE_CEILING.md (Active): a tripwire + stream-scaling benchmark harness, scripts/bench_tripwires.sh — no production code. Spec: full matrix (blit, rsyncd spun on the daemon host over ssh, rsync-over-ssh, rclone best-config, cp -a for local cells), fresh never-seen targets every timed run, CSV schema identical to docs/bench/10gbe-2026-07-05/tool_comparison.csv, one-command run against any daemon host, plus a stream-scaling probe mode (files/s vs actual negotiated stream count). Check: shell correctness (quoting, set -euo pipefail interactions, trap/teardown safety — it must never rm anything it did not create), fairness of the tool comparison methodology (fresh targets, shared data root, warm-read parity, rclone fairness flags per DIAGNOSIS.md), the tripwire/exit-code contract, the baseline-diff awk, the stream-count measurement validity, and FAST/SIMPLE/RELIABLE fit. Also check the .review/findings/sf-1-tripwire-harness.md record and the REVIEW.md row for accuracy. Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this as a code-review pass only: first I’ll read the repo guidance/state and the target diff, then check the changed script and review records against the stated sf-1 contract.
exec
/usr/bin/zsh -lc "sed -n '1,220p' /home/michael/.codex/RTK.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,240p' .agents/repo-guidance.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# RTK - Rust Token Killer (Codex CLI)

**Usage**: Token-optimized CLI proxy for shell commands.

## Rule

Always prefix shell commands with `rtk`.

Examples:

```bash
rtk git status
rtk cargo test
rtk npm run build
rtk pytest -q
```

## Meta Commands

```bash
rtk gain            # Token savings analytics
rtk gain --history  # Recent command savings history
rtk proxy <cmd>     # Run raw command without filtering
```

## Verification

```bash
rtk --version
rtk gain
which rtk
```

 succeeded in 0ms:
# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

Blit is a high-performance, extensible file enumeration, planning, transfer,
and orchestration platform for local and remote backups, migration, and
cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.

## Reading Order

This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
convention and keeps its own canonical files at different paths; the
`.agents/` files below are pointer stubs, not duplicates. Read in this order:

1. `docs/STATE.md` — single entry point for current active work, queue, and
   blockers (the canonical equivalent of `.agents/state.md`; see
   `.agents/state.md` for why the path differs).
2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
   equivalent of `.agents/decisions.md`).
5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
   repo-specific `slice` operator below).
6. Everything else in `docs/` — reference or historical; check its
   `**Status**:` header.
7. Code and tests are ground truth for behavior; plans are ground truth for
   intent. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

`DEVLOG.md` is append-only history — write to it, never read it for current
state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## Operator Vocabulary (repo-specific extension)

`AGENTS.md`'s Operator Requests section defines the toolkit's generic
vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
In this repo every one of those words resolves to a procedure in
`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
`.agents/decisions.md` files directly — read the matching section there and
execute it exactly:

- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
  now/next/blockers.
- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
  until `**Status**: Active`.
- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
  supersessions.
- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
  questions.
- `slice` (repo-specific, no generic-template equivalent) → pick up the next
  review finding and run it through the codex review loop
  (`docs/agent/GPT_REVIEW_LOOP.md`).

**Review policy (D-2026-07-04-1): every code change and every plan change
goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
exceptions.** The `.review/README.md` async sentinel hand-off is retired;
its `findings/`/`results/` records and `REVIEW.md` remain the record store.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo does not currently use `.agents/playbooks/` —
the codex review loop and `docs/agent/PROTOCOL.md` already cover that role
for review-loop work.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Test count may grow but never drop versus the prior baseline unless the
  removal is called out in the finding doc's Known gaps.
- Windows parity: after touching platform-specific code (`win_fs`, planners),
  run `scripts/windows/run-blit-tests.ps1`.
- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
  `docs/STATE.md`, unless the commit message contains `[state: skip]`
  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
  run it locally before pushing docs changes.
- Full command list and policy also live in `.agents/repo-map.json`.

## Remotes & Sync

- `origin` — `https://github.com/roethlar/Blit.git` (GitHub, canonical).
- `gitea` — `http://q:3000/michael/blit_v2.git` (LAN gitea mirror; pushed
  manually alongside or after `origin`, not auto-synced by any hook or CI
  job — it can lag GitHub by a commit or more at any given time).
- (Names verified against `git remote -v` 2026-07-04; an earlier revision
  of this doc called GitHub `github` and the mirror `origin` — that never
  matched the actual config and misread `origin/master` references.)
- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
  go well beyond a simple push policy — see Earned Practices below.

## Earned Practices

These are absolute; they exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (`docs/DECISIONS.md` D-2026-06-07-1).

- **No agent-created branches.** Agents never create git branches on their
  own decision. All work happens on `master` or the branch the owner already
  checked out.
- **Owner is the sole gate for git operations that publish, rewrite, or
  destroy.** No `push`, `push --force`/`--force-with-lease`,
  `reset --hard`, rebase or other history rewrite, `commit --amend` on
  pushed commits, or deletion of any branch/tag/ref (local or remote)
  without the owner approving that exact action in the current session.
  Working-tree edits, local commits, and read-only inspection
  (`status`/`log`/`diff`/`show`) need no special approval.
- **Branch deletion is by explicit name only** — the owner names the branch,
  the agent deletes that branch.
- **Before any push:** list the exact local refs, remote refs, and
  destination remotes, then stop and wait for approval.
- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made two now-abandoned branch tips ancestors of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches no-ops without landing any code
  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
  (`git diff <branch> master`) before treating anything as landed or
  deleting it.
- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
  checkpoint or verification step. Agents report observations; the owner
  declares pass/fail. Never self-certify a gate or continue a plan past one
  because the condition appears met. Approvals are single-use, step-specific,
  never carried across sessions. When the owner asks a question or thinks out
  loud, answer in plain English and stop — act only on an explicit decision.

## Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types
  PascalCase, constants SHOUT_CASE; match existing names (`transfer_engine`,
  `TransferOrchestrator`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, transfer engine,
  orchestrator); most logic and unit tests live here. New modules get
  re-exported in `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
  in `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
  protoc.
- Integration tests live per-crate (`crates/blit-cli/tests/`,
  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
  tooling.
- `blit-utils` was intentionally removed; its admin verbs now live in
  `blit-cli`. Do not re-add it as a workspace member.

## Commit and Docs Hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
  to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
  overflow into `DEVLOG.md`.

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/state.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-map.json` (mirrors `AGENTS.md` §5)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --no-renames 7202c1a' in /home/michael/dev/Blit
 succeeded in 0ms:
7202c1a (HEAD -> master) sf-1: tripwire + stream-scaling harness
 .review/findings/sf-1-tripwire-harness.md |  96 ++++++++
 REVIEW.md                                 |  11 +
 scripts/bench_tripwires.sh                | 384 ++++++++++++++++++++++++++++++
 3 files changed, 491 insertions(+)

codex
The repo guidance points current state to `docs/STATE.md`, and the commit only adds the harness plus review bookkeeping. I’m pulling the active plan, diagnosis notes, CSV schema, and committed file contents with line numbers so the findings can cite the exact introduced lines.
exec
/usr/bin/zsh -lc "rtk sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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

exec
/usr/bin/zsh -lc "rtk sed -n '1,260p' .review/findings/sf-1-tripwire-harness.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk sed -n '1,320p' docs/plan/SMALL_FILE_CEILING.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# sf-1 — Tripwire + stream-scaling harness

**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-1.
**Status**: implemented, codex review pending.

## What

`scripts/bench_tripwires.sh` — makes the 2026-07-05 tool-comparison
baseline (`docs/bench/10gbe-2026-07-05/`) re-runnable against any
daemon host in one command, and adds the plan's stream-scaling probe
(files/s vs the stream count the transfer actually ran with). No
production code.

## Approach

Derived from `scripts/bench_10gbe.sh` (timing/generation/fresh-target
patterns) plus the session's ad-hoc comparison methodology recorded in
DEVLOG 2026-07-05 00:51 and DIAGNOSIS.md — the ad-hoc runner itself
was never committed, so this reconstructs it durably.

- **Matrix** (schema-identical to the committed
  `tool_comparison.csv`: `transport,direction,workload,run,ms,status`):
  blit / rsyncd / rsync-over-ssh / rclone-sftp × push/pull, and
  blit / rsync / rclone / `cp -a` × local, over the baseline's three
  workloads (1 GiB large, 10k×4 KiB small, 512 MiB+5k×2 KiB mixed).
  rclone runs its best measured LAN config (`--ignore-checksum`,
  tuned `--transfers`, sftp transport) per DIAGNOSIS.md. The harness
  matrix and the plan's tripwire list are the same set by
  construction (plan acceptance criterion).
- **One command**: `DAEMON_HOST=… REMOTE_ROOT=… REMOTE_BLIT_DAEMON=…
  ./scripts/bench_tripwires.sh`. By default it spins both daemons on
  the target host over ssh — blitd via `--root` (exports the
  per-invocation session dir as module `default`, no config file
  needed) and rsyncd via a generated config — and tears both down plus
  the session dir on exit. `SPIN_DAEMONS=0` targets already-running
  daemons. All tools share one data root (session methodology).
- **Fresh targets every run** (blit and rsync both no-op onto
  already-delivered content): local dests recreated, remote push
  targets are per-run never-seen subdirs; pull sources seeded once per
  workload (seeding writes leave the ARC warm — baseline was warm
  re-reads).
- **Scale probe**: fixed 4 KiB files at counts crossing
  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
  25k→8, 50k→10 expected); records files/s and the **measured** stream
  count (per-stream `stream complete` completion lines in blitd's
  stderr, `data_plane.rs:224`, delta-counted per push). Measured-vs-
  table divergence is exactly the sf-2 evidence the plan wants the
  curve to show.
- **Tripwire verdict is the exit code**: summary prints best-of per
  cell, blit vs fastest rival; any rival win → `TRIPPED` + exit 3.
  Also diffs blit cells against the committed baseline CSV (the ±10%
  regression criterion) when present.
- Missing tools (rsync/rclone locally or remotely) skip their cells
  with a note; a wedged tool is capped by `timeout` and recorded in
  the status column rather than hanging the run.

## Files

- `scripts/bench_tripwires.sh` (new, executable)

## Tests

Script-only slice — cargo suite unaffected (run anyway: fmt, clippy,
full workspace suite green; count vs 1479 baseline in verdict file).
Script verified by execution:

- `bash -n` clean.
- **Local-only e2e** (`SIZE_MB=32 SMALL_COUNT=500 RUNS=2 … matrix`):
  all local cells timed, CSV written, summary + baseline diff printed,
  exit 3 with `cp` tripping blit on tiny local copies (harness working
  as designed; rig verdicts belong to sf-4).
- **Full remote-path e2e over loopback** (ssh shim executing "remote"
  commands locally; real network transfers to a real spun blitd +
  rsyncd on 127.0.0.1): daemon spin-up, seeding, every push/pull cell,
  scale probe with stream counting (200 files → 1 stream measured),
  teardown verified (no stray daemons, session dir removed).
  rclone-sftp cells recorded status 1 in this rig-less test (no sftp
  auth to localhost) — the status column captured it and the run
  continued, which is the designed failure path.

## Known gaps

- The scale probe is push-only (the plan's target cell); pull scaling
  can be added when a pull-side per-stream log line exists.
- Stream counting needs the daemon's stderr (`SPIN_DAEMONS=1` owns it;
  otherwise `BLITD_LOG`); against a foreign daemon the column is empty
  rather than guessed.
- rclone-sftp cells assume ssh-agent/key auth to the host (same
  requirement the session had); no rclone config file is generated.
- Loopback e2e cannot validate 10 GbE-scale numbers — sf-4 is the rig
  re-measure slice.
- Observed during loopback testing, recorded for sf-2: a 1000-file
  push rode 1 stream where the proposal table says 2 — consistent
  with the DIAGNOSIS.md one-stream-for-10k-files gap; the daemon-side
  proposal call (`control.rs:798`) and its input manifest need the
  sf-2 pins.

 succeeded in 0ms:
# Small-file transfer to the hardware ceiling

**Status**: Active
**Created**: 2026-07-05
**Supersedes**: nothing
**Decision ref**: D-2026-07-04-4 (Draft → Active, owner "go")

## Principle (owner, 2026-07-05)

blit's guiding principles are **FAST, SIMPLE, RELIABLE** — every
change serves at least one or it's scrapped. blit must be the
fastest way to transfer files in **any** scenario. Goals are
therefore **ceiling-driven, never competitor-relative**: a
"beat tool X by N%" bar embeds a stopping condition and is the wrong
way to engineer this tool. Other tools function only as
**tripwires** — any scenario where any tool measures faster than
blit is, by definition, proof blit is off its hardware ceiling and
is a finding to fix, regardless of margins.

## Goal

For the workload classes where the 2026-07-04/05 10 GbE session
measured blit off its ceiling — many-tiny-file and mixed transfers —
blit's wall time becomes bounded by a **named hardware limit** (wire,
target-filesystem parallel create floor, source enumeration floor),
demonstrated by profile evidence and a stream-scaling curve, not by
blit's own stream policy or per-file overhead.

Measured gap analysis (durable evidence:
`docs/bench/10gbe-2026-07-05/` — DIAGNOSIS.md carries the daemon-log
extracts and arithmetic; the CSVs carry every matrix cell; DEVLOG
2026-07-05 entries are the narrative record):

| cell | blit today | ceiling arithmetic | tripwire |
|---|---|---|---|
| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor: ~150 µs/file proven single-pipe on this ZFS, ÷ parallelism → **~0.2–0.5 s** | rsyncd 1.5 s |
| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol class: **≪ 200 ms** | rsyncd 367 ms |
| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small remainder as above | rsyncd 1.24 s |

Diagnosis (from the session's daemon logs): the 10k push rode **one
stream** — `engine::initial_stream_proposal` is byte-weighted, so
40 MiB proposes a single stream despite 10,000 files — and paid
~215 µs/file sequentially on the daemon. The parallel machinery
(elastic streams, work-stealing, mid-transfer resize) exists and
negotiated 8 connections for the 1 GiB push in the same session.
This is a policy gap plus per-file overhead, not missing machinery.

## Non-goals

- Competitor-relative targets of any kind (see Principle).
- WAN/latency-shaped tuning (separate scenario class; gets its own
  ceiling analysis when a rig exists).
- Non-Linux rig ceiling targets (no measurement hardware this plan
  can bind to; Windows/macOS must not regress — suite + CI guard).
- Encrypted-transport scenarios (ssh-wrapped tools measured only as
  tripwires; blit's transport security model is unchanged by this
  plan).

## Constraints

- Every slice serves FAST without violating SIMPLE (dial stays the
  single tuning owner; no second engine, no special-case paths that
  survive past their measured need) or RELIABLE (REV4 invariants:
  byte-identical, StallGuard, cancellation, byte accounting).
- No wire-visible protocol change without a dedicated owner gate on
  the wire design before code (sf-6); mixed-version peers keep
  working via existing negotiation.
- No measured cell regresses beyond run-to-run noise (±10%),
  guarded by the committed baseline.
- Test count never drops; every slice through the codex loop
  (D-2026-07-04-1).
- Small-file parallel writes must respect the receiver capacity
  profile (spinning-pool receivers bound their own parallelism —
  the existing bounded-unilateral dial contract, D-2026-06-20-1).

## Acceptance criteria

- [ ] For each cell above: a recorded **limiter analysis** (profile
      + stream-scaling curve, committed with the slice records)
      demonstrating wall time is bound by a named hardware limit,
      not by stream policy or blit-controlled per-file overhead.
- [ ] Scaling evidence: files/s rises with stream count until the
      named limiter binds — the curve flattens at hardware, not at
      policy.
- [ ] **Tripwires clean**: no tool in the committed sf-1 harness
      matrix — rsyncd, rsync-over-ssh, rclone in its best measured
      config (`--ignore-checksum`, tuned `--transfers`), and `cp -a`
      for local cells — measures faster than blit on any cell. (The
      harness and this list are the same set by construction; adding
      a tripwire tool means adding it to the harness.)
- [ ] All baseline matrix cells stay within run-to-run noise (±10%)
      of the committed `docs/bench/10gbe-2026-07-05/` baseline.
- [ ] The comparison + scaling harness is committed and the owner
      can rerun it against any daemon host in one command.

## Design

Levers, cheapest first, measuring between each — sequencing exists
to find the ceiling with the least machinery, not to stop early:

1. **File-count-aware stream proposal** (blit-core `engine/`):
   `initial_stream_proposal` (and the pull-side equivalent) weight
   file count alongside bytes so many-tiny-file manifests open
   multiple streams; work-stealing spreads per-file cost across
   daemon workers. Push knows counts from enumeration, pull from
   the manifest.
2. **Per-file cost to the syscall floor** (daemon receive + client
   pull write paths): profile first (`strace -c`/`perf` during a
   small transfer), then cut — candidates: temp-file+rename
   pattern, separate set-times/set-perms syscalls, per-file
   need-list echo. The profile, not intuition, names the cuts.
3. **Resize-on-file-backlog**: feed the existing ue-2 resize
   machinery a backlog signal so a stream drowning in tiny files
   triggers mid-transfer ADD — this is also the organic resize
   trigger byte-bound workloads can never produce.
4. **Tar-shard push lane** (wire-visible, own owner gate): bundle
   tiny files into shard frames on the push wire as the local
   engine and delegated lane already do — amortizes both protocol
   roundtrips and daemon syscalls. Reached when the limiter
   analysis shows per-file framing itself is the binding cost.

Risks: parallel small-file writes can seek-storm spinning pools —
bounded by the receiver capacity profile (constraint above); lever 2
touches platform-sensitive syscall paths — Windows suite must stay
green; lever 4 adds wire complexity — SIMPLE requires the limiter
analysis to prove it earns its keep before design review.

## Slices

1. **sf-1 tripwire harness**: commit `scripts/bench_tripwires.sh`
   (derived from the session's ad-hoc runner): full matrix — blit,
   rsyncd (spun on the daemon host over ssh), rsync-over-ssh,
   rclone best-config, `cp -a` local — fresh targets every run,
   plus a stream-scaling probe mode (files/s vs stream count). The
   2026-07-05 baseline already lives in `docs/bench/10gbe-2026-07-05/`
   (committed with this plan); sf-1 makes it re-runnable in one
   command. No production code.
2. **sf-2 dial file-count weighting**: proposal-table unit pins
   (10k tiny → multi-stream; 1×1 GiB unchanged; mixed →
   intermediate) + loopback e2e pin that a 10k-file push opens >1
   data-plane connection.
3. **sf-3a per-file cost limiter analysis** (analysis-only, w8-1b
   precedent): `strace -c`/`perf` profile of daemon receive and
   client pull-write during a small transfer; deliverable is a
   committed analysis naming each per-file syscall cost and the
   ordered list of candidate cuts, each with its expected saving.
   No code.
4. **sf-3b… one cut per slice**: each accepted cut from sf-3a lands
   as its own review-loop slice with its own loopback
   per-file-cost proxy pin (so CI catches gross regressions
   without the rig). The count of sf-3x slices is set by sf-3a's
   list, not guessed here.
5. **sf-4 rig re-measure + limiter analysis**: rerun sf-1 harness on
   the 10 GbE rig; record the limiter analysis per cell. Hardware-
   bound everywhere + tripwires clean → acceptance review with the
   owner. Otherwise the analysis names what binds; continue.
6. **sf-5 resize-on-backlog feed** (if sf-4 names stream count
   under load as a binder, or the owner wants the ue-2 organic
   trigger regardless — flagged at sf-4).
7. **sf-6 tar-shard push lane** (if sf-4/sf-5's analysis names
   per-file wire framing as the binder). Wire-visible; the owner
   gate consumes the full REV4 wire-contract deliverable set
   **before any code**: the proto messages/fields and their field
   numbers named; capability negotiation for the shard lane
   specified; old-client→new-daemon and new-client→old-daemon
   behavior stated; and mixed-version compatibility tests specified
   and landing **before** any behavior depends on the lane.
8. **sf-7 verdict**: final rig run, limiter analyses committed,
   acceptance checklist walked with the owner; plan → Shipped or
   the remaining gap gets its own named follow-on.

## Open questions

- **sf-6 wire gate** (standing): the tar-shard lane's wire design
  needs explicit owner sign-off at execution time — recorded here
  so no session treats sf-6 as pre-authorized code. — owner

exec
/usr/bin/zsh -lc "rtk sed -n '1,240p' REVIEW.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend

- `[ ]` Open — coder may pick up
- `[~]` In progress — coded/committed, codex review + adjudication in
  flight (`docs/agent/GPT_REVIEW_LOOP.md`, D-2026-07-04-1)
- `[x]` Verified — codex verdict adjudicated, accepted findings fixed
  (records: `.review/results/<id>.codex.md` + `<id>.gpt-verdict.md`;
  rows graded before 2026-07-04 carry `<id>.verified.json` from the
  retired async loop)

## Unified transfer engine (REV4) — code→GPT-review→fix loop

Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` (Active). Loop:
`docs/agent/GPT_REVIEW_LOOP.md` (D-2026-06-20-6). These rows do **not** use
the async `.review/` sentinel; status here means: `[ ]` not started ·
`[~]` coded+committed, GPT review/adjudication in flight · `[x]` reviewed,
accepted findings fixed, validation green. Records per slice:
`.review/findings/<id>.md`, `.review/results/<id>.codex.md`,
`.review/results/<id>.gpt-verdict.md`. Order/deps: REV4 §"Slice dependencies".

| ID | Title | Status | Commit(s) |
|----|-------|--------|-----------|
| ue-r2-1a | Salvage adaptive PR1+PR2 substrate; resolve StallGuard-vs-`Probe`; work-stealing behavior tests | `[x]` | `e569eea`…`771a632` + review fixes |
| ue-r2-1b | Wire dial contract: capacity profile + peer capability + resize proto (`receiver_capacity=11`); compat tests | `[x]` | `2741dc8` + review fix `5bd345a` |
| ue-r2-1c | `TransferEngine` shell + `TransferOrchestrator` as local adapter; local fast paths → engine strategies | `[x]` | `7730eb1`+`dc9b0ed`+`29e210b` + review fix `15e6334` |
| ue-r2-1d | Streaming plan foundation (partial-scan InitialPlan/PlanUpdate); prove ~1s start; RELIABLE exceptions | `[x]` | `c08a5c1` + review fixes |
| ue-r2-1e | Live cheap dials replace the `determine_remote_tuning` ladder | `[x]` | `3be9105`..`15968f4` + review fix `46da929` |
| ue-r2-1f | Push converge through the engine; retire daemon `desired_streams` ladder | `[x]` | `a4a9f70` + review fix `0c8da50` |
| ue-r2-1g | PullSync multistream through the engine (absorbs MULTISTREAM_PULL) | `[x]` | `48e583e` + review fix `4a2e58d` |
| ue-r2-1h | Delete deprecated `Pull` RPC (+ its `pull_stream_count` ladder) after harvest; port relay onto PullSync | `[x]` | `2a13f53` (+`9f37a7a` baseline/staging-slip, `48c5a11` win-1) + review fix `f6f52d7` |
| ue-r2-2 | Stream resize: negotiated `DataPlaneResize`/`Ack`, mid-transfer add/drop — **REV4 complete** | `[x]` | `042ca4b`..`0788e83` + review fix `ec4a3fe` |

## Small-file ceiling (SMALL_FILE_CEILING) — code→GPT-review→fix loop

Plan: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4).
Same codex loop and record formats as the REV4 section above. Slice
order and gates live in the plan (sf-6 is owner-gated on wire design;
sf-3b… count is set by sf-3a's analysis, rows added as filed).

| ID | Title | Status | Commit(s) |
|----|-------|--------|-----------|
| sf-1 | Tripwire + stream-scaling harness (`scripts/bench_tripwires.sh`) — baseline re-runnable in one command | `[~]` | |

## Design-review queue (ratified D-2026-06-11-2, in execution order)

Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (per-finding evidence).
Coder loop: pick the topmost `[ ]` row. W2.3 requires a `docs/plan/` doc with
**Status: Active** before code.

| ID | Severity | Title | Status | Branch | Commit |
|----|----------|-------|--------|--------|--------|
| w5-1-log-backend | Medium | Install stderr log backend (warn) in all 4 binaries + one prefix convention; today every log::warn/error is discarded | `[x]` | master | `56bda09`+`7145202` |
| w4-2-delete-push-upload-channel | Medium | Delete the 262,144-slot push upload channel (drain-and-discard; wedges gRPC-fallback pushes >262k files) | `[x]` | master | `03bcb1d` |
| w5-2-retry-classifier-consolidation | Medium | Delete dead contradictory blit-core/errors.rs; move is_retryable into blit-core with contract test | `[x]` | master | `9c960dc` |
| w4-1-abortondrop-family | High | Hoist AbortOnDrop; fix the remaining detach-on-drop sites (2 of 5 deleted with the Pull RPC at ue-r2-1h; design-2 now scopes to push/control.rs only; JoinSet for per-stream workers). Codex NEEDS FIXES (1 Low: relocated drop-test was vacuous) → fixed `bedfa52` | `[x]` | master | `65ecb93`+`bedfa52` |
| w9-5-jobs-lifecycle-e2e | Medium | jobs/detach lifecycle e2e tests (Subscribe, watch fallback, cancel exit codes) — net before W4.3 | `[x]` | master | `ad773d8` |
| w4-3-daemon-disconnect-racing | Medium | Daemon handlers race tx.closed()+cancel token (delegated_pull's select generalized to resolve_transfer_outcome + resolve_streaming_outcome; 2 live sites — pull spawn closure died with the Pull RPC at ue-r2-1h); false supports_cancellation comment fixed, dispatch policy itself unchanged (flip = open owner question, since decided D-2026-07-04-3 → w4-5). Codex PASS (0 findings) | `[x]` | master | `37d7f91` |
| w4-5-supports-cancellation-flip | Medium | Flip supports_cancellation for Push/PullSync (owner-authorized D-2026-07-04-3): CancelJob + TUI F2 work on attached transfers; policy-only after w4-3's race wiring (one-predicate flip — Pull history-only stays gated; TUI/CLI needed zero logic changes); contract change exit 2→0 pinned at table + RPC-handler level, authz now covers flipped kinds; every old-policy comment surface updated incl. proto wire-contract doc. Codex NEEDS FIXES (1 Low: module-doc scope log still claimed Pull wired) → fixed `1708075` | `[x]` | master | `05a8b39`+`1708075` |
| w1-2-data-socket-policy-helper | Medium | Shared configure_data_socket (NODELAY/keepalive/tuned buffers) hoisted to blit-core; pull client connect + daemon push/pull_sync accepts all route through it; pull_sync passes the dial's tcp_buffer_bytes (resize accept reads it live — the computed-and-discarded gap closed); daemon's silently-swallowing twin + socket2 dep deleted. design-3 (connect timeouts) untouched. Codex PASS (0 findings) | `[x]` | master | `16237e2` |
| w1-3-tcp-keepalive-honesty | Medium | Real TcpKeepalive timing (idle 60s / interval 10s / retries 5) at the single site left after w1-2 (the shared helper; daemon copy already deleted, logs-failure clause satisfied structurally) — dead idle peer detected in ~2 min, not ~2 h; comments now true; socket2 features=["all"] for retries + test getters. Codex PASS (0 findings) | `[x]` | master | `865fc1e` |
| w1-4-accept-token-constants | Low | Shared DATA_PLANE_ACCEPT_TIMEOUT(30s)/DATA_PLANE_TOKEN_TIMEOUT(15s) in remote::transfer::socket replacing the 3 declarations left at HEAD (4th died with the Pull RPC); values byte-identical. Codex NEEDS FIXES (1 Low: stall_guard comments named the deleted pair) → fixed `d17b089` | `[x]` | master | `6a19e1d`+`d17b089` |
| w2-1-delete-warmup-machinery | Medium | Delete dead auto_tune warmup branches + analyze_warmup_result (honest static table) | `[x]` | master | `2a8a490` |
| w2-2-stream-ladder-owner | Medium | Single stream-count/chunk owner: the 3 stream ladders died with REV4 (ue-r2-1e dial / -1f initial_stream_proposal takes file_count / -1h Pull RPC; absorption recorded D-2026-06-20-1); this slice closed the remaining leg — deleted the dead transfer_plan chunk lane (16/32 MiB ladder, Plan/PlannedPayloads wrappers, chunk_bytes_override + refresh sites, never-called plan_to_daemon_format, orphaned TuningParams); dial is the single chunk owner; W3.1's "settled tuning owner" = engine::TransferDial. Codex NEEDS FIXES (1 Low: new ensure_dial comment said "fallback batch" in the data-plane branch) → fixed `27f53a0` | `[x]` | master | `01209bc`+`27f53a0` |
| w2-3-multistream-pull-plan | High | Multi-stream pull-sync: write plan doc (authorized D-2026-06-11-2), harvest deprecated Pull's pattern, implement — absorbed into REV4 (D-2026-06-20-1); delivered as `ue-r2-1g` | `[x]` | master | `48e583e` |
| w2-4-delete-pull-rpc | High | Delete deprecated Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); port scan_remote_files — absorbed into REV4; delivered as `ue-r2-1h` | `[x]` | master | `2a13f53` |
| w3-1-memory-aware-buffer-pool | High | BufferPool::for_data_plane(chunk_bytes, streams) owns the formula (streams*2+4, shared 64 KiB DATA_PLANE_BUFFER_FLOOR) + available/4 memory cap with a 2-buffers-per-stream liveness floor (buffer shrinks, never concurrency — the double-buffered sender holds 2); replaces the 3 pasted sites; elastic paths authorize dial.ceiling_max_streams() up front (closes both "growing the pool live is a W3.1 concern" deferrals); fixes the sysinfo units bug (0.38 returns bytes; old *1024 over-reported memory 1024x, making every cap vacuous); RECEIVE_CHUNK_SIZE comment truth. 8 params-layer pins, mutation-verified. Codex PASS (0 findings) | `[x]` | master | `f49f8f6` |
| w6-1-progress-event-contract | Medium | ProgressEvent contract owned by blit-core: bytes ride Payload only (FileComplete's bytes field DELETED — design-1's class unrepresentable); files count once via byteless FileComplete{wire-relative path} or Payload.files (aggregate lane: delegated bridge, tar-shard appliers); ManifestBatch = direction-flavored denominator, documented. All producers normalized (TCP receive double-emit fixed; tar-shard members + resume lanes gain missing events; send side moves planned bytes to Payload; gRPC pull absolute-path leak fixed; 2 dead emitters conformed pending w8). Consumers collapsed onto shared ProgressTotals (CLI monitor — closes design-1 — + all 3 TUI forwarders; TUI's 3 accumulate_* rules deleted). +12 blit-core tests incl. 4 producer emission pins, 2 mutation-verified; 1460→1472/0/2. Codex PASS (0 findings) | `[x]` | master | `8fd8978` |
| w6-2-progress-residue-verify | Medium | Verify-then-fix map §1.6 residue: all three claims CONFIRMED at HEAD `8fd8978` (delegated live progress wire-dead — zero BytesProgress producers; daemon row counters fed only by delegated dispatch core.rs:667 — push receive + pull_sync serve stay 0; TransferProgress/GetState/TransferComplete hardcode bytes_total/files_* to 0). Per the ratified spec each confirmed item became its own follow-on: w6-2a/-2b/-2c filed in the pending-review section (independent slices — 2a needs only the already-fed delegated counter; suggested order 2b→2a→2c on smallest-first grounds, coder's pick). Verification + filing only, no code. Codex NEEDS FIXES (2 Low, doc-coherence: "no code anywhere constructs" overstated vs consumer tests; 2b-as-substrate-for-2a wording) → both fixed | `[x]` | master | `0aba593` + fix |
| w4-4-blocking-work-off-runtime | Medium | Blocking work off the runtime: push manifest requires-upload checks (canonical containment walk + stat, ~3M+ syscalls per 1M-file push, previously inline on a tokio worker) now buffer and run in chunked spawn_blocking batches (MANIFEST_CHECK_CHUNK=128 = need-list early-flush threshold; lexical-containment alternative rejected — weakens F2); need-list order kept, mid-manifest TCP spin-up moved to post-chunk-drain, ManifestComplete drains the remainder, design-4 untouched. collect_pull_entries_with_checksums runs ENTIRELY on one spawn_blocking thread (single-file branch's inline full-file Blake3 + metadata probes were pinning a worker). +4 tests incl. containment-escape via the batched path, mutation-verified; 1472→1476/0/2. Codex NEEDS FIXES (1 Medium: chunk-only draining muted the batcher's 64KiB/5ms early-flush for trickling manifests) → manifest_drain_due chunk-or-delay trigger, fixed `768e7e3` | `[x]` | master | `0feca34`+`768e7e3` |
| w9-1-ungate-windows-tests | High | Remove blanket #[cfg(unix)] from remote transfer tests with nothing unix-specific | `[x]` | master | `9324559` |
| w9-2-revive-root-tests | Medium | Relocate dead workspace-root tests/ into blit-core/tests (MirrorPlanner coverage); delete connection.rs; fix AGENTS.md §4 | `[x]` | master | `461525d` |
| w9-3-test-harness-builder | Medium | One daemon-spawn harness: TestContext::builder() (read_only/delegation/extra_daemon_args) + spawn_daemon/spawn_second_daemon absorb the SEVEN clones at HEAD (audit counted 5; w9-4/w9-5 had each added another — the finding's prediction twice proven) plus 5 cli_bin/7 run_with_timeout/4 ChildGuard copies; daemon build OnceLock'd per test binary (R16-F1 independence kept; was ~75 nested cargo invocations serializing on the build-dir flock — the daemon-spawn load-flakiness home); new blit_core::remote::grpc_server owns the audit-1 HTTP/2 keepalive (30s/20s) as production_server_builder() — daemon main.rs + all FIVE fake tonic servers (not 3: remote_remote ×2, jobs_lifecycle, pull_sync_with_spec_wire ×2) route through it, zero bare Server::builder() left; port-collision race surfaced by the build de-serialization fixed two-layer (process-global claimed-port set + child-death readiness check). Net −1,251 test-tree lines; 1478→1479 same-method A/B (+1 keepalive pin, mutation-verified). Codex NEEDS FIXES (1 Medium: fake-server :0 bind bypassed the claimed set — wrong-listener race for mixed fake/daemon binaries) → claim_port() shared, fixed | `[x]` | master | `f6e592e`+`8641bc6` |
| w9-4-readonly-enforcement-tests | Medium | Tests for all 3 read-only-module gates (push, purge, delegated pull) — zero coverage today | `[x]` | master | `4d67210` |
| w7-1-mirror-executor-consolidation | Medium | One mirror/purge deletion executor + parallel enumerate_local_manifest in blit-core (R58-F3 class closure) | `[ ]` | — | — |
| w7-2-filter-spec-chokepoint | Medium | filter_from_spec pub; push handler uses validated chokepoint (mirror-purge filter currently unvalidated) | `[ ]` | — | — |
| w7-3-wire-metadata-helpers | Medium | Wire metadata + path helpers into blit-core; one mtime error convention; delete per-crate twins | `[ ]` | — | — |
| w7-4-hash-reader-helper | Medium | checksum::hash_reader owning the 256 KiB loop; daemon build_file_header calls it | `[x]` | master | `6b2f433` |
| w7-5-presenter-formatting | Medium | format_bps in blit_app::display (binary units); switch jobs.rs + 5 TUI copies | `[ ]` | — | — |
| w7-6-default-port-pub | Low | RemoteEndpoint::DEFAULT_PORT pub; delete 9031 literals | `[x]` | master | `de04054` |
| w8-1-foundation-deadcode-sweep | Medium | Delete tar_stream, delete.rs, copy/parallel+stats, chunked_copy_file, fs_enum leftovers (~800 lines). zero_copy EXCLUDED → w8-1b | `[ ]` | — | — |
| w8-1b-zero-copy-fast-eval | Medium | Evaluate wiring splice/zero_copy into the receive pipeline (owner: FAST potential); outcome = plan doc or deletion | `[x]` | master | `6189d82` |
| w8-2-delete-control-plane-payload | Medium | Delete transfer_payloads_via_control_plane (zero-caller duplicate); sequence with W1.1 chunk_bytes deletion | `[ ]` | — | — |
| w8-3-deadcode-hygiene-sweep | Low | --interval-ms flag, blit-cli unused deps, blit-app stubs, stale #[allow(dead_code)] sweep | `[ ]` | — | — |
| w5-3-daemon-status-helpers | Medium | internal_err({:#}) + io_to_status helpers; sweep ~69 chain-amputating + 116 Status::internal sites | `[ ]` | — | — |
| w5-4-mpsc-sendfail-vocabulary | Medium | One honest mpsc send-failure vocabulary; prefer joining the exited task's real error | `[ ]` | — | — |
| w5-5-logger-trait-cleanup | Low | Logger trait permanently-noop error channel cleanup | `[ ]` | — | — |
| w9-6-test-misc | Low | Harness stderr capture; tuning-tier unit tests | `[ ]` | — | — |
| w10-docs-batch | Medium | Docs batch: AGENTS.md ghost names, WORKFLOW_PHASE_2 re-status, --resume/--retry help scoping (help+manpage+README), comment-truth sweep | `[ ]` | — | — |

## Currently pending review

| ID                | Severity | Title                                       | Status | Branch      | Commit    |
|-------------------|----------|---------------------------------------------|--------|-------------|-----------|
| relay-1-subpath-double-join | Low | `--relay-via-cli` with a subpath source scans `sub/sub` (endpoint rel_path joined twice). Pre-existing (deleted Pull-RPC code had the identical join); surfaced by the ue-r2-1h self-review panel; port kept parity, fix deferred | `[ ]` | — | — |
| win-1-push-needlist-separators | High | Windows daemon push need-list echoed native separators — every nested push to a Windows daemon stalled 30s. One-line `relative_path_to_posix` fix; reviewed within the ue-r2-1h codex+panel batch | `[x]` | master | `48c5a11` |
| design-1-cli-pull-byte-double-count | Medium | CLI pull progress double-counts bytes on the TCP data plane (producer reports both Payload and FileComplete with full bytes; CLI fold adds both). From design map §1.6, hand-verified. Fixed structurally by w6-1 (producer double-emit removed AND FileComplete's bytes field deleted — the class is unrepresentable); graded within the w6-1 codex round | `[x]` | master | `8fd8978` |
| design-2-orphaned-daemon-data-planes | High | Daemon data-plane tasks detach (not abort) on control-stream death at 3 spawn sites; orphan unreachable by CancelJob. AbortOnDrop fix exists but never propagated. From design map §1.9, hand-verified. Fixed by w4-1 (2 of 3 sites deleted with the Pull RPC at ue-r2-1h; remaining push/control.rs site now wrapped); graded within the w4-1 codex round | `[x]` | master | `65ecb93` |
| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
| w6-2a-delegated-bytesprogress-producer | Medium | Delegated live progress is wire-dead: proto BytesProgress has zero producers — the dst daemon sends Started, silence, then one post-hoc ManifestBatch (delegated_pull.rs:363-369 deliberate 0.1.0 gap, :433). The row atomic is ALREADY fed (core.rs:667); bridge it onto the DelegatedPullProgress stream on the progress tick so CLI footer + TUI delegated pane go live. Client side needs nothing (w6-1 aggregate lane + report_bytes_progress ready). Filed by w6-2 verification | `[ ]` | — | — |
| w6-2b-daemon-counters-push-pullsync | Medium | Daemon row byte counters stay 0 for push receive (FsTransferSink built without with_byte_progress, push/data_plane.rs:1086 passes None) and pull_sync serve (no counter at all 3 send pipelines, pull_sync.rs:635/:765/:795) — GetState/TransferProgress/TransferComplete all report 0 bytes for 2 of 3 active kinds. Wire job.bytes_counter() through both handlers (independent of 2a, whose delegated counter is already fed). Filed by w6-2 verification | `[ ]` | — | — |
| w6-2c-daemon-progress-denominators | Medium | Daemon event stream has no denominators or file counts: TransferProgress hardcodes bytes_total/files_completed/files_total 0 (core.rs:240-242), TransferComplete.files 0 (:322-325, + tcp_fallback_used false :329), GetState bytes_total 0 (:994-996) — "N of M"/percent impossible for every consumer. Thread manifest totals + a files counter onto ActiveJobs rows. Filed by w6-2 verification | `[ ]` | — | — |
| design-4-fallback-midmanifest-negotiation | High | Forced-gRPC pushes fail at ≥128 files (FILE_LIST_EARLY_FLUSH_ENTRIES; ~100 flaky). Mechanism VERIFIED two-sided: daemon announced fallback negotiation mid-manifest AND a force_grpc client streamed FileData with no negotiation at all — both racing the daemon manifest loop's FileData rejection. Fixed: daemon early-flush branch TCP-only; client gates fallback sends on fallback_negotiated. Owner-ratified 2026-06-12. NOTE: grade before design-5 (sequential overlapping commits on push/client/mod.rs) | `[x]` | master | `ddfeb58` |
| design-5-send-failure-masks-rejection | Medium | Push rejection reason (e.g. read-only) masked by 'failed to send push request payload' when the client loses the send-vs-status race — first CI failure surfaced by the w9-1/w9-4 ungating (macOS+Windows). Fixed: prefer_server_error harvests the daemon's terminal status on send failure at the 3 manifest-phase sites; 500-file deterministic regression. Owner-ratified 2026-06-12 ("strong bias for proper fixes"). Grade after design-4 | `[x]` | master | `08d71a2` |
| audit-h1-mirror-relay-incomplete-scan | Data-loss | Reject `mirror --relay-via-cli` for remote→remote (round 2: gate moved before mirror confirm prompt + yes=false regression test) | `[x]` | `master` | `4467faf` |
| audit-h3a-push-receive-stall | Robustness | StallGuard on the daemon push-receive socket (`TRANSFER_STALL_TIMEOUT` hoist) — closes one of three remaining stall-guard gaps from R3 H3; symmetric with audit-1c CLI pull-receive | `[x]` | `master` | `dd51a1c` |
| audit-m28-tui-sot-sweep | Docs | TUI source-of-truth sweep (round 2: audit INDEX + R3 updated to record 2026-06-04 owner ratification of H10b + resolution of L39/M27/M28) | `[x]` | `master` | `15fabbf` |
| audit-l39-m27-env-var-purge | Convention | Owner-directed env-var purge (round 2: bench-script prose + Clap `hide_short_help` doc corrections in 3 sites) | `[x]` | `master` | `ec06a95` |
| audit-h11-f1-confirm-detail-err | Data-loss UI | F1 confirm-detail explicit Local/Remote/Err arms + `debug_assert!` (round 2: re-armed at build-fix HEAD; h11 logic itself was correct in dirty tree, blocked by uncommitted Phase 6 dual-pane modules) | `[x]` | `master` | `1b3cb39` |
| audit-h3b-pull-data-plane-write-stall | Robustness | New `StallGuardWriter<W>` wired inside `DataPlaneSession` (round 2: same build-fix re-arm + tightened `Ok(0)` semantics so a zero-byte poll_write doesn't reset the deadline) | `[x]` | `master` | `1b3cb39` |
| audit-h3c-slice1-grpc-fallback-frame-contract | Robustness | Slice 1 of 2: gRPC fallback chunk cap at 1 MiB (`GRPC_FALLBACK_CHUNK_BYTES`) decoupled from TCP tuning; 3 CLI pull receive sites routed through `recv_fallback_message` (the chokepoint slice 2 will wrap with the dynamic progress watchdog). Round-2 adversarial concerns all addressed. Verified 2026-06-11 (owner accept; review assessment found the cap also fixes the tonic 4 MiB decode-limit failure — see DEVLOG, feeds slice-2 re-scope) | `[x]` | `master` | `bf4cc82` |
| d-62-f1-trigger-error | Feature | Inline validation feedback in the F1 trigger modal (round 2) | `[x]` | `phase5/a1` | `0b47a72` |
| d-63-f1-push-progress | Feature | Live byte/file footer for the F1 push (round 2) | `[x]` | `phase5/a1` | `aba54f8` |
| d-64-f1-push-ttl | Feature | Auto-hide the F1 push outcome footer (round 2) | `[x]` | `phase5/a1` | `2f67e96` |
| d-65-f1-push-mirror-move | Feature | Mirror/move for the F1 push direction (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-66-f4-clear-confirm | Feature | y/N gate on the F4 profile-history clear (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-67-help-clear-confirm | Feature | Flag the F4 clear y/N confirm in the `?` keymap (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-68-f1-remote-remote-copy | Feature | Remote→remote delegated copy from the F1 trigger (round 4) | `[x]` | `phase5/a1` | `c93bcd6` |
| d-69-f1-delegated-progress | Feature | Live byte/file footer for remote→remote delegated copy | `[x]` | `phase5/a1` | `2f1f5d2` |
| d-70-f1-delegated-mirror | Feature | Remote→remote delegated mirror from the F1 trigger | `[x]` | `phase5/a1` | `0b98666` |
| d-71-f1-delegated-move | Feature | Remote→remote delegated move from the F1 trigger (round 3) | `[x]` | `phase5/a1` | `57ed8e9` |
| m2f-1-f2-source-daemon | Feature | Tag F2 transfer rows with their source daemon (multi-daemon F2 step 1) | `[x]` | `phase5/a1` | `aeac25d` |
| m2f-2-f2-composite-key | Feature | Key F2 transfers by (daemon, transfer_id) (multi-daemon F2 step 2, round 2) | `[x]` | `phase5/a1` | `1aed724` |
| m2f-3-f2-merge-snapshot | Feature | Additive per-daemon snapshot hydration + refresh identity fix (multi-daemon F2 step 3) | `[x]` | `phase5/a1` | `7202418` |
| m2f-4-f2-tagged-events | Feature | Carry the source daemon per F2 stream event (multi-daemon F2 step 4) | `[x]` | `phase5/a1` | `8979ff2` |
| m2f-5-f2-fanout | Feature | F2 watches all discovered daemons via merged Subscribe streams (multi-daemon F2 step 5, round 2) | `[x]` | `phase5/a1` | `49f1fce` |
| m2f-6-f2-daemon-column | Feature | Render the source-daemon column in F2 tables (multi-daemon F2 step 6) | `[x]` | `phase5/a1` | `a5456cc` |
| m2f-7-f2-multi-daemon-cancel | Feature | Single cancel (K) targets the selected row's daemon (multi-daemon F2 step 7) | `[x]` | `phase5/a1` | `bbd0084` |
| m2f-8-f2-batch-cancel | Feature | Batch cancel (X) targets each active row's own daemon (multi-daemon F2 step 8) | `[x]` | `phase5/a1` | `dfdaabd` |
| m2f-9-f2-discovery-refan | Feature | Auto re-fan F2 when the discovered-daemon set changes (multi-daemon F2 step 9, round 3) | `[x]` | `phase5/a1` | `9204a4d` |
| e-8-config-default-remote | Feature | Fall back to `[daemon] default_remote` config when no --remote flag (Milestone E) | `[x]` | `phase5/a1` | `bf56a66` |
| m2f-10-f2-per-daemon-health | Feature | Partial-degrade F2 banner when one daemon's stream drops (multi-daemon F2 step 10) | `[x]` | `phase5/a1` | `365be9a` |
| e-9-theme-f2-row-highlight | Feature | F2 active-row highlight honors `[theme] accent_color`, contrasting fg (Milestone E, round 2) | `[x]` | `phase5/a1` | `7dd3e31` |
| e-10-theme-f3f4-highlight | Feature | F3/F4 selection highlights honor `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `895fe06` |
| e-11-theme-f1-highlight | Feature | F1 daemon-list highlight honors `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `ab85658` |
| bridge-1-prometheus-scaffold | Feature | Prometheus bridge step 1: GetState→prom-text formatter + print-once CLI (Milestone E, round 2) | `[x]` | `phase5/a1` | `9411754` |
| bridge-2-prometheus-http | Feature | Prometheus bridge step 2: long-running /metrics HTTP server, pull-model scrape (Milestone E, round 3) | `[x]` | `phase5/a1` | `5eb9e61` |
| bridge-3-prometheus-readme | Docs | Prometheus bridge step 3: operator README (usage, scrape config, metric reference) (Milestone E) | `[x]` | `phase5/a1` | `9561fb2` |
| keys-1-config-quit | Feature | Key remapping step 1: configurable quit key via `[keys]` config + KeyMap (Milestone E, round 2) | `[x]` | `phase5/a1` | `19c6b7f` |
| keys-2-config-refresh | Feature | Key remapping step 2: configurable refresh key via `[keys]` config, quit/refresh collision policy (Milestone E, round 2) | `[x]` | `phase5/a1` | `ead1adb` |
| keys-3-config-pane-switch | Feature | Key remapping step 3: configurable pane-switch digit aliases + generalized collision policy (Milestone E) | `[x]` | `phase5/a1` | `43d5842` |
| dark-1-theme-base-colors | Feature | dark/light step 1: configurable `[theme] background`/`foreground` base layer (Milestone E) | `[x]` | `phase5/a1` | `775bbe7` |
| dark-2-theme-mode-preset | Feature | dark/light step 2: `[theme] mode = dark\|light` presets (explicit colors override, incl. invalid→terminal-default) (Milestone E, round 2) | `[x]` | `phase5/a1` | `ce4c50f` |
| keys-4-config-movement | Feature | Key remapping step 4: configurable list-cursor aliases `[keys] move_down/up/top/bottom`, lowest-precedence in the collision policy, arrow/Home/End failsafe (Milestone E) | `[x]` | `phase5/a1` | `f9e3378` |
| rec-1-recent-persistence | Feature | Persist `GetState.recent[]` across daemon restarts via dedicated recents.jsonl (separate from planner's perf_local.jsonl); non-blocking write-through + atomic rewrite, opt-in (recent-persistence step 1) | `[x]` | `phase5/a1` | `7c095b2` |
| rec-2-clear-recent | Feature | `ClearRecent` RPC: wipe recent ring + recents.jsonl, never touching planner's perf_local.jsonl (core safety test); empty request, count response (recent-persistence step 2) | `[x]` | `phase5/a1` | `9c2955e` |
| rec-3-tui-clear-recent | Feature | F2 `E` "clear recent" action: empties local view + fans ClearRecent RPC to watched daemons (fire-and-forget); blit-app client helper; footer hint (recent-persistence step 3, final) | `[x]` | `phase5/a1` | `00d2ba5` |
| audit-3a-mutex-poisoning | Robustness | Recover poisoned ActiveJobs table/recent mutexes via `unwrap_or_else(into_inner)` instead of `expect` panic cascade (audit-3 part 1 of 2) | `[x]` | `phase5/a1` | `198ff31` |
| audit-3b-rng-fallible | Robustness | `generate_token` returns `Result` (RNG failure → Status::Internal) instead of panicking the spawned data-plane task; 6 callers propagate via `?` (audit-3 part 2 of 2) | `[x]` | `phase5/a1` | `eeb7c16` |
| audit-5a-bridge-correctness | Robustness | Prometheus bridge: one-shot scrape timeout (8s, fail-loudly) + `\r` escaping in escape_label (audit-5 part 1 of 2) | `[x]` | `phase5/a1` | `f6d2d2d` |
| audit-1a-delegation-port-zero | Robustness | Reject IANA-reserved source port 0 at the delegation gate before DNS/connect (audit-1 item 5; timeouts deferred to audit-1b + owner decision on idle-timeouts) | `[x]` | `phase5/a1` | `a3147b6` |
| audit-1b-net-timeouts-keepalive | Robustness | Delegation DNS-resolve (10s) + dst→src connect (30s) timeouts via net_timeout::within; daemon HTTP/2 keepalive (30s/20s) reaps vanished subscribers — owner-decided over idle-close (audit-1 items 1/2/4) | `[x]` | `phase5/a1` | `1d88fea` |
| audit-7-cargo-lock | Style | Track Cargo.lock for reproducible builds (4-binary workspace); remove from .gitignore (audit-7 item 10, owner-approved — supersedes the never-add rule for the lockfile only) | `[x]` | `phase5/a1` | `dfaecfe` |
| rec-4-clear-recent-confirm | Feature | F2 `E` clear-recent now asks `clear recent? y/N` first (owner-requested); reuses F2CancelStatus confirm machinery via ConfirmingClearRecent variant | `[x]` | `phase5/a1` | `3673ee1` |
| audit-2a-cli-connect-timeout | Robustness | blit_app::client::connect_with_timeout + swap all 10 admin BlitClient::connect sites incl jobs::query. Round 2: DNS-aware outer timeout (connect_timeout alone didn't bound slow DNS). Round 3: corrected stale connect_timeout docs (audit-2 part 1 of 2) | `[x]` | `phase5/a1` | `179f5fa` |
| audit-2b-remote-connect-timeout | Robustness | Bound remaining connects DNS-aware: RemotePull/PushClient::connect at source (fixes 3 data-path sites) + transfers/remote 2 BlitClient sites + blit-cli completions (audit-2 part 2 of 2) | `[x]` | `phase5/a1` | `40ed2d6` |
| audit-4-windows-handle-leak | Bug | RAII OwnedHandle guard closes the CreateFileW handle on every exit path in capture_snapshot (was leaked on the GetFileInformationByHandle `?`). Windows target cargo check passed with `CARGO_FEATURE_PURE=1`; target clippy blocked by pre-existing Windows warnings; Darwin gates pass | `[x]` | `phase5/a1` | `4e77897` |
| audit-5b1-bridge-listener-write | Robustness | Bridge: SO_REUSEADDR listener (build_listener via TcpSocket) + response write timeout (write_all_within, 10s) (audit-5 items 5/6; part 1 of 2 for the server hardening) | `[x]` | `phase5/a1` | `28e9956` |
| audit-5b2-bridge-server-lifecycle | Robustness | Bridge: graceful ctrl_c shutdown + Semaphore concurrency bound (MAX_CONCURRENT_SCRAPES=64) in serve() (audit-5 items 3/4; part 2 of 2 — completes audit-5) | `[x]` | `phase5/a1` | `05f77ec` |
| audit-6d-path-safety-unicode | Test Gap | path_safety: lock in Unicode-opaque containment boundary (NFC/NFD, bidi U+202E, ZWJ, separator/dot lookalikes) — preserved verbatim, can't smuggle traversal; non-UTF-8 unreachable via &str (audit-6 item 4) | `[x]` | `phase5/a1` | `d75cdcf` |
| audit-11-data-plane-underflow | Bug | send_file_double_buffered: clamp each read to `remaining` before subtracting — an over-returning reader (file grew / lying TransferSource) underflowed remaining (debug panic / release u64::MAX runaway) and could push undeclared bytes; now sends exactly header.size | `[x]` | `phase5/a1` | `6a0feb0` |
| audit-12-buffer-pool-leak | Robustness | BufferPool acquire/try_acquire: defer std::mem::forget(permit) until after the vec! allocation so an alloc panic releases the memory-budget permit by unwind instead of leaking it (permit leak → pool starvation) | `[x]` | `phase5/a1` | `326b3ff` |
| audit-9-cancel-auth | Bug | CancelJob now authorizes the caller against the transfer's originating peer (host/IP-only, port-insensitive; loopback + UDS bypass); cross-tenant cancel → PermissionDenied. New CancelOutcome::Unauthorized | `[x]` | `phase5/a1` | `3c5a398` |
| audit-10-cancel-completion-race | Bug | DelegatedPull select: order the handler branch first in the biased select (via resolve_delegated_pull_outcome helper) so a completion wins over a simultaneous CancelJob — was mis-recording a success as "cancelled via CancelJob" | `[x]` | `phase5/a1` | `3601f1e` |
| audit-8-tui-task-leak | Robustness | TUI Subscribe forwarder races tx.closed() (via forward_step) so it exits on F2 re-fan even for a silent daemon (was leaking conn+Receiver+slot); + outer tokio::time::timeout around jobs::subscribe open. Round 2: also bound the initial GetState snapshot fetch (fetch_snapshot_within → degraded Err) | `[x]` | `phase5/a1` | `2d7b6f7` |
| audit-6f-dns-rebinding-test | Test Gap | delegation_gate DNS-rebinding regression: ScriptedResolver returns IP A then B; assert the gate resolves once, binds A, never consults B (+ converse: denies on the first resolution only) (audit-6 item 6) | `[x]` | `phase5/a1` | `28e0b95` |
| audit-6g-copy-fallback-test | Test Gap | copy_file fast-path→fallback: byte-identical copy on all platforms + macOS clonefile-EEXIST forces the clonefile→fcopyfile hop. Round 2: assert clone_succeeded to pin the hop (not the buffered tail). Buffered-tail needs a production seam (flagged) (audit-6 item 7) | `[x]` | `phase5/a1` | `4c4db89` |
| audit-7e-cleanup | Style | Remove 33 tracked AppleDouble ._* sidecars + 2 empty npm stubs (package.json/lock); gitignore ._*. Rust-only workspace, no build/test impact (audit-7 code-health) | `[x]` | `phase5/a1` | `16a92ce` |
| audit-6c-bridge-http-integration | Test Gap | bridge end-to-end HTTP test: real client → handle_conn over loopback; GET /metrics (unreachable daemon → 200 + blit_daemon_up 0) and GET /favicon → 404 (audit-6 item 3) | `[x]` | `phase5/a1` | `02c7a9c` |
| audit-7b-dead-code | Style | Remove dead compare.rs fns (+orphaned imports), 3 STALE fs_enum allow(dead_code) (fields are live), write-only diagnostics written_at field, empty blit-app progress.rs stub. remote_remote_direct.rs left (live, 285 lines) (audit-7 code-health) | `[x]` | `phase5/a1` | `5a5f735` |
| audit-6a-blit-app-filter-tests | Test Gap | blit-app transfers/filter.rs: 6 tests on build/build_spec (glob/size/age propagation, reference_time capture, malformed-glob + bad-size rejection, Duration→secs). Note: "zero #[cfg(test)]" premise stale — 8 files already tested (audit-6 item 1) | `[x]` | `phase5/a1` | `8820226` |
| audit-7c-docs | Docs | ARCHITECTURE.md: add blit-app/blit-tui/blit-prometheus-bridge crate sections + diagram; complete gRPC surface to all 15 RPCs (verified vs proto). README: fix clone URL your_org→roethlar/Blit. Round 2: bridge as blit-app consumer, full module table (check/scan/display), F4=profile/verify/diagnostics (audit-7 code-health) | `[x]` | `phase5/a1` | `a11845a` |
| audit-6b-tui-render-test | Test Gap | F4 render_into driven through ratatui TestBackend (Profile+Verify+Diagnostics+Transfer): renders default state at 120x40 + tiny 8x3 area, asserts no panic (clamp). f2/f3/help already covered (audit-6 item 2) | `[x]` | `phase5/a1` | `267f093` |
| audit-6e-move-directory-coverage | Test Gap | directory-tree push-move + pull-move integration tests (recursive copy-then-delete-source; all files land + entire source removed). All 4 cardinal directions already covered single-file; this fills the multi-file gap (audit-6 item 5). Round 2: assert recursive remote source-tree removal on pull-move | `[x]` | `phase5/a1` | `6d410ac` |
| audit-1c1-stall-guard | Robustness | StallGuard<R> AsyncRead idle-timeout adapter (no-bytes-for-30s → TimedOut; re-armed per read = idle not total). Owner scope=all pulls. Part 1 of 2; part 2 wires it into the receive pipeline (audit-1c) | `[x]` | `phase5/a1` | `0cfa534` |
| audit-1c2-stall-wiring | Robustness | Wire StallGuard into the receive pipeline: generic-ize execute_receive_pipeline + 6 read helpers over AsyncRead, wrap the socket in pull.rs (unconditional → all pulls). Completes audit-1c (audit-1 item 3) | `[x]` | `phase5/a1` | `906cedf` |
| retry-wait1-classifier-loop | Feature | retryable-error classifier (transient io kinds incl. StallGuard TimedOut; fatal eyre/path/gate not retried) + run_with_retries loop in blit-app. Owner-approved --retry/--wait part 1 of 2; part 2 adds flags+wiring | `[x]` | `phase5/a1` | `e5e59fb` |
| retry-wait2-cli-wiring | Feature | --retry<N>/--wait<SECS> on TransferArgs (default 0/5) + wrap run_transfer/run_move in run_with_retries (resumable retry on transient failures). Completes the retry-wait feature (owner-approved follow-up) | `[x]` | `phase5/a1` | `68b34ac` |
| audit-13-buffer-pool-double-locking | Performance | BufferPool release/return_vec: single-lock cache return via cache_returned_buffer + drop redundant per-release buffer_size zeroing (truncate common path); verified no consumer relies on pre-zeroed pool buffers (Gemini-sourced) | `[x]` | `phase5/a1` | `f9d3f2f` |
| audit-14-resume-copy-redundant-seek | Performance | resume_copy_file: drop the redundant per-iteration src seek (sequential) + track dst_cursor_pos to seek dst only on divergence. Pure syscall reduction; existing byte-exact resume suite covers it (Gemini-sourced) | `[x]` | `phase5/a1` | `b7f8177` |
| audit-15-grpc-missing-connection-timeouts | Robustness | RECOMMEND-DEFER (analysis only, no code): blanket Server::timeout(30s) would kill the 7 streaming RPCs (Subscribe/DelegatedPull/Pull/PullSync/Push/Find/DiskUsage); dead-peer case already covered by audit-1b keepalive. Reviewer to grade decision (Gemini-sourced) | `[x]` | `phase5/a1` | `f0ed9e5` |
| audit-7d1-extract-progress-accum | Refactor | main.rs split part 1: extract 5 pure progress helpers (accumulate_pull/push/delegated_progress, pull_throughput, du_total_from_entries) verbatim into crate::progress_accum; crate-root use keeps call sites + tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `5112705` |
| audit-7d2-extract-display-f3 | Refactor | main.rs split part 2: extract 4 pure F3 state→display mappers (f3_pull_to_display + private confirm_detail, f3_du_to_display, f3_del_to_display) verbatim into crate::display_f3; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `315f923` |
| audit-7d3-extract-display-f1 | Refactor | main.rs split part 3: extract 4 pure F1 state→display mappers (f1_trigger_prompt, f1_push_status + private push_present_verb/push_past_verb) verbatim into crate::display_f1; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `1e50f7d` |
| audit-7d4-extract-display-f2 | Refactor | main.rs split part 4: extract 2 pure F2 cancel mappers (cancel_status_to_display, cancel_status_remaining_ttl) verbatim into crate::display_f2; F2CancelStatus stays in main.rs (event loop mutates it), referenced read-only via crate-root path; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `0ed685a` |
| audit-7d5-extract-config-reload | Refactor | main.rs split part 5: extract Ctrl+R config hot-reload helpers (reload_tui_config I/O wrapper + pure classify_reload) verbatim into crate::config_reload; ReloadBanner stays in main.rs (AppState field), referenced via crate-root path; reload_tui_config re-exported at crate root, classify_reload imported test-locally (sole non-test caller moved with it). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `4e728b5` |
| audit-7d6-extract-tick-budget | Refactor | main.rs split part 6: extract pure sleep-budget math (compute_tick_budget, min_opt — Duration/Option, no AppState) verbatim into crate::tick_budget; crate-root use keeps event-loop call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `a99a136` |
| audit-7d7-extract-del-request | Refactor | main.rs split part 7: extract 3 pure F3-delete request builders (del_wire_path, build_delete_request, is_deletable_remote_path — no async/AppState) verbatim into crate::del_request; crate-root use keeps dispatcher + plan_f1_* + spawn tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `b18e5f9` |
| audit-7d8-extract-exec-plan | Refactor | main.rs split part 8: extract 3 pure transfer-execution builders (f3_pull_options, build_f1_push_execution, build_delegated_execution — no async/AppState/IO) verbatim into crate::exec_plan; remove_local_source stays in main.rs (does IO); crate-root use keeps spawn_* tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `d47cc24` |
| audit-7d9-extract-theme-color | Refactor | main.rs split part 9: extract 2 pure theme-color mappers (base_theme_style, raw_color_to_ratatui — no AppState) verbatim into crate::theme_color; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d). NOTE: plan_f1_trigger/plan_f1_delegated inspected + NOT moved (mutate &mut AppState, coupled). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `6ddce2e` |
| bug-mirror-literal-backslash | Bug fix | Mirror failure on POSIX filenames containing literal `\` (e.g. macOS Logic Pro `1\4 Single.pst`): blanket `path.to_string_lossy().replace('\\', "/")` at 11 sites was destructive. New canonical `blit_core::path_posix::relative_path_to_posix` (component-walk) + every Path→wire site routed through it. Round 2: GPT-reopen — `relative_str_to_posix` was dropping trailing `/` in admin completion input (regressed dir-prefix completions); now preserves trailing-separator UX semantic for user input while keeping wire/manifest form clean. 10 regression tests. Owner-verified. | `[x]` | `phase5/a1` | `5a034dd` |
| tui-key-dispatch-press-only-filter | Bug fix | TUI input task dropped Repeat events: `spawn_input_task` matched only `KeyEventKind::Press`, silently dropping autorepeat (and any other non-Press kind). Now accepts Press + Repeat, only filters Release. Plus a `BLIT_TUI_INPUT_TRACE=1` env-gated diagnostic log to `/tmp/blit-tui-input.log` for follow-up if needed. Owner-authorized. | `[x]` | `phase5/a1` | `2e5bcb9` |
| windows-move-tree-hang | Known issue | `test_remote_move_local_to_remote_directory_tree` hangs on Windows CI (14+ consecutive runs). Other 3 `remote_move` tests pass. Suspected: local source-delete `fs::remove_dir_all` blocked by open file handles from push enumeration (POSIX-vs-Windows unlink semantics). Test gated off Windows with `cfg_attr ignore`; root cause needs interactive Windows debugging. Owner-authorized defer. | `[x]` | `phase5/a1` | `2e5bcb9` |

## Open findings

| ID         | Severity | Title                                                    | Branch |
|------------|----------|----------------------------------------------------------|--------|
| B          | Feature  | `GetState` RPC + `ActiveJobs` table + recent ring        | `phase5/getstate` |
| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  | `phase5/m-jobs` |
| C          | Feature  | `Subscribe` RPC + byte-level instrumentation             | `phase5/c` |
| A.1        | Feature  | TUI implementation                                       |        |
| D          | Feature  | Verify + diagnostics screens                             |        |
| E          | Feature  | Polish (themes, refresh rates, config)                   |        |
| P0-§2.6    | Feature  | Live remote benchmark capture (hardware-bound)           |        |
| audit-1-daemon-timeouts | Robustness | Network operation timeout gaps in delegation path (DNS, gRPC connect, pull_sync_with_spec, subscribe idle) — items 1/2/4 done (audit-1a/1b); item 3 = audit-1c (design pending) | |
| audit-1c-transfer-stall-timeout | Robustness | DESIGN PENDING APPROVAL: no-bytes-30s idle timeout on the delegated pull via an opt-in AsyncRead StallGuard adapter (delegated-only). See finding for approach + open scope question. Prereq for --retry/--wait | |
| audit-2-cli-timeouts | Robustness | Missing connection timeouts on all CLI/admin-verb gRPC connections (~15 sites) | |
| audit-3-panic-resilience | Robustness | SysRng panic in generate_token + 7 mutex poisoning expects in ActiveJobs | |
| audit-4-windows-handle-leak | Bug | Windows HANDLE leak on GetFileInformationByHandle failure in change journal snapshot | |
| audit-5-bridge-robustness | Robustness | Prometheus bridge: one-shot timeout, \r escaping, graceful shutdown, connection limit, write timeout, SO_REUSEADDR | |
| audit-6-test-gaps | Test Gap | Missing test coverage: blit-app (zero tests), TUI rendering, bridge integration, Unicode paths, DNS rebinding, copy fallback | |
| audit-7-code-health | Style | Monolithic files (11K-line main.rs), dead code, 28 AppleDouble artifacts, stale docs, missing Cargo.lock | |
| audit-8-tui-task-leak | Robustness | TUI subscribe forwarder task leak on reconnect; setup task has no connect/RPC timeout | |
| audit-9-cancel-auth | Bug | CancelJob RPC lacks peer authorization — any client can cancel any transfer | |
| audit-10-cancel-completion-race | Bug | Cancel/completion race in delegated_pull: success recorded as "cancelled" under biased select | |
| audit-11-data-plane-underflow | Bug | Buffer underflow in send_file_double_buffered when reader returns excess bytes | |
| audit-12-buffer-pool-leak | Robustness | Semaphore permit leak on OOM panic in BufferPool acquire/try_acquire | |
| audit-13-buffer-pool-double-locking | Performance | Double-locking and redundant memory zeroing in BufferPool | |
| audit-14-resume-copy-redundant-seek | Performance | Redundant seek system calls in sequential block-level resume | |
| audit-15-grpc-missing-connection-timeouts | Robustness | Missing request/idle timeouts on tonic gRPC server control plane | |

## Verified history

Phase 5 A.0 complete. Per-finding audit trails in
`.review/findings/a0-*.md` and `.review/results/a0-*.verified.json`.
Sub-slices on branch `phase5/blit-app-extract`:

- `b5d2414` Crate scaffold + endpoints
- `4800cfc` df / du / find / list-modules / rm
- `009583c` ls (initial)
- `af436b2` ls — LocalListing enum fix
- `39966df` scan
- `d6ee06a` profile
- `334a684` diagnostics (perf + dump)

exec
/usr/bin/zsh -lc 'rtk git show --no-renames --format=fuller --stat 7202c1a' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 7202c1adb978b5da40cfc404e9d09d42a09ec15b
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sat Jul 4 22:53:00 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sat Jul 4 22:53:00 2026 -0400

    sf-1: tripwire + stream-scaling harness
    
    scripts/bench_tripwires.sh re-runs the 2026-07-05 tool-comparison
    baseline against any daemon host in one command (spins blitd + rsyncd
    over ssh, shared data root, fresh targets every run, schema-identical
    CSV) and adds the stream-scaling probe (files/s vs measured stream
    count from the daemon's per-stream completion lines). Tripwire verdict
    is the exit code; blit cells are diffed against the committed baseline.
    No production code (SMALL_FILE_CEILING sf-1, D-2026-07-04-4).
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 .review/findings/sf-1-tripwire-harness.md |  96 ++++++++
 REVIEW.md                                 |  11 +
 scripts/bench_tripwires.sh                | 384 ++++++++++++++++++++++++++++++
 3 files changed, 491 insertions(+)

codex
The active plan’s sf-1 requirements match the user’s checklist: full tool matrix, fresh targets, CSV schema parity, one-command daemon-host run, and a push scaling probe. I’m now checking the script details rather than trusting the finding record’s summary.
exec
/usr/bin/zsh -lc "rtk sed -n '1,260p' docs/bench/10gbe-2026-07-05/DIAGNOSIS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# 10 GbE session — small-file limiter diagnosis (2026-07-04/05)

Durable extract of the measurement basis behind
`docs/plan/SMALL_FILE_CEILING.md` (the raw run dirs live under
gitignored `logs/`; every number the plan relies on is reproduced
here). Rig: skippy (TrueNAS SCALE, 32 cores, ZFS `generic-pool`,
enp66s0f1 @ MTU 9000) ↔ netwatch-01 (Arch, 32 cores, enp6s0 10 GbE @
MTU 9000, tmpfs local ends). Methodology: engine-vs-wire isolation —
async ZFS writes, ARC-warm re-reads, no sync between runs.

## Wire ceiling (iperf3 3.21)

- forward 9.88 Gbit/s single stream / 9.91 ×4 streams; reverse 9.91.
- Reference throughput ceiling ≈ 1.24 GB/s.

## Stream-count evidence (blitd stderr, small vs large push)

Large 1 GiB push — the dial negotiated 8 data-plane connections;
work-stealing gave the single file to one stream:

    stream complete: files=1, bytes=1073741824 (9.69 Gbps)
    (+7× "stream complete: files=0, bytes=0" clean idle teardowns)

Small 10k×4 KiB push — **one** connection accepted, one stream
carried all 10,000 files:

    stream complete: files=10000, bytes=40960000 (0.15 Gbps)
    aggregate 0.15 Gbps (40960000 bytes in 2.14s)   [runs: 2.14/2.26/2.36s]

Per-file arithmetic: 2.14–2.36 s ÷ 10,000 ≈ **215–235 µs/file
sequential** on the daemon (ZFS create+write+set-times per file).
Wire time for the same payload: 40 MiB @ 9.9 Gbit/s ≈ **34 ms** —
the wall is ~65× the wire cost.

Mixed 512 MiB+5k push: single stream, 2.6–3.0 Gbit/s data plane.

## Tripwire + fs-floor evidence (tool_comparison.csv, best of 2)

- rsyncd (native protocol, same ZFS target): 10k push **1.49 s** →
  proves a ≤ ~150 µs/file single-pipe receive floor exists on this
  filesystem; 10k pull 367 ms.
- blit 10k push 2.37 s (variance to 3.31 s), pull 446 ms.
- blit wins all large/mixed-pull/local cells (see CSV); losing cells
  are exactly the per-file-bound ones.
- rclone fairness (cmp_fair*.csv): `--ignore-checksum` local 1 GiB
  1011→227 ms (default hashing dominated); sftp small pull 5.0→2.7 s;
  its native unencrypted server (`serve webdav`) is worse than sftp
  on small files (10k push 315 s, pull 109 s) — sftp is rclone's
  best LAN transport; no rclone config approaches blit or rsync.

## Receive-side CPU (zero-copy revisit gate data, D-2026-06-12-1)

- Pull receiver (client, tmpfs sink): 0.45 cores at 9.5 Gbit/s
  (0.133 u + 0.315 s over 1.019 s wall).
- Push receiver (skippy daemon, ZFS sink): 127 cpu-ticks over 887 ms
  = **1.43 cores** at 9.5 Gbit/s — above the eval doc's "fraction of
  one core" estimate; far from saturation on 32 cores.

## Pull-side ceiling context

Client writes land on tmpfs (µs-class creates), so the 10k-pull wall
(446 ms blit / 367 ms rsyncd) is protocol + client per-file handling,
not storage — the ceiling class for this cell is ≪ 200 ms.

exec
/usr/bin/zsh -lc "rtk bash -lc 'git show 7202c1a:scripts/bench_tripwires.sh | nl -ba | sed -n \"221,460p\"'" in /home/michael/dev/Blit
 succeeded in 0ms:
   221	        done
   222	
   223	        [[ -n "$DAEMON_HOST" ]] || continue
   224	
   225	        log "=== seeding pull source: $workload ==="
   226	        fresh_remote "$SESSION/seed/$workload"
   227	        "$BLIT" copy "$src/" "${BLIT_EP}seed/$workload/" --yes >/dev/null 2>&1 \
   228	            || { echo "seeding $workload over blit failed — is the daemon reachable at $BLIT_EP ?" >&2; exit 1; }
   229	
   230	        log "=== remote cells: $workload ==="
   231	        for run in $(seq 1 "$RUNS"); do
   232	            # push — fresh never-seen remote target every run
   233	            fresh_remote "$SESSION/push/blit_${workload}_r${run}"
   234	            timed_row blit push "$workload" "$run" \
   235	                "$BLIT" copy "$src/" "${BLIT_EP}push/blit_${workload}_r${run}/" --yes
   236	            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 && $RSYNCD_STARTED == 1 ]]; then
   237	                fresh_remote "$SESSION/push/rsyncd_${workload}_r${run}"
   238	                timed_row rsyncd push "$workload" "$run" \
   239	                    rsync -a --whole-file --inplace --no-compress "$src/" "$RSYNCD_URL/push/rsyncd_${workload}_r${run}/"
   240	            fi
   241	            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 ]]; then
   242	                fresh_remote "$SESSION/push/rsync_ssh_${workload}_r${run}"
   243	                timed_row rsync_ssh push "$workload" "$run" \
   244	                    rsync -a --whole-file --inplace --no-compress -e ssh "$src/" "$SSH_HOST:$SESSION/push/rsync_ssh_${workload}_r${run}/"
   245	            fi
   246	            if (( HAVE_RCLONE )); then
   247	                fresh_remote "$SESSION/push/rclone_${workload}_r${run}"
   248	                timed_row rclone_sftp push "$workload" "$run" \
   249	                    rclone copy "$src" ":sftp,host=$SSH_HOST:$SESSION/push/rclone_${workload}_r${run}" \
   250	                        --ignore-checksum --transfers "$RCLONE_TRANSFERS"
   251	            fi
   252	
   253	            # pull — same seeded source for every tool, fresh local target
   254	            dst="$WORK/dst_pull"
   255	            fresh_local "$dst"
   256	            timed_row blit pull "$workload" "$run" "$BLIT" copy "${BLIT_EP}seed/$workload/" "$dst/" --yes
   257	            if [[ $RSYNCD_STARTED == 1 && $HAVE_RSYNC == 1 ]]; then
   258	                fresh_local "$dst"
   259	                timed_row rsyncd pull "$workload" "$run" rsync -a --whole-file --inplace --no-compress "$RSYNCD_URL/seed/$workload/" "$dst/"
   260	            fi
   261	            if [[ "$HAVE_REMOTE_RSYNC" == 1 && $HAVE_RSYNC == 1 ]]; then
   262	                fresh_local "$dst"
   263	                timed_row rsync_ssh pull "$workload" "$run" rsync -a --whole-file --inplace --no-compress -e ssh "$SSH_HOST:$SESSION/seed/$workload/" "$dst/"
   264	            fi
   265	            if (( HAVE_RCLONE )); then
   266	                fresh_local "$dst"
   267	                timed_row rclone_sftp pull "$workload" "$run" \
   268	                    rclone copy ":sftp,host=$SSH_HOST:$SESSION/seed/$workload" "$dst" \
   269	                        --ignore-checksum --transfers "$RCLONE_TRANSFERS"
   270	            fi
   271	        done
   272	    done
   273	}
   274	
   275	# ── stream-scaling probe ─────────────────────────────────────────────
   276	# files/s vs the stream count the transfer ACTUALLY ran with, measured
   277	# from the daemon's per-stream completion lines ("stream complete",
   278	# data_plane.rs) — not from what the proposal table says it should be.
   279	# The plan's acceptance curve: files/s rises with streams until a named
   280	# hardware limiter binds; flattening at a policy-chosen count is the
   281	# sf-2 finding.
   282	run_scale() {
   283	    [[ -n "$DAEMON_HOST" ]] || { log "scale mode needs DAEMON_HOST"; return; }
   284	    echo "files,bytes,ms,files_per_sec,streams,status" > "$SCALE_CSV"
   285	    local count src target before streams start end ms status
   286	    for count in $SCALE_COUNTS; do
   287	        src="$WORK/scale_src_$count"
   288	        log "=== scale probe: $count x ${SMALL_SIZE}B ==="
   289	        gen_small_n "$src" "$count" "$SMALL_SIZE"
   290	        target="$SESSION/push/scale_$count"
   291	        fresh_remote "$target"
   292	        before=0
   293	        [[ -n "$BLITD_LOG" ]] && before=$(rssh "wc -l < '$BLITD_LOG' 2>/dev/null || echo 0")
   294	        status=0
   295	        start=$(date +%s%N)
   296	        timeout "$TIMEOUT_S" "$BLIT" copy "$src/" "${BLIT_EP}push/scale_$count/" --yes >/dev/null 2>&1 || status=$?
   297	        end=$(date +%s%N)
   298	        ms=$(( (end - start) / 1000000 ))
   299	        streams=""
   300	        [[ -n "$BLITD_LOG" ]] && streams=$(rssh "tail -n +$(( before + 1 )) '$BLITD_LOG' 2>/dev/null | grep -c 'stream complete'" || echo "")
   301	        local fps
   302	        fps=$(awk -v c="$count" -v ms="$ms" 'BEGIN { if (ms > 0) printf "%.1f", c * 1000 / ms; else printf "0" }')
   303	        log "  $count files: ${ms}ms  ${fps} files/s  streams=${streams:-?} (status $status)"
   304	        echo "$count,$(( count * SMALL_SIZE )),$ms,$fps,${streams},$status" >> "$SCALE_CSV"
   305	        rm -rf "$src"
   306	    done
   307	    [[ -n "$BLITD_LOG" ]] || log "NOTE: no BLITD_LOG — streams column empty (set it, or use SPIN_DAEMONS=1)"
   308	}
   309	
   310	# ── summary: best-of per cell, tripwire verdict, baseline delta ─────
   311	summarize() {
   312	    log ""
   313	    log "=== TRIPWIRE SUMMARY (best of $RUNS, successful runs only) ==="
   314	    # exit 3 from awk marks "tripped"; anything else from awk is a bug.
   315	    local tripped=0
   316	    awk -F, '
   317	        NR > 1 && $6 == 0 {
   318	            cell = $2 "," $3
   319	            key = $1 SUBSEP cell
   320	            if (!(key in best) || $5 < best[key]) best[key] = $5
   321	            cells[cell] = 1; tools[$1] = 1
   322	        }
   323	        END {
   324	            printf "%-12s %-8s %10s %10s %-12s %s\n", "direction", "workload", "blit_ms", "rival_ms", "rival", "verdict"
   325	            n = 0
   326	            for (cell in cells) {
   327	                if (!(("blit" SUBSEP cell) in best)) continue
   328	                b = best["blit" SUBSEP cell]
   329	                rbest = -1; rname = "-"
   330	                for (t in tools) {
   331	                    if (t == "blit") continue
   332	                    k = t SUBSEP cell
   333	                    if (k in best && (rbest < 0 || best[k] < rbest)) { rbest = best[k]; rname = t }
   334	                }
   335	                split(cell, dw, ",")
   336	                verdict = "clean"
   337	                if (rbest >= 0 && rbest < b) { verdict = "TRIPPED"; n++ }
   338	                printf "%-12s %-8s %10d %10s %-12s %s\n", dw[1], dw[2], b, (rbest < 0 ? "-" : rbest), rname, verdict
   339	            }
   340	            exit (n > 0 ? 3 : 0)
   341	        }' "$MATRIX_CSV" | sort | tee -a "$LOG_DIR/bench.log" || tripped=$?
   342	
   343	    if [[ -f "$BASELINE_CSV" ]]; then
   344	        log ""
   345	        log "=== blit vs committed baseline ($BASELINE_CSV, best-of, +/-10% is run noise) ==="
   346	        awk -F, '
   347	            FNR == 1 { file++; next }
   348	            $1 == "blit" && $6 == 0 {
   349	                key = $2 "," $3
   350	                if (file == 1) { if (!(key in base) || $5 < base[key]) base[key] = $5 }
   351	                else           { if (!(key in now)  || $5 < now[key])  now[key] = $5 }
   352	            }
   353	            END {
   354	                for (key in now) {
   355	                    if (key in base)
   356	                        printf "  blit %-14s %6dms -> %6dms  (%+.1f%%)\n", key, base[key], now[key], (now[key] - base[key]) * 100.0 / base[key]
   357	                    else
   358	                        printf "  blit %-14s (no baseline cell)\n", key
   359	                }
   360	            }' "$BASELINE_CSV" "$MATRIX_CSV" | sort | tee -a "$LOG_DIR/bench.log"
   361	    fi
   362	    return "$tripped"
   363	}
   364	
   365	# ── main ─────────────────────────────────────────────────────────────
   366	log "tripwire harness: mode=$MODE host=${DAEMON_HOST:-<local-only>} runs=$RUNS out=$LOG_DIR"
   367	(( HAVE_RSYNC ))  || log "NOTE: rsync not installed — rsync cells skipped"
   368	(( HAVE_RCLONE )) || log "NOTE: rclone not installed — rclone cells skipped"
   369	
   370	HAVE_REMOTE_RSYNC=0
   371	[[ -n "$DAEMON_HOST" ]] && setup_remote
   372	
   373	RC=0
   374	if [[ "$MODE" == matrix || "$MODE" == all ]]; then
   375	    run_matrix
   376	    summarize || RC=$?
   377	fi
   378	if [[ "$MODE" == scale || "$MODE" == all ]]; then
   379	    run_scale
   380	fi
   381	
   382	log ""
   383	log "results: $LOG_DIR (matrix.csv / scale.csv / bench.log)"
   384	exit "$RC"

exec
/usr/bin/zsh -lc 'rtk git show 7202c1a -- REVIEW.md .review/findings/sf-1-tripwire-harness.md' in /home/michael/dev/Blit
 succeeded in 0ms:
7202c1a sf-1: tripwire + stream-scaling harness (62 seconds ago) <Michael Coelho>
.review/findings/sf-1-tripwire-harness.md | 96 +++++++++++++++++++++++++++++++
 REVIEW.md                                 | 11 ++++
 2 files changed, 107 insertions(+)

.review/findings/sf-1-tripwire-harness.md
  @@ -0,0 +1,96 @@
  +# sf-1 — Tripwire + stream-scaling harness
  +
  +**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-1.
  +**Status**: implemented, codex review pending.
  +
  +## What
  +
  +`scripts/bench_tripwires.sh` — makes the 2026-07-05 tool-comparison
  +baseline (`docs/bench/10gbe-2026-07-05/`) re-runnable against any
  +daemon host in one command, and adds the plan's stream-scaling probe
  +(files/s vs the stream count the transfer actually ran with). No
  +production code.
  +
  +## Approach
  +
  +Derived from `scripts/bench_10gbe.sh` (timing/generation/fresh-target
  +patterns) plus the session's ad-hoc comparison methodology recorded in
  +DEVLOG 2026-07-05 00:51 and DIAGNOSIS.md — the ad-hoc runner itself
  +was never committed, so this reconstructs it durably.
  +
  +- **Matrix** (schema-identical to the committed
  +  `tool_comparison.csv`: `transport,direction,workload,run,ms,status`):
  +  blit / rsyncd / rsync-over-ssh / rclone-sftp × push/pull, and
  +  blit / rsync / rclone / `cp -a` × local, over the baseline's three
  +  workloads (1 GiB large, 10k×4 KiB small, 512 MiB+5k×2 KiB mixed).
  +  rclone runs its best measured LAN config (`--ignore-checksum`,
  +  tuned `--transfers`, sftp transport) per DIAGNOSIS.md. The harness
  +  matrix and the plan's tripwire list are the same set by
  +  construction (plan acceptance criterion).
  +- **One command**: `DAEMON_HOST=… REMOTE_ROOT=… REMOTE_BLIT_DAEMON=…
  +  ./scripts/bench_tripwires.sh`. By default it spins both daemons on
  +  the target host over ssh — blitd via `--root` (exports the
  +  per-invocation session dir as module `default`, no config file
  +  needed) and rsyncd via a generated config — and tears both down plus
  +  the session dir on exit. `SPIN_DAEMONS=0` targets already-running
  +  daemons. All tools share one data root (session methodology).
  +- **Fresh targets every run** (blit and rsync both no-op onto
  +  already-delivered content): local dests recreated, remote push
  +  targets are per-run never-seen subdirs; pull sources seeded once per
  +  workload (seeding writes leave the ARC warm — baseline was warm
  +  re-reads).
  +- **Scale probe**: fixed 4 KiB files at counts crossing
  +  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
  +  25k→8, 50k→10 expected); records files/s and the **measured** stream
  +  count (per-stream `stream complete` completion lines in blitd's
  +  stderr, `data_plane.rs:224`, delta-counted per push). Measured-vs-
  +  table divergence is exactly the sf-2 evidence the plan wants the
  +  curve to show.
  +- **Tripwire verdict is the exit code**: summary prints best-of per
  +  cell, blit vs fastest rival; any rival win → `TRIPPED` + exit 3.
  +  Also diffs blit cells against the committed baseline CSV (the ±10%
  +  regression criterion) when present.
  +- Missing tools (rsync/rclone locally or remotely) skip their cells
  +  with a note; a wedged tool is capped by `timeout` and recorded in
  +  the status column rather than hanging the run.
  +
  +## Files
  +
  +- `scripts/bench_tripwires.sh` (new, executable)
  +
  +## Tests
  +
  +Script-only slice — cargo suite unaffected (run anyway: fmt, clippy,
  +full workspace suite green; count vs 1479 baseline in verdict file).
  +Script verified by execution:
  +
  +- `bash -n` clean.
  +- **Local-only e2e** (`SIZE_MB=32 SMALL_COUNT=500 RUNS=2 … matrix`):
  +  all local cells timed, CSV written, summary + baseline diff printed,
  +  exit 3 with `cp` tripping blit on tiny local copies (harness working
  +  as designed; rig verdicts belong to sf-4).
  +- **Full remote-path e2e over loopback** (ssh shim executing "remote"
  +  commands locally; real network transfers to a real spun blitd +
  +  rsyncd on 127.0.0.1): daemon spin-up, seeding, every push/pull cell,
  +  scale probe with stream counting (200 files → 1 stream measured),
  +  teardown verified (no stray daemons, session dir removed).
  +  rclone-sftp cells recorded status 1 in this rig-less test (no sftp
  +  auth to localhost) — the status column captured it and the run
  +  continued, which is the designed failure path.
  +
  +## Known gaps
  +
  +- The scale probe is push-only (the plan's target cell); pull scaling
  +  can be added when a pull-side per-stream log line exists.
  +- Stream counting needs the daemon's stderr (`SPIN_DAEMONS=1` owns it;
  +  otherwise `BLITD_LOG`); against a foreign daemon the column is empty
  +  rather than guessed.
  +- rclone-sftp cells assume ssh-agent/key auth to the host (same
  +  requirement the session had); no rclone config file is generated.
  +- Loopback e2e cannot validate 10 GbE-scale numbers — sf-4 is the rig
  +  re-measure slice.
  +- Observed during loopback testing, recorded for sf-2: a 1000-file
  +  push rode 1 stream where the proposal table says 2 — consistent
  +  with the DIAGNOSIS.md one-stream-for-10k-files gap; the daemon-side
  +  proposal call (`control.rs:798`) and its input manifest need the
  +  sf-2 pins.
  +96 -0

REVIEW.md
  @@ -35,6 +35,17 @@ accepted findings fixed, validation green. Records per slice:
  +## Small-file ceiling (SMALL_FILE_CEILING) — code→GPT-review→fix loop
  +
  +Plan: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4).
  +Same codex loop and record formats as the REV4 section above. Slice
  +order and gates live in the plan (sf-6 is owner-gated on wire design;
  +sf-3b… count is set by sf-3a's analysis, rows added as filed).
  +
  +| ID | Title | Status | Commit(s) |
  +|----|-------|--------|-----------|
  +| sf-1 | Tripwire + stream-scaling harness (`scripts/bench_tripwires.sh`) — baseline re-runnable in one command | `[~]` | |
  +
   ## Design-review queue (ratified D-2026-06-11-2, in execution order)
   
   Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
  +11 -0

exec
/usr/bin/zsh -lc "rtk bash -lc 'git show 7202c1a:scripts/bench_tripwires.sh | nl -ba | sed -n \"1,220p\"'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	#!/usr/bin/env bash
     2	# Tripwire + stream-scaling harness (SMALL_FILE_CEILING sf-1).
     3	#
     4	# Re-runs the 2026-07-05 tool-comparison matrix against any daemon host
     5	# in one command, plus a stream-scaling probe (files/s vs negotiated
     6	# stream count). Derived from scripts/bench_10gbe.sh and the ad-hoc
     7	# session runner behind docs/bench/10gbe-2026-07-05/tool_comparison.csv
     8	# (same CSV schema, so runs are directly comparable to the committed
     9	# baseline).
    10	#
    11	# Tripwire semantics (docs/plan/SMALL_FILE_CEILING.md, Principle):
    12	# the tools here are NOT targets — any cell where any tool measures
    13	# faster than blit is proof blit is off its hardware ceiling and is a
    14	# finding to fix. The harness matrix and the plan's tripwire list are
    15	# the same set by construction.
    16	#
    17	# Usage (one command against a daemon host):
    18	#   DAEMON_HOST=skippy \
    19	#   REMOTE_ROOT=/mnt/generic-pool/video/blit-bin/bench-data \
    20	#   REMOTE_BLIT_DAEMON=/mnt/generic-pool/video/blit-bin/blit-daemon \
    21	#   ./scripts/bench_tripwires.sh [matrix|scale|all]     # default: all
    22	#
    23	#   Local-only tripwires (no DAEMON_HOST): blit vs rsync/rclone/cp on
    24	#   this machine's ${TMPDIR:-/tmp}.
    25	#
    26	# Environment:
    27	#   DAEMON_HOST        network + ssh name of the daemon host (remote cells)
    28	#   SSH_HOST           ssh alias if it differs from DAEMON_HOST
    29	#   REMOTE_ROOT        writable dir on the daemon host; a per-invocation
    30	#                      session dir is created (and removed) under it.
    31	#                      NOTE: must be exec-friendly for SPIN_DAEMONS — on
    32	#                      TrueNAS /tmp and /home are noexec (session lesson).
    33	#   REMOTE_BLIT_DAEMON path to blit-daemon ON the daemon host
    34	#   SPIN_DAEMONS=1     spin blitd (--root, module "default") + rsyncd on
    35	#                      the daemon host over ssh; 0 = daemons already run
    36	#                      (then set BLIT_PORT/BLIT_MODULE/RSYNCD_PORT and
    37	#                      optionally BLITD_LOG for scale-mode stream counts)
    38	#   BLIT_PORT=9031  BLIT_MODULE=default  RSYNCD_PORT=8730
    39	#   BLITD_LOG          remote path of blitd's stderr log (scale mode
    40	#                      stream counting when SPIN_DAEMONS=0)
    41	#   RUNS=2             timed runs per cell (baseline was best-of-2)
    42	#   TIMEOUT_S=600      per-run cap (a wedged tool records status 124)
    43	#   RCLONE_TRANSFERS=16  rclone best-config concurrency (fairness flags
    44	#                      --ignore-checksum + tuned --transfers per
    45	#                      docs/bench/10gbe-2026-07-05/DIAGNOSIS.md)
    46	#   SIZE_MB=1024 SMALL_COUNT=10000 SMALL_SIZE=4096   workload knobs
    47	#   SCALE_COUNTS="200 1000 5000 10000 25000 50000"   probe file counts
    48	#                      (chosen to cross engine::initial_stream_proposal
    49	#                      tiers: expected proposals 1/2/4/8/8/10)
    50	#   BASELINE_CSV       committed baseline to diff blit cells against
    51	#                      (default docs/bench/10gbe-2026-07-05/tool_comparison.csv)
    52	#
    53	# Requirements: ssh key access to the host (rsync-over-ssh and
    54	# rclone-sftp cells deliberately pay the cipher tax — that is their
    55	# datapoint); rsync on both ends; rclone on the client. Missing tools
    56	# skip their cells with a note instead of failing the run.
    57	#
    58	# Methodology (matches the committed baseline): local ends on
    59	# ${TMPDIR:-/tmp} (tmpfs on the rig), fresh never-seen target dirs for
    60	# EVERY timed run (blit and rsync both no-op onto delivered content),
    61	# pull sources seeded once per workload (write path leaves ZFS ARC
    62	# warm, so pulls are warm re-reads), async writes, no sync between
    63	# runs, wall-clock ms.
    64	#
    65	# Exit codes: 0 = ran and no tripwire tripped; 3 = at least one tool
    66	# beat blit somewhere (the summary names the cells); 1 = harness error.
    67	
    68	set -euo pipefail
    69	
    70	MODE=${1:-all}
    71	case "$MODE" in matrix|scale|all) ;; *) echo "usage: $0 [matrix|scale|all]" >&2; exit 1;; esac
    72	
    73	SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
    74	REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
    75	BLIT=${BLIT:-"$REPO_ROOT/target/release/blit"}
    76	
    77	DAEMON_HOST=${DAEMON_HOST:-}
    78	SSH_HOST=${SSH_HOST:-$DAEMON_HOST}
    79	REMOTE_ROOT=${REMOTE_ROOT:-}
    80	REMOTE_BLIT_DAEMON=${REMOTE_BLIT_DAEMON:-}
    81	SPIN_DAEMONS=${SPIN_DAEMONS:-1}
    82	BLIT_PORT=${BLIT_PORT:-9031}
    83	BLIT_MODULE=${BLIT_MODULE:-default}
    84	RSYNCD_PORT=${RSYNCD_PORT:-8730}
    85	BLITD_LOG=${BLITD_LOG:-}
    86	RUNS=${RUNS:-2}
    87	TIMEOUT_S=${TIMEOUT_S:-600}
    88	RCLONE_TRANSFERS=${RCLONE_TRANSFERS:-16}
    89	SIZE_MB=${SIZE_MB:-1024}
    90	SMALL_COUNT=${SMALL_COUNT:-10000}
    91	SMALL_SIZE=${SMALL_SIZE:-4096}
    92	SCALE_COUNTS=${SCALE_COUNTS:-"200 1000 5000 10000 25000 50000"}
    93	BASELINE_CSV=${BASELINE_CSV:-"$REPO_ROOT/docs/bench/10gbe-2026-07-05/tool_comparison.csv"}
    94	
    95	[[ -x "$BLIT" ]] || { echo "blit binary not found at $BLIT (build with cargo build --release or set BLIT=)" >&2; exit 1; }
    96	
    97	WORK=$(mktemp -d "${TMPDIR:-/tmp}/blit_tripwires.XXXXXX")
    98	STAMP=$(date +%Y%m%dT%H%M%S)
    99	LOG_DIR="$REPO_ROOT/logs/tripwires_$STAMP"
   100	mkdir -p "$LOG_DIR"
   101	MATRIX_CSV="$LOG_DIR/matrix.csv"
   102	SCALE_CSV="$LOG_DIR/scale.csv"
   103	
   104	HAVE_RSYNC=1; command -v rsync >/dev/null || HAVE_RSYNC=0
   105	HAVE_RCLONE=1; command -v rclone >/dev/null || HAVE_RCLONE=0
   106	
   107	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$LOG_DIR/bench.log"; }
   108	
   109	rssh() { ssh -o BatchMode=yes "$SSH_HOST" "$@"; }
   110	
   111	# ── remote session lifecycle ─────────────────────────────────────────
   112	SESSION=""            # per-invocation dir under REMOTE_ROOT
   113	BLITD_STARTED=0
   114	RSYNCD_STARTED=0
   115	
   116	teardown() {
   117	    rm -rf "$WORK"
   118	    if [[ -n "$SESSION" ]]; then
   119	        if (( BLITD_STARTED )); then
   120	            rssh "kill \$(cat '$SESSION/blitd.pid') 2>/dev/null" || true
   121	        fi
   122	        if (( RSYNCD_STARTED )); then
   123	            rssh "kill \$(cat '$SESSION/rsyncd.pid') 2>/dev/null" || true
   124	        fi
   125	        # Only ever the directory this invocation created.
   126	        rssh "rm -rf '$SESSION'" || true
   127	    fi
   128	}
   129	trap teardown EXIT
   130	
   131	setup_remote() {
   132	    [[ -n "$REMOTE_ROOT" ]] || { echo "DAEMON_HOST set but REMOTE_ROOT is empty" >&2; exit 1; }
   133	    rssh "true" || { echo "cannot ssh to $SSH_HOST" >&2; exit 1; }
   134	    SESSION="$REMOTE_ROOT/tripwires_$STAMP"
   135	    rssh "mkdir -p '$SESSION/push' '$SESSION/seed'"
   136	
   137	    HAVE_REMOTE_RSYNC=$(rssh "command -v rsync >/dev/null && echo 1 || echo 0")
   138	
   139	    if (( SPIN_DAEMONS )); then
   140	        [[ -n "$REMOTE_BLIT_DAEMON" ]] || { echo "SPIN_DAEMONS=1 needs REMOTE_BLIT_DAEMON" >&2; exit 1; }
   141	        log "spinning blit-daemon on $SSH_HOST (--root $SESSION, port $BLIT_PORT)"
   142	        rssh "nohup '$REMOTE_BLIT_DAEMON' --root '$SESSION' --port $BLIT_PORT --no-mdns \
   143	                  > '$SESSION/blitd.log' 2>&1 & echo \$! > '$SESSION/blitd.pid'"
   144	        BLITD_STARTED=1
   145	        BLIT_MODULE=default
   146	        BLITD_LOG="$SESSION/blitd.log"
   147	        if [[ "$HAVE_REMOTE_RSYNC" == 1 ]]; then
   148	            log "spinning rsyncd on $SSH_HOST (port $RSYNCD_PORT, module bench -> $SESSION)"
   149	            rssh "printf 'port = %s\npid file = %s/rsyncd.pid\nuse chroot = false\n[bench]\n  path = %s\n  read only = false\n' \
   150	                      '$RSYNCD_PORT' '$SESSION' '$SESSION' > '$SESSION/rsyncd.conf' && \
   151	                  rsync --daemon --config='$SESSION/rsyncd.conf'"
   152	            RSYNCD_STARTED=1
   153	        else
   154	            log "NOTE: rsync missing on $SSH_HOST — rsyncd + rsync_ssh cells skipped"
   155	        fi
   156	        sleep 1   # both daemons bind before the first cell
   157	    fi
   158	    BLIT_EP="$DAEMON_HOST:$BLIT_PORT:/$BLIT_MODULE/"    # trailing slash is load-bearing (endpoint.rs)
   159	    RSYNCD_URL="rsync://$DAEMON_HOST:$RSYNCD_PORT/bench"
   160	}
   161	
   162	# ── timing ───────────────────────────────────────────────────────────
   163	# One timed run: records transport,direction,workload,run,ms,status
   164	# (identical schema to the committed baseline CSV). Never aborts the
   165	# harness on tool failure — the status column carries it.
   166	timed_row() {
   167	    local transport="$1" direction="$2" workload="$3" run="$4"; shift 4
   168	    local start end ms status=0
   169	    start=$(date +%s%N)
   170	    timeout "$TIMEOUT_S" "$@" >/dev/null 2>&1 || status=$?
   171	    end=$(date +%s%N)
   172	    ms=$(( (end - start) / 1000000 ))
   173	    log "  $transport $direction $workload r$run: ${ms}ms (status $status)"
   174	    echo "$transport,$direction,$workload,$run,$ms,$status" >> "$MATRIX_CSV"
   175	}
   176	
   177	fresh_local() { rm -rf "$1"; mkdir -p "$1"; }
   178	fresh_remote() { rssh "rm -rf '$1' && mkdir -p '$1'"; }
   179	
   180	# ── workload generation (same shapes as the baseline) ───────────────
   181	gen_large() { mkdir -p "$1"; dd if=/dev/urandom of="$1/large_${SIZE_MB}M.bin" bs=1M count="$SIZE_MB" 2>/dev/null; }
   182	gen_small_n() { # $1=dir $2=count $3=size
   183	    local dir="$1" count="$2" size="$3" i sub
   184	    mkdir -p "$dir"
   185	    for i in $(seq 1 "$count"); do
   186	        sub="$dir/d$(( i / 1000 ))"
   187	        mkdir -p "$sub"
   188	        dd if=/dev/urandom of="$sub/f${i}.dat" bs="$size" count=1 2>/dev/null
   189	    done
   190	}
   191	gen_mixed() {
   192	    mkdir -p "$1"
   193	    dd if=/dev/urandom of="$1/big.bin" bs=1M count=512 2>/dev/null
   194	    gen_small_n "$1/smalls" 5000 2048
   195	}
   196	
   197	# ── the matrix ───────────────────────────────────────────────────────
   198	run_matrix() {
   199	    echo "transport,direction,workload,run,ms,status" > "$MATRIX_CSV"
   200	
   201	    log "=== generating workloads (large ${SIZE_MB}M / small ${SMALL_COUNT}x${SMALL_SIZE}B / mixed 512M+5000x2K) ==="
   202	    gen_large "$WORK/src_large"
   203	    gen_small_n "$WORK/src_small" "$SMALL_COUNT" "$SMALL_SIZE"
   204	    gen_mixed "$WORK/src_mixed"
   205	
   206	    local workload src run dst
   207	    for workload in large small mixed; do
   208	        src="$WORK/src_$workload"
   209	
   210	        log "=== local cells: $workload ==="
   211	        for run in $(seq 1 "$RUNS"); do
   212	            dst="$WORK/dst_local"
   213	            fresh_local "$dst"; timed_row blit local "$workload" "$run" "$BLIT" copy "$src/" "$dst/" --yes
   214	            if (( HAVE_RSYNC )); then
   215	                fresh_local "$dst"; timed_row rsync local "$workload" "$run" rsync -a --whole-file --inplace --no-compress "$src/" "$dst/"
   216	            fi
   217	            if (( HAVE_RCLONE )); then
   218	                fresh_local "$dst"; timed_row rclone local "$workload" "$run" rclone copy "$src" "$dst" --ignore-checksum --transfers "$RCLONE_TRANSFERS"
   219	            fi
   220	            fresh_local "$dst"; timed_row cp local "$workload" "$run" cp -a "$src/." "$dst/"

exec
/usr/bin/zsh -lc "rtk bash -lc 'printf \"schema: \"; head -n 1 docs/bench/10gbe-2026-07-05/tool_comparison.csv; sed -n \"1,20p\" docs/bench/10gbe-2026-07-05/tool_comparison.csv'" in /home/michael/dev/Blit
 succeeded in 0ms:
schema: transport,direction,workload,run,ms,status
transport,direction,workload,run,ms,status
blit,push,large,1,955,0
rsyncd,push,large,1,7232,0
rsync_ssh,push,large,1,2251,0
rclone_sftp,push,large,1,3128,0
blit,pull,large,1,890,0
rsyncd,pull,large,1,1029,0
rsync_ssh,pull,large,1,2200,0
rclone_sftp,pull,large,1,2851,0
blit,push,large,2,949,0
rsyncd,push,large,2,1010,0
rsync_ssh,push,large,2,2213,0
rclone_sftp,push,large,2,3088,0
blit,pull,large,2,884,0
rsyncd,pull,large,2,1036,0
rsync_ssh,pull,large,2,1861,0
rclone_sftp,pull,large,2,2884,0
blit,push,small,1,2373,0
rsyncd,push,small,1,1493,0
rsync_ssh,push,small,1,1641,0

codex
The script is compact enough to audit directly. A few areas need deeper checks now: remote quoting/safety around `ssh`, whether skipped rsyncd cells weaken the required full matrix, and whether the stream-count grep is actually measuring the intended transfer.
exec
/usr/bin/zsh -lc "rtk sed -n '1,120p' docs/bench/10gbe-2026-07-05/tool_comparison.csv" in /home/michael/dev/Blit
 succeeded in 0ms:
transport,direction,workload,run,ms,status
blit,push,large,1,955,0
rsyncd,push,large,1,7232,0
rsync_ssh,push,large,1,2251,0
rclone_sftp,push,large,1,3128,0
blit,pull,large,1,890,0
rsyncd,pull,large,1,1029,0
rsync_ssh,pull,large,1,2200,0
rclone_sftp,pull,large,1,2851,0
blit,push,large,2,949,0
rsyncd,push,large,2,1010,0
rsync_ssh,push,large,2,2213,0
rclone_sftp,push,large,2,3088,0
blit,pull,large,2,884,0
rsyncd,pull,large,2,1036,0
rsync_ssh,pull,large,2,1861,0
rclone_sftp,pull,large,2,2884,0
blit,push,small,1,2373,0
rsyncd,push,small,1,1493,0
rsync_ssh,push,small,1,1641,0
rclone_sftp,push,small,1,5848,0
blit,pull,small,1,484,0
rsyncd,pull,small,1,367,0
rsync_ssh,pull,small,1,435,0
rclone_sftp,pull,small,1,5022,0
blit,push,small,2,3313,0
rsyncd,push,small,2,1513,0
rsync_ssh,push,small,2,1624,0
rclone_sftp,push,small,2,6012,0
blit,pull,small,2,446,0
rsyncd,pull,small,2,374,0
rsync_ssh,pull,small,2,425,0
rclone_sftp,pull,small,2,5010,0
blit,push,mixed,1,1829,0
rsyncd,push,mixed,1,1248,0
rsync_ssh,push,mixed,1,1966,0
rclone_sftp,push,mixed,1,3883,0
blit,pull,mixed,1,577,0
rsyncd,pull,mixed,1,686,0
rsync_ssh,pull,mixed,1,1311,0
rclone_sftp,pull,mixed,1,3863,0
blit,push,mixed,2,2222,0
rsyncd,push,mixed,2,1239,0
rsync_ssh,push,mixed,2,1978,0
rclone_sftp,push,mixed,2,3966,0
blit,pull,mixed,2,586,0
rsyncd,pull,mixed,2,693,0
rsync_ssh,pull,mixed,2,1290,0
rclone_sftp,pull,mixed,2,3946,0
blit,local,large,1,173,0
rsync,local,large,1,332,0
rclone,local,large,1,1011,0
blit,local,small,1,79,0
rsync,local,small,1,87,0
rclone,local,small,1,199,0
blit,local,mixed,1,119,0
rsync,local,mixed,1,248,0
rclone,local,mixed,1,557,0

exec
/usr/bin/zsh -lc 'rtk rg -n "stream complete|stream_count|initial_stream_proposal|data_plane.rs" crates scripts docs/bench/10gbe-2026-07-05/DIAGNOSIS.md' in /home/michael/dev/Blit
 succeeded in 0ms:
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:21:    stream complete: files=1, bytes=1073741824 (9.69 Gbps)
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:22:    (+7× "stream complete: files=0, bytes=0" clean idle teardowns)
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:27:    stream complete: files=10000, bytes=40960000 (0.15 Gbps)
scripts/bench_tripwires.sh:48:#                      (chosen to cross engine::initial_stream_proposal
scripts/bench_tripwires.sh:277:# from the daemon's per-stream completion lines ("stream complete",
scripts/bench_tripwires.sh:278:# data_plane.rs) — not from what the proposal table says it should be.
scripts/bench_tripwires.sh:300:        [[ -n "$BLITD_LOG" ]] && streams=$(rssh "tail -n +$(( before + 1 )) '$BLITD_LOG' 2>/dev/null | grep -c 'stream complete'" || echo "")
crates/blit-daemon/src/service/pull_sync.rs:16:use blit_core::engine::{initial_stream_proposal, TransferDial};
crates/blit-daemon/src/service/pull_sync.rs:344:        let stream_count = negotiated_pull_streams(
crates/blit-daemon/src/service/pull_sync.rs:364:            stream_count,
crates/blit-daemon/src/service/pull_sync.rs:416:/// today's negotiated stream_count"). The resume path does not call
crates/blit-daemon/src/service/pull_sync.rs:437:    let proposal = initial_stream_proposal(total_bytes, file_count, dial.ceiling_max_streams());
crates/blit-daemon/src/service/pull_sync.rs:655:    stream_count: u32,
crates/blit-daemon/src/service/pull_sync.rs:683:    // Send negotiation. ue-r2-1g: stream_count is the engine's
crates/blit-daemon/src/service/pull_sync.rs:692:                stream_count,
crates/blit-daemon/src/service/pull_sync.rs:723:    let streams = stream_count.max(1) as usize;
crates/blit-daemon/src/service/pull_sync.rs:859:                                target_stream_count: p.target_streams as u32,
crates/blit-daemon/src/service/pull_sync.rs:1268:                stream_count: 1, // Single stream for resume mode
crates/blit-daemon/src/service/pull_sync.rs:1989:        // stream_count" — 1 on pull_sync.
crates/blit-daemon/src/service/delegated_pull.rs:342:                stream_count: 0,
crates/blit-cli/src/transfers/remote_remote_direct.rs:200:                started.source_data_plane_endpoint, started.stream_count
crates/blit-daemon/src/service/push/data_plane.rs:71:    stream_count: u32,
crates/blit-daemon/src/service/push/data_plane.rs:74:    let streams = stream_count.max(1) as usize;
crates/blit-daemon/src/service/push/data_plane.rs:224:        "blitd: push data plane: stream complete: files={}, bytes={} ({:.2} Gbps)",
crates/blit-daemon/src/service/push/data_plane.rs:279:    stream_count: u32,
crates/blit-daemon/src/service/push/data_plane.rs:283:    let streams = stream_count.max(1) as usize;
crates/blit-daemon/src/service/push/data_plane.rs:841:            stream_count: 0,
crates/blit-daemon/src/service/push/control.rs:310:                                        stream_count: stream_target,
crates/blit-daemon/src/service/push/control.rs:432:                    stream_count: stream_target,
crates/blit-daemon/src/service/push/control.rs:564:    let within_ceiling = req.target_stream_count <= ceiling && *resize_live < ceiling;
crates/blit-daemon/src/service/push/control.rs:591:            req.target_stream_count
crates/blit-daemon/src/service/push/control.rs:598:            effective_stream_count: req.target_stream_count,
crates/blit-daemon/src/service/push/control.rs:800:    blit_core::engine::initial_stream_proposal(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:776:            stream_count: 1,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:934:            target_stream_count: 2,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1035:            stream_count: 1,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1043:            target_stream_count: 2,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1072:    assert_eq!(acks[0].effective_stream_count, 2);
crates/blit-core/tests/proto_wire_compat.rs:56:    stream_count: u32,
crates/blit-core/tests/proto_wire_compat.rs:231:        stream_count: 4,
crates/blit-core/tests/proto_wire_compat.rs:238:    assert_eq!(new.stream_count, 4);
crates/blit-core/tests/proto_wire_compat.rs:306:        stream_count: 4,
crates/blit-core/tests/proto_wire_compat.rs:316:    assert_eq!(old.stream_count, 4);
crates/blit-core/tests/proto_wire_compat.rs:375:                target_stream_count: 5,
crates/blit-core/tests/proto_wire_compat.rs:399:                target_stream_count: 3,
crates/blit-core/tests/proto_wire_compat.rs:427:                effective_stream_count: 5,
crates/blit-core/tests/proto_wire_compat.rs:441:                effective_stream_count: 2,
crates/blit-core/tests/proto_wire_compat.rs:466:        target_stream_count: 9,
crates/blit-core/tests/proto_wire_compat.rs:476:        target_stream_count: 4,
crates/blit-core/tests/proto_wire_compat.rs:484:        effective_stream_count: 9,
crates/blit-core/tests/proto_wire_compat.rs:497:        stream_count: 2,
crates/blit-core/src/buffer.rs:166:/// let pool = Arc::new(BufferPool::for_data_plane(dial.chunk_bytes(), stream_count));
crates/blit-core/src/remote/pull.rs:312:        let stream_count = bounded_stream_count(negotiation.stream_count);
crates/blit-core/src/remote/pull.rs:335:                stream_count,
crates/blit-core/src/remote/pull.rs:863:                    data_plane_live = bounded_stream_count(neg.stream_count);
crates/blit-core/src/remote/pull.rs:1027:                    let within_ceiling = bounded_stream_count(cmd.target_stream_count)
crates/blit-core/src/remote/pull.rs:1028:                        == cmd.target_stream_count.max(1) as usize;
crates/blit-core/src/remote/pull.rs:1063:                            cmd.target_stream_count
crates/blit-core/src/remote/pull.rs:1070:                                effective_stream_count: cmd.target_stream_count,
crates/blit-core/src/remote/pull.rs:1657:fn bounded_stream_count(negotiated: u32) -> usize {
crates/blit-core/src/remote/pull.rs:1676:    stream_count: usize,
crates/blit-core/src/remote/pull.rs:1692:    if stream_count <= 1 && resize.is_none() {
crates/blit-core/src/remote/pull.rs:1795:    for _ in 0..stream_count.max(1) {
crates/blit-core/src/remote/pull.rs:2035:    //! daemon now drives with `stream_count > 1`. Fail-whole is the
crates/blit-core/src/remote/pull.rs:2152:    fn stream_count_is_bounded_by_the_local_receiver_ceiling() {
crates/blit-core/src/remote/pull.rs:2155:        // squatter) advertising a huge stream_count must not drive an
crates/blit-core/src/remote/pull.rs:2157:        use super::bounded_stream_count;
crates/blit-core/src/remote/pull.rs:2160:        assert_eq!(bounded_stream_count(0), 1, "floor");
crates/blit-core/src/remote/pull.rs:2161:        assert_eq!(bounded_stream_count(1), 1);
crates/blit-core/src/remote/pull.rs:2162:        assert_eq!(bounded_stream_count(16), 16.min(ceiling));
crates/blit-core/src/remote/pull.rs:2163:        assert_eq!(bounded_stream_count(u32::MAX), ceiling, "ceiling");
crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
crates/blit-core/src/engine/dial.rs:429:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
crates/blit-core/src/engine/dial.rs:642:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
crates/blit-core/src/engine/dial.rs:646:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
crates/blit-core/src/engine/dial.rs:650:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
crates/blit-core/src/engine/dial.rs:651:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:652:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:653:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:654:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:655:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:656:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:657:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:658:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:659:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:660:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:661:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
crates/blit-core/src/engine/dial.rs:663:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
crates/blit-core/src/engine/dial.rs:664:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
crates/blit-core/src/engine/dial.rs:665:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
crates/blit-core/src/engine/dial.rs:666:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
crates/blit-core/src/engine/dial.rs:667:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
crates/blit-core/src/engine/dial.rs:668:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
crates/blit-core/src/engine/dial.rs:670:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
crates/blit-core/src/engine/dial.rs:671:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
crates/blit-core/src/remote/transfer/tar_safety.rs:9://!   - `crates/blit-daemon/src/service/push/data_plane.rs::apply_tar_shard_sync`
crates/blit-core/src/remote/transfer/tar_safety.rs:237:    // fallback (`service/push/data_plane.rs`), so silencing the
crates/blit-core/src/remote/transfer/grpc_fallback.rs:66://! - `crates/blit-daemon/src/service/push/data_plane.rs:347` —
crates/blit-core/src/remote/push/client/mod.rs:162:        stream_count: usize,
crates/blit-core/src/remote/push/client/mod.rs:175:        let streams = stream_count.max(1);
crates/blit-core/src/remote/push/client/mod.rs:896:                                                neg.stream_count.max(1) as usize,
crates/blit-core/src/remote/push/client/mod.rs:1236:                                                target_stream_count: p.target_streams as u32,
crates/blit-core/src/remote/push/client/mod.rs:1287:                                            target_stream_count: p.target_streams as u32,

exec
/usr/bin/zsh -lc 'rtk rg -n "rclone|ignore-checksum|transfers|webdav|sftp|rsyncd|tool_comparison" docs/bench/10gbe-2026-07-05 docs/plan/SMALL_FILE_CEILING.md DEVLOG.md' in /home/michael/dev/Blit
 succeeded in 0ms:
94 matches in 6 files:

DEVLOG.md:8:**2026-07-05 02:07:15Z** - **PLAN + PRINCIPLE (SMALL_FILE_CEILING draft, clau...
DEVLOG.md:10:**2026-07-05 00:51:18Z** - **BENCHMARK ADDENDUM (tool comparison + zero-copy ...
DEVLOG.md:32:**2026-07-04 13:53:22Z** - **DECISIONS (owner Q&A, claude)**: The owner asked...
DEVLOG.md:80:**2026-06-11 02:40:00Z** - **AUDIT**: Phase A bug-class candidates dispositio...
DEVLOG.md:82:**2026-06-11 02:05:00Z** - **AUDIT**: Design-coherence review Phase A execute...
DEVLOG.md:84:**2026-06-11 00:42:30Z** - **REVIEW**: Verification and acceptance of `audit-...
DEVLOG.md:96:**2026-05-14 12:00:00Z** - **FIX**: post-6e750b9 review followups against the...
DEVLOG.md:98:**2026-05-13 18:30:00Z** - **FEAT + SCOPE**: §3.1 / §3.3 / §5.2 / §5.4 closed...
DEVLOG.md:142:**2026-05-03 00:30:00Z** - **ACTION**: Implemented Phase 2 of `docs/plan/REMO...
DEVLOG.md:148:**2026-05-02 22:15:00Z** - **ACTION**: Closed Round 15 finding R15-F1 (Low) —...
DEVLOG.md:158:**2026-05-02 17:30:00Z** - **ACTION**: Closed F7 + F8 from `docs/reviews/code...
DEVLOG.md:194:**2026-04-24 02:54:00Z** - **ACTION**: Completed the remaining UX feedback it...
DEVLOG.md:196:**2026-04-24 02:32:00Z** - **ACTION**: Addressed UX feedback items #1 and #3 ...
DEVLOG.md:216:**2026-03-06** - **FIX**: Fixed remote-to-remote transfers delivering zero-co...
DEVLOG.md:226:**2025-01-28 14:00:00Z** - **ACTION**: Changed `resume_copy_file` to use Blak...
DEVLOG.md:228:**2025-01-28 13:00:00Z** - **ACTION**: Integrated resumable transfers into th...
DEVLOG.md:230:**2025-01-28 12:00:00Z** - **ACTION**: Implemented resumable file copy with b...
DEVLOG.md:232:**2025-01-28 11:00:00Z** - **ACTION**: Reverted temp file approach for atomic...
DEVLOG.md:234:**2025-01-28 10:00:00Z** - **ACTION**: Added buffer pool for daemon tar shard...
DEVLOG.md:242:**2025-01-26 12:30:00Z** - **ACTION**: Verified parallel payload dispatch alr...
DEVLOG.md:248:**2025-01-26 10:00:00Z** - **ACTION**: Implemented buffer pool for reusable a...
DEVLOG.md:252:**2025-01-21 12:30:00Z** - **ACTION**: Documented security requirements for r...
DEVLOG.md:254:**2025-01-21 12:00:00Z** - **ACTION**: Implemented destructive operation conf...
DEVLOG.md:284:**2025-10-27 23:35:00Z** - **ACTION**: Refactored `blit-cli` entrypoint into ...
DEVLOG.md:302:**2025-10-26 21:56:50Z** - **ACTION**: Reworked remote push streaming to hono...
  +3 more in DEVLOG.md
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:37:## Tripwire + fs-floor evidence (tool_comparison.csv, best of 2)
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:39:- rsyncd (native protocol, same ZFS target): 10k push **1.49 s** →
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:45:- rclone fairness (cmp_fair*.csv): `--ignore-checksum` local 1 GiB
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:46:1011→227 ms (default hashing dominated); sftp small pull 5.0→2.7 s;
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:47:its native unencrypted server (`serve webdav`) is worse than sftp
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:48:on small files (10k push 315 s, pull 109 s) — sftp is rclone's
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:49:best LAN transport; no rclone config approaches blit or rsync.
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md:62:(446 ms blit / 367 ms rsyncd) is protocol + client per-file handling,
docs/bench/10gbe-2026-07-05/cmp_fair.csv:2:rclone_webdav,push,large,1,20,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:3:rclone_webdav,pull,large,1,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:4:rclone_webdav,push,large,2,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:5:rclone_webdav,pull,large,2,17,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:6:rclone_webdav,push,small,1,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:7:rclone_webdav,pull,small,1,17,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:8:rclone_webdav,push,small,2,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:9:rclone_webdav,pull,small,2,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:10:rclone_webdav,push,mixed,1,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:11:rclone_webdav,pull,mixed,1,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:12:rclone_webdav,push,mixed,2,18,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:13:rclone_webdav,pull,mixed,2,17,1
docs/bench/10gbe-2026-07-05/cmp_fair.csv:14:rclone_sftp_nohash,push,small,1,5218,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:15:rclone_sftp_nohash,pull,small,1,2742,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:16:rclone_local_nohash,local,large,1,227,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:17:rclone_local_nohash,local,small,1,136,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:18:rclone_local_nohash,local,mixed,1,127,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:2:rclone_webdav,push,large,1,3326,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:3:rclone_webdav,pull,large,1,1011,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:4:rclone_webdav,push,large,2,3365,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:5:rclone_webdav,pull,large,2,1004,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:6:rclone_webdav,push,small,1,314978,0
docs/bench/10gbe-2026-07-05/cmp_fair2.csv:7:rclone_webdav,pull,small,1,108708,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:3:rsyncd,push,large,1,7232,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:5:rclone_sftp,push,large,1,3128,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:7:rsyncd,pull,large,1,1029,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:9:rclone_sftp,pull,large,1,2851,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:11:rsyncd,push,large,2,1010,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:13:rclone_sftp,push,large,2,3088,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:15:rsyncd,pull,large,2,1036,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:17:rclone_sftp,pull,large,2,2884,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:19:rsyncd,push,small,1,1493,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:21:rclone_sftp,push,small,1,5848,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:23:rsyncd,pull,small,1,367,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:25:rclone_sftp,pull,small,1,5022,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:27:rsyncd,push,small,2,1513,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:29:rclone_sftp,push,small,2,6012,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:31:rsyncd,pull,small,2,374,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:33:rclone_sftp,pull,small,2,5010,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:35:rsyncd,push,mixed,1,1248,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:37:rclone_sftp,push,mixed,1,3883,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:39:rsyncd,pull,mixed,1,686,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:41:rclone_sftp,pull,mixed,1,3863,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:43:rsyncd,push,mixed,2,1239,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:45:rclone_sftp,push,mixed,2,3966,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:47:rsyncd,pull,mixed,2,693,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:49:rclone_sftp,pull,mixed,2,3946,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:52:rclone,local,large,1,1011,0
  +2 more in docs/bench/10gbe-2026-07-05/tool_comparison.csv
docs/plan/SMALL_FILE_CEILING.md:23:measured blit off its ceiling — many-tiny-file and mixed transfers —
docs/plan/SMALL_FILE_CEILING.md:36:| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor:...
docs/plan/SMALL_FILE_CEILING.md:37:| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol...
docs/plan/SMALL_FILE_CEILING.md:38:| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small rem...
docs/plan/SMALL_FILE_CEILING.md:86:matrix — rsyncd, rsync-over-ssh, rclone in its best measured
docs/plan/SMALL_FILE_CEILING.md:87:config (`--ignore-checksum`, tuned `--transfers`), and `cp -a`
docs/plan/SMALL_FILE_CEILING.md:132:rsyncd (spun on the daemon host over ssh), rsync-over-ssh,
docs/plan/SMALL_FILE_CEILING.md:133:rclone best-config, `cp -a` local — fresh targets every run,

exec
/usr/bin/zsh -lc 'rtk bash -n scripts/bench_tripwires.sh' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'rtk git ls-tree -r 7202c1a -- scripts/bench_tripwires.sh .review/findings/sf-1-tripwire-harness.md REVIEW.md' in /home/michael/dev/Blit
 succeeded in 0ms:
100644 blob 9b2eac5c4993a30f7103fa75ba09ab4d1633d60f	.review/findings/sf-1-tripwire-harness.md
100644 blob 29b3a3db4de006926fff3ae1bed679a7a97a6869	REVIEW.md
100755 blob 162b53769de808889655b8b7f3914d082308c50f	scripts/bench_tripwires.sh

codex
`bash -n` is clean and the CSV header matches. The review issues, if any, are likely semantic: what the run omits or mis-measures while still producing plausible output.
exec
/usr/bin/zsh -lc "rtk sed -n '1,260p' scripts/bench_10gbe.sh" in /home/michael/dev/Blit
 succeeded in 0ms:
#!/usr/bin/env bash
# Benchmark blit over 10GbE: local, remote push/pull, TCP vs gRPC, NFS vs native.
#
# Usage:
#   # Set these before running:
#   export REMOTE_HOST=truenas.local        # TrueNAS hostname/IP
#   export NFS_MOUNT=/mnt/truenas           # Local NFS mount point (optional)
#   export SMB_MOUNT=/mnt/truenas_smb       # Local SMB mount point (optional)
#   export REMOTE_MODULE=bench              # blit-daemon module name on remote
#   export REMOTE_PORT=9031                 # blit-daemon port on remote
#
#   # Then run:
#   ./scripts/bench_10gbe.sh
#
# Prerequisites:
#   - Release binaries built: cargo build --release
#   - For remote tests: blit-daemon running on REMOTE_HOST with a module configured
#   - For NFS tests: NFS share mounted at NFS_MOUNT
#   - For SMB tests: SMB share mounted at SMB_MOUNT

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
BLIT="$REPO_ROOT/target/release/blit"
BLIT_DAEMON="$REPO_ROOT/target/release/blit-daemon"

# --- Configuration ---
SIZE_MB=${SIZE_MB:-1024}
SMALL_COUNT=${SMALL_COUNT:-10000}
SMALL_SIZE=${SMALL_SIZE:-4096}
RUNS=${RUNS:-3}
REMOTE_HOST=${REMOTE_HOST:-}
REMOTE_PORT=${REMOTE_PORT:-9031}
REMOTE_MODULE=${REMOTE_MODULE:-bench}
NFS_MOUNT=${NFS_MOUNT:-}
SMB_MOUNT=${SMB_MOUNT:-}

WORK=$(mktemp -d /tmp/blit_10gbe_bench.XXXXXX)
LOG_DIR="$REPO_ROOT/logs/bench_10gbe_$(date +%Y%m%dT%H%M%S)"
mkdir -p "$LOG_DIR"

trap 'echo "Cleaning up..."; rm -rf "$WORK"' EXIT

# --- Helpers ---
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$LOG_DIR/bench.log"; }

generate_large_file() {
    local dir="$1"
    mkdir -p "$dir"
    dd if=/dev/urandom of="$dir/large_${SIZE_MB}M.bin" bs=1M count="$SIZE_MB" 2>/dev/null
    log "Generated ${SIZE_MB}M large file in $dir"
}

generate_small_files() {
    local dir="$1"
    mkdir -p "$dir"
    for i in $(seq 1 "$SMALL_COUNT"); do
        local subdir="$dir/d$(( i / 1000 ))"
        mkdir -p "$subdir"
        dd if=/dev/urandom of="$subdir/f${i}.dat" bs="$SMALL_SIZE" count=1 2>/dev/null
    done
    log "Generated $SMALL_COUNT × ${SMALL_SIZE}B small files in $dir"
}

generate_mixed() {
    local dir="$1"
    mkdir -p "$dir"
    # One large file
    dd if=/dev/urandom of="$dir/big.bin" bs=1M count=512 2>/dev/null
    # Many small files
    for i in $(seq 1 5000); do
        local subdir="$dir/d$(( i / 500 ))"
        mkdir -p "$subdir"
        dd if=/dev/urandom of="$subdir/f${i}.dat" bs=2048 count=1 2>/dev/null
    done
    log "Generated mixed workload in $dir (512M + 5000×2K)"
}

run_timed() {
    local label="$1"
    shift
    local total=0
    local best=999999
    for run in $(seq 1 "$RUNS"); do
        local start=$(date +%s%N)
        "$@" 2>/dev/null
        local end=$(date +%s%N)
        local ms=$(( (end - start) / 1000000 ))
        total=$(( total + ms ))
        if (( ms < best )); then best=$ms; fi
        log "  $label run $run: ${ms}ms"
    done
    local avg=$(( total / RUNS ))
    log "  $label avg: ${avg}ms  best: ${best}ms"
    echo "$label,$avg,$best" >> "$LOG_DIR/results.csv"
}

# Like run_timed, but recreates the local destination before EVERY
# run: blit skips unchanged files, so re-running a copy onto its own
# output measures an incremental no-op, not a full copy. `noop` rows
# use bare run_timed on purpose.
run_timed_fresh() {
    local label="$1"
    local dest="$2"
    shift 2
    local total=0
    local best=999999
    for run in $(seq 1 "$RUNS"); do
        cleanup_dest "$dest"
        local start=$(date +%s%N)
        "$@" 2>/dev/null
        local end=$(date +%s%N)
        local ms=$(( (end - start) / 1000000 ))
        total=$(( total + ms ))
        if (( ms < best )); then best=$ms; fi
        log "  $label run $run: ${ms}ms"
    done
    local avg=$(( total / RUNS ))
    log "  $label avg: ${avg}ms  best: ${best}ms"
    echo "$label,$avg,$best" >> "$LOG_DIR/results.csv"
}

# Push to a FRESH remote subdirectory every run, for the same reason:
# a re-push onto already-delivered content no-ops through the
# need-list (regardless of transport), so each run gets its own
# never-seen target under the module. Extra args (e.g. --force-grpc)
# follow the src argument.
push_timed() {
    local label="$1"
    local src="$2"
    shift 2
    local total=0
    local best=999999
    for run in $(seq 1 "$RUNS"); do
        local target="${REMOTE}${label}_r${run}/"
        local start=$(date +%s%N)
        "$BLIT" copy "$src" "$target" --yes -v "$@" 2>/dev/null
        local end=$(date +%s%N)
        local ms=$(( (end - start) / 1000000 ))
        total=$(( total + ms ))
        if (( ms < best )); then best=$ms; fi
        log "  $label run $run: ${ms}ms"
    done
    local avg=$(( total / RUNS ))
    log "  $label avg: ${avg}ms  best: ${best}ms"
    echo "$label,$avg,$best" >> "$LOG_DIR/results.csv"
}

cleanup_dest() {
    rm -rf "$1" 2>/dev/null || true
    mkdir -p "$1"
}

# --- Generate test data ---
log "=== Generating test data ==="
SRC_LARGE="$WORK/src_large"
SRC_SMALL="$WORK/src_small"
SRC_MIXED="$WORK/src_mixed"

generate_large_file "$SRC_LARGE"
generate_small_files "$SRC_SMALL"
generate_mixed "$SRC_MIXED"

echo "test,avg_ms,best_ms" > "$LOG_DIR/results.csv"

# ============================================================
# 1. LOCAL → LOCAL (baseline)
# ============================================================
log ""
log "=== LOCAL → LOCAL ==="

for workload in large small mixed; do
    eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
    dest="$WORK/dst_local_$workload"

    log "--- $workload ---"
    run_timed_fresh "local_${workload}_copy" "$dest" "$BLIT" copy "$src" "$dest" --yes

    # Incremental (no-change) run against the last copy's output
    run_timed "local_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
done

# ============================================================
# 2. LOCAL → NFS MOUNT (if available)
# ============================================================
if [[ -n "$NFS_MOUNT" && -d "$NFS_MOUNT" ]]; then
    log ""
    log "=== LOCAL → NFS ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        dest="$NFS_MOUNT/blit_bench_$workload"

        log "--- $workload (NFS) ---"
        run_timed_fresh "nfs_${workload}_copy" "$dest" "$BLIT" copy "$src" "$dest" --yes
        run_timed "nfs_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
        rm -rf "$dest"
    done
fi

# ============================================================
# 3. LOCAL → SMB MOUNT (if available)
# ============================================================
if [[ -n "$SMB_MOUNT" && -d "$SMB_MOUNT" ]]; then
    log ""
    log "=== LOCAL → SMB ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        dest="$SMB_MOUNT/blit_bench_$workload"

        log "--- $workload (SMB) ---"
        run_timed_fresh "smb_${workload}_copy" "$dest" "$BLIT" copy "$src" "$dest" --yes
        run_timed "smb_${workload}_noop" "$BLIT" mirror "$src" "$dest" --yes
        rm -rf "$dest"
    done
fi

# ============================================================
# 4. LOCAL → REMOTE PUSH (TCP data plane)
# ============================================================
if [[ -n "$REMOTE_HOST" ]]; then
    # Trailing slash is load-bearing: module endpoints parse as
    # server:/module/ (endpoint.rs rejects the bare form).
    REMOTE="$REMOTE_HOST:$REMOTE_PORT:/$REMOTE_MODULE/"

    log ""
    log "=== LOCAL → REMOTE PUSH (TCP) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"

        log "--- $workload (TCP push) ---"
        push_timed "push_tcp_${workload}" "$src"
    done

    log ""
    log "=== LOCAL → REMOTE PUSH (gRPC fallback) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"

        log "--- $workload (gRPC push) ---"
        push_timed "push_grpc_${workload}" "$src" --force-grpc
    done

    # ============================================================
    # 5. REMOTE → LOCAL PULL (per-workload subpaths: each pull reads
    # only its own workload's dir — pulling the module root would
    # time the accumulated union of everything pushed above)
    # ============================================================
    log ""
    log "=== REMOTE → LOCAL PULL (TCP) ==="
    for workload in large small mixed; do
        eval "src=\$SRC_$(echo $workload | tr a-z A-Z)"
        pull_src="${REMOTE}push_tcp_${workload}_r1/$(basename "$src")/"
        dest="$WORK/dst_pull_tcp_$workload"

        log "--- $workload (TCP pull) ---"
        run_timed_fresh "pull_tcp_${workload}" "$dest" "$BLIT" copy "$pull_src" "$dest" --yes -v
    done

    log ""

exec
/usr/bin/zsh -lc "rtk sed -n '780,815p' crates/blit-daemon/src/service/push/control.rs" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk sed -n '212,232p' crates/blit-daemon/src/service/push/data_plane.rs" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk sed -n '420,460p' crates/blit-core/src/engine/dial.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
            } else {
                let same_size = meta.len() == header.size;
                let same_mtime = metadata_mtime_seconds(&meta)
                    .map(|seconds| seconds == header.mtime_seconds)
                    .unwrap_or(false);
                !(same_size && same_mtime)
            }
        }
        Err(_) => true,
    };
    Ok(requires_upload)
}

/// ue-r2-1f: the daemon's private `desired_streams` ladder retired
/// into the engine's shared shape-aware proposal (same table), clamped
/// to the receiver ceiling this daemon advertises in its
/// CapacityProfile. Single owner for the push stream-count start; the
/// client's dial clamps again on its side.
fn engine_stream_proposal(files: &[FileHeader]) -> u32 {
    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    blit_core::engine::initial_stream_proposal(
        total_bytes,
        files.len(),
        blit_core::engine::local_receiver_capacity().max_streams as usize,
    )
}

#[cfg(test)]
mod data_plane_handle_abort_tests {
    //! design-2 / w4-1: `handle_push_stream`'s `data_plane_handle` was
    //! a bare `Option<JoinHandle<...>>`. Any early `?` return while a
    //! data-plane accept/receive task was running (the manifest
    //! loop's several fallible `send_control_message` calls, or the
    //! `stream.message()?` race in the post-manifest select loop)
    //! dropped the handle without aborting it, leaving the task
    //! running with no owner — unreachable by `CancelJob`. This pins

 succeeded in 0ms:
        .await
        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;

    let stats = TransferStats {
        files_transferred: outcome.files_written as u64,
        bytes_transferred: outcome.bytes_written,
        bytes_zero_copy: 0,
    };

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    let gbps = (stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
    eprintln!(
        "blitd: push data plane: stream complete: files={}, bytes={} ({:.2} Gbps)",
        stats.files_transferred, stats.bytes_transferred, gbps
    );
    Ok(stats)
}

// ── ue-r2-2: resizable accept (mid-transfer stream ADD) ──────────────

/// A control-loop → acceptor registration: the credential the next

 succeeded in 0ms:
/// that is the receiving daemon (it has the manifest) clamped to its
/// own advertised ceiling; on pull_sync it is the sending daemon (it
/// enumerated the source) clamped to the CLIENT's advertised
/// `receiver_capacity.max_streams` (`ue-r2-1g`) — either way the byte
/// receiver's profile is the bound. Table carried over verbatim from
/// the daemon push `desired_streams` ladder it retires (the ladder
/// the old `tuning.rs` doc said "wins"), now engine-owned. The
/// sender's dial clamps again on its side (`set_negotiated_streams`).
/// Live mid-transfer stream changes arrive with `ue-r2-2` resize.
pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
    if file_count == 0 {
        return 1;
    }
    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
        16
    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
        12
    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
        10
    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
        8
    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
        4
    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
        2
    } else {
        1
    };
    proposal.min(ceiling.max(1) as u32)
}

/// Blocked-time ratio for one tuner tick: the share of the tick's
/// wall-clock (× stream count) the senders spent inside socket writes.
/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
/// hysteresis band holds the dial still rather than guessing).
pub(crate) fn blocked_ratio(
    delta_blocked_nanos: u64,
    elapsed: std::time::Duration,
    streams: usize,
) -> f64 {
    let denom = elapsed.as_nanos().saturating_mul(streams as u128);

exec
/usr/bin/zsh -lc "rtk sed -n '640,670p' crates/blit-core/src/engine/dial.rs" in /home/michael/dev/Blit
 succeeded in 0ms:

    #[test]
    fn initial_stream_proposal_matches_the_retired_daemon_table() {
        const MIB64: u64 = 1024 * 1024;
        const GIB: u64 = 1024 * MIB64;
        // Empty need-list → 1 (the old ladder's empty-guard).
        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
        // Byte-keyed tiers: exact lower boundaries AND just-below each
        // (codex ue-r2-1f: representative values would miss a doubled
        // threshold).
        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
        // File-count keys fire independently of bytes.
        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
        // Ceiling clamps the proposal (receiver profile authority).
        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);

exec
/usr/bin/zsh -lc "rtk sed -n '1,28p' DEVLOG.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# DEVLOG

Entries are latest-first. Each line starts with an ISO 8601 timestamp.
Per R5-F5 of `docs/reviews/followup_review_2026-05-02.md`: new entries
go at the top of the file, immediately below this header, so reviewers
scanning chronologically don't miss appended-at-the-bottom changes.

**2026-07-05 02:07:15Z** - **PLAN + PRINCIPLE (SMALL_FILE_CEILING draft, claude)**: Owner ordered "make a plan to beat rsync" then issued a foundational correction mid-interview that now governs all perf work: **goals are ceiling-driven, never competitor-relative** — blit's guiding principles are FAST/SIMPLE/RELIABLE (every change serves one or is scrapped), blit must be the fastest way to transfer files in ANY scenario, and a "beat X by N%" bar embeds a stopping condition ("you'll stop when you hit it — NOT what we are building"). A margin answer (≥25%) given in a quick Q&A was explicitly retracted ("stop, undo that") — future sessions must not re-litigate margins. First-draft BEAT_RSYNC.md was deleted (name itself was competitor-framed; also caught the interactive-rm alias silently declining deletes over non-tty — verify with ls, not exit codes). **`docs/plan/SMALL_FILE_CEILING.md` (Draft, `78eabfd`)**: principle as §1; goal = the measured small-file/mixed cells become bound by a NAMED hardware limit (wire 34 ms for the 10k-push payload, ~150 µs/file single-pipe fs floor proven on the rig's own ZFS, tmpfs-class pull ceiling) demonstrated by profile + stream-scaling curve; tools (rsyncd/rsync-ssh/rclone-best/cp) are tripwires only — any tool faster anywhere = finding, never finish line. Slices sf-1 (tripwire harness + scaling probe), sf-2 (file-count-aware stream proposal — the measured one-stream-for-10k-files policy gap), sf-3a (profile-first limiter analysis) + sf-3b… (one cut per slice), sf-4 (rig re-measure), sf-5 (resize-on-backlog, ue-2's organic trigger), sf-6 (tar-shard wire lane — gated on full REV4 wire-contract deliverables before code), sf-7 (verdict). Codex plan review: **NEEDS FIXES, 5/5 accepted** (evidence made durable in-repo at `docs/bench/10gbe-2026-07-05/` — DIAGNOSIS.md + all session CSVs; harness=tripwire-set by construction; sf-3 split; sf-6 wire deliverables named — the High; competitor-relative residue purged) → fixed `811a3f2`, records `219cecf`. STATE queue carries the draft; **no code until the owner flips Active** (PROTOCOL plan step 6). skippy torn down at session close: daemons stopped, cmp/bench-data payloads removed; binaries + configs staged at blit-bin/ for sf-4. Owner decisions outstanding: plan Active flip, four 10 GbE gate declarations, zero-copy option a/b/c, push go.

**2026-07-05 00:51:18Z** - **BENCHMARK ADDENDUM (tool comparison + zero-copy gate data, claude)**: Owner-requested follow-ups on the same rig. **(1) Zero-copy revisit gate measured** (D-2026-06-12-1's "receive-side CPU saturation" trigger — previously unevaluated): pull receiver (client, tmpfs sink) 0.45 cores at 9.5 Gbit/s; push receiver (skippy daemon, ZFS sink) **1.43 cores** — above the eval doc's "fraction of one core" estimate but nowhere near saturation on 32-core hosts; gate NOT met on this rig, wording is rig-bound (owner reviewing options: append data + regeneralize gate / amend decision / leave). `zero_copy.rs` status re-confirmed: zero callers ever, `(zero-copy 0 bytes)` is an unfed counter, EAGAIN busy-wait defect means any revival is a rewrite; deletion (owner-ratified) still pending w8-1. **(2) blit vs rsync vs rclone** (4 transports × push/pull × 3 workloads × 2 runs, shared data root on skippy, fresh targets every run; CSV: logs/bench_10gbe_20260704T201804/tool_comparison.csv): **large 1 GiB** — blit fastest both directions (push ~950 ms, pull ~885 ms ≈ wire ceiling); rsyncd (native, unencrypted) ~1.01/1.03 s; ssh-transported rsync 1.9–2.3 s and rclone-sftp 2.9–3.1 s pay the cipher tax. **small 10k×4 KiB** — **rsync beats blit on push** (rsyncd 1.5 s, rsync-ssh 1.6 s vs blit 2.4–3.3 s; the known push-side per-file gap — pull asymmetry is 23k vs 4.4k files/s) and edges it on pull (0.37 s vs 0.45–0.48 s); rclone-sftp ~5–6 s. **mixed** — rsyncd wins push (1.24 s vs blit 1.8–2.2 s), blit wins pull (0.58 s vs 0.69 s). **local tmpfs** — blit wins all three (173/79/119 ms vs rsync 332/87/248, rclone 1011/199/557). Takeaway filed: small/mixed PUSH is blit's one loss to rsync on this rig — push-side per-file receive cost is the actionable gap (queue-candidate alongside w6-2b territory); rsyncd's r1 large-push 7.2 s was a one-time cold outlier (r2 1.01 s representative). rsyncd left running on skippy:8730 alongside blitd for further comparison work; both torn down at session close.

**2026-07-05 00:34:30Z** - **BENCHMARK SESSION (10 GbE, TrueNAS↔Arch, claude + owner)**: The owner-called REV4 sign-off session ran end-to-end on the recorded pair (skippy = TrueNAS SCALE, glibc 2.36, binaries exec'd from the pool because /tmp and /home are noexec; client = netwatch-01 Arch, enp6s0 10 GbE). Env: **MTU 9000 both ends** (client NM profile was silently 1500 — set to 9000 via profile edit + full `connection up` reactivation after `device reapply` refused on immutable fields; 8972-byte DF probe green), **iperf3 baseline 9.88 Gbit/s single-stream / 9.91 both directions** — the parity reference (~1.24 GB/s). **Results (validated matrix, avg of 3 true cold runs each)**: TCP push 1 GiB **907 ms ≈ 9.5 Gbit/s** (first payload 14.5 ms), TCP pull 1 GiB **904 ms ≈ 9.5 Gbit/s** — both at the wire ceiling; gRPC-fallback push 1 GiB 1070 ms ≈ 8.0 Gbit/s (first payload 1.0–2.2 s scaling with manifest size — the design-4 no-mid-manifest-negotiation property, deliberate); gRPC pull ≈ TCP pull on large (880 vs 904 ms — 1 MiB-chunked fallback is wire-competitive on clean LAN), TCP 31% faster on 10k-small pulls; push 10k×4 KiB 2.29 s (~4.4k files/s) vs pull 433 ms (~23k files/s — push side per-file daemon work is the gap, w4-4/w9 territory); local blit beats rsync 2.3× on 1 GiB (157 vs 359 ms), no-op mirror 3 ms vs rsync 42 ms. **ue-1 loopback parity band: HOLDS** — local/push/pull per workload: large 195/178/162 ms, small 83/147/116, mixed 119/163/104; worst spread 1.8×, no 10×/2× gap (record: logs/bench_10gbe_20260704T201804/loopback_parity_band.csv). **Reverse direction validated** (daemon on client, skippy as pusher/puller through ufw): push 7.25 Gbit/s data-plane (first payload 3.9 ms), pull 9.75 Gbit/s. **ue-2 observation**: no organic mid-transfer resize triggered anywhere — a single stream saturates 10 GbE (daemon log: 1 GiB stream at 9.69 Gbit/s; negotiated extra streams close cleanly at 0 bytes), including under deliberate 2-concurrent-push contention (8.30+3.21 Gbit/s, clean completion); organic resize needs a constrained/variable wire — owner call whether the ue-r2-2 deterministic suite + this no-wedge evidence satisfies the gate. **Zero-copy datapoint** (D-2026-06-12-1 revisit): every transfer reported `zero-copy 0 bytes` yet the wire is saturated — splice wiring buys nothing at 10 GbE; CPU is not the bottleneck. **Methodology recorded**: engine-vs-wire isolation by design — tmpfs local ends, ZFS absorbing writes async, ARC-warm re-reads, no sync/ARC-eviction between runs (owner asked; disk-path variants — post-push `zpool sync` timing, cold-ARC pulls via `primarycache` — deferred as owner-gated follow-ups). **Session findings fixed en route**: bench_10gbe.sh had never survived past its local phase — stale endpoint grammar (missing load-bearing trailing slash) + daemon-only `--force-grpc-data` flag on the client, both silent aborts under run_timed's `2>/dev/null` + `set -e` (`b9befb8`); codex review of that fix returned 2 High (run_timed re-runs measure no-ops; shared module root lets gRPC pushes no-op and de-isolates pull labels) — both accepted, fixed `92d6326` (run_timed_fresh / per-run push subdirs / per-workload subpath pulls), validated by a full re-run (avg≈best every row). Verdicts: `.review/results/bench-script-fix.{codex,gpt-verdict}.md`. **New foot-gun filed**: `blit copy src_large dst` with an existing local dir named without `./` parses as an mDNS discovery endpoint and errors "remote source must include a module or root" (blit-app endpoints.rs) — STATE open question. Benchmark payload cleaned off the pool; binaries + bench.toml left at /mnt/generic-pool/video/blit-bin/ for future sessions. **Owner calls now pending**: ue-1 declaration (evidence says band holds), ue-2 declaration, zero-copy revisit verdict, REV4 → Shipped flip, and the GitHub push (w9-3 Windows CI check still queued behind it).

**2026-07-04 23:35:54Z** - **CODER (w9-3-test-harness-builder, claude)**: Landed w9-3 through the codex loop (owner go: "continue, use /playbook reviewloop codex" — no playbooks exist in this repo, resolved to the `slice` operator per `.agents/repo-guidance.md` → topmost ratified open row per the 19th handoff). A 6-agent inventory workflow re-derived the audit's 2026-06-11 evidence at HEAD before coding and found the rot had GROWN: **seven** daemon-harness clones, not five — w9-4 (`readonly_enforcement.rs`) and w9-5 (`jobs_lifecycle.rs`) each added another private spawn_daemon/config-struct copy *because* common couldn't express delegation or a second daemon, proving the finding's "the next one will miss at least one" prediction twice — plus 5 `cli_bin` copies, 7 `run_with_timeout`, 4 `ChildGuard`, and **five** bare `Server::builder()` fake servers (not three: `remote_remote.rs` ×2, `jobs_lifecycle.rs`, `pull_sync_with_spec_wire.rs` ×2) vs production's audit-1 keepalive. Slice `f6e592e`: common/mod.rs is now the single owner — `TestContext::builder()` (`.read_only()`/`.delegation()`/`.extra_daemon_args()`; `new()`/`new_read_only()` signature-stable, zero edits in the 13 pre-existing consumers), `spawn_daemon(workspace, name, module_dir, opts)` + `TestContext::spawn_second_daemon` primitives (config superset: `delegation_allowed` serialized explicit `true` = the daemon's own absent-default, verified in runtime.rs before choosing; `[delegation]` table optional), `ensure_daemon_built()` OnceLock'ing the nested `cargo build` (R16-F1 per-process independence kept; ~75 invocations per full run → ≤1 per binary; also fixes remote_remote's dropped `--target` handling and the tcp_fallback/jobs/readonly spawns that ran NO build), shared `spawn_fake_blit_server` scaffold, and new `blit_core::remote::grpc_server::production_server_builder()` (owns the 2026-05-23 keepalive 30s/20s; daemon main.rs + all five fakes route through it; zero bare `Server::builder()` left, grep-verified; +1 mutation-verified pin test). Mid-slice the validation run itself caught the **daemon-spawn load-flakiness live**: `test_admin_find` got an empty listing from another test's daemon — `pick_unused_port`'s probe-drop-to-bind TOCTOU, previously masked by the per-test cargo builds serializing bring-ups; fixed two-layer (process-global claimed-port set — cargo runs test binaries sequentially, so per-process scope is exactly right — plus a `try_wait` child-death check in the readiness poll so an externally stolen port panics with the real reason instead of silently testing a foreign daemon). stderr policy unified to null (was piped-but-never-read; real capture stays w9-6). Review: codex **NEEDS FIXES (1 Medium, accepted — a genuinely sharp catch)**: `spawn_fake_blit_server` still bound `:0` OUTSIDE the claimed set, so a fake could take a port promised to a not-yet-bound daemon in mixed binaries (remote_remote, jobs_lifecycle) — same wrong-listener class, missed path; fixed `8641bc6` (`claim_port()` shared by both paths; the fake keeps its probe listener so its path has no gap at all). Records `c62d15b`. Net −1,251 test-tree lines. Validation: fmt/clippy clean; test-count gate proven by same-method A/B via `git stash` — HEAD 1478/0/2, slice 1479/0/2 across 37 suites, exactly +1, per-file `#[test]` counts identical (STATE's recorded "1479" baseline was a different aggregation, off-by-one vs the same tree); full suite ×2 + `admin_verbs` ×10 post-fix all green. All on master, unpushed. Next: strict design-queue order gives **w7-1** (mirror-executor consolidation) as topmost ratified open row; filed alternatives w6-2a/b/c + relay-1, coder's pick.

**2026-07-04 22:09:54Z** - **OWNER Q&A + PUSH (benchmark host plan, claude)**: Owner pushed `master` → `github` at `10d89e0` (first push carrying the w9-1/w9-4 ungated suites since the Windows triage — windows-latest CI on it is the "meaningfully green" check; `origin` gitea mirror not pushed, lags). Benchmark host question settled in Q&A: the 10 GbE **sign-off pair is TrueNAS (skippy) ↔ Arch client, all-Linux** — the zero-copy/splice revisit gate (D-2026-06-12-1) needs a Linux consumer, and ue-1's parity band should measure the engine without Windows I/O confounders (win_fs, TCP autotuning, filter drivers). Key facts recorded: the client box **dual-boots Win 11/Arch on identical hardware** (so a later TrueNAS→Win 11 bare-metal pull in the same hardware window gives a clean platform delta as a deployment-parity datapoint, not a gate), and a **Windows VM on the Arch install** exists for Windows-specific functional checks — explicitly never for perf numbers (virtio/NAT skews throughput). iperf3 baseline per pair before any Blit numbers. Plan recorded in STATE.md Queue item 2. Handoff commit follows; push of the handoff itself pending the owner's ref-listing go per push policy.

**2026-07-04 21:46:39Z** - **CODER (design-3-unbounded-data-plane-connects, claude)**: Landed design-3 through the codex loop (same session, fourth slice; coder's pick of the long-sanctioned smaller alternative over the large w9-3 harness consolidation — queue policy leaves sequencing to the coder). Both TCP data-plane client connects ran unbounded — the audit-2 wave bounded every control-plane connect at 30 s but never reached the data plane, so a firewalled/black-holed data port (the daemon advertises a fresh ephemeral port per transfer; asymmetric firewalls passing 9031 but blocking ephemerals are common) hung for the kernel SYN timeout (60–127 s) with no message. Sites re-verified at HEAD: the pull site is now `connect_pull_stream` (split at ue-r2-2, shared by resize-ADD dials), the push site `DataPlaneSession::connect_with_probe` (elastic dials included). Slice `49dcec6`: `remote::transfer::socket::dial_data_plane(addr, handshake, tcp_buffer_size)` — the client-side mirror of the daemon's bounded accept, in the w1-family policy module: connect bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` (the row's sanctioned constant — no fifth 30 s literal), `configure_data_socket` applied, handshake write bounded by `DATA_PLANE_TOKEN_TIMEOUT` (mirrors the acceptor's bounded token read — the finding's "also the token write" clause); on either timeout the chain carries an `io::ErrorKind::TimedOut` source with text naming addr + the likely-firewall cause, so `remote::retry::is_retryable` classifies it transient and `--retry` re-dials. Both call sites collapsed onto the helper; socket.rs's w1-2-era "connect timeouts live at the call sites" module-doc paragraph rewritten (comment-truth). Tests +3 (blit-core 389 → 392): happy path (policy + handshake delivery), deterministic timeout SHAPE via an accepting-but-never-reading peer against a 64 MiB handshake (TimedOut chain + retryable — mutation-verified: swapping the timeout error for a plain eyre message fails the pin), TEST-NET black-hole connect bounded (environment-tolerant: fast-reject networks skip the shape assertions, the bound is asserted always). Review: codex **PASS, zero findings** (independently confirmed the pull resize-ADD non-fatal-dial posture survived and StallGuard/cancellation/byte accounting untouched). Validation: fmt/clippy clean, `cargo test --workspace` 1476 → 1479/0/2 across 37 suites. All on master, unpushed. Session total: w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3 — four rows closed, six commits of records. Next: w9-3 (test-harness builder) is the topmost ratified open row and the right size for a fresh session; filed alternatives w6-2a/b/c + relay-1.

**2026-07-04 21:33:31Z** - **CODER (w4-4-blocking-work-off-runtime, claude)**: Landed w4-4 through the codex loop (same session, standing "reviewloop" go → topmost ratified open row after w6-1/w6-2). Both audit halves re-verified at HEAD before coding — the 2026-06-11 sites moved: daemon pull.rs died at ue-r2-1h, its enumeration relocated into pull_sync.rs; the push manifest loop's per-entry blocking is control.rs `file_requires_upload` (canonical-containment ancestor walk ≥2 canonicalize syscalls + stat, inline on a tokio worker — ~3M+ blocking syscalls for a 1M-file push). Slice `0feca34`: **(A)** manifest entries buffer into `PendingManifestEntry` and drain through `drain_manifest_checks` — ONE spawn_blocking per chunk runs the batch's checks (batch moved in/out, no clones), need-list pushes follow in async context in manifest order; `MANIFEST_CHECK_CHUNK` = FILE_LIST_EARLY_FLUSH_ENTRIES (128); the mid-manifest TCP data-plane spin-up moved from per-entry to post-chunk-drain (same guard — design-4's no-mid-manifest-fallback-negotiation invariant untouched); ManifestComplete drains the sub-chunk remainder (no spin-up — post-manifest negotiation owns it); the spec's lexical-containment alternative was REJECTED as weakening the F2 canonical posture (an in-module symlink could redirect the stat outside; check stays canonical, now on a blocking thread). **(B)** `collect_pull_entries_with_checksums` now runs entirely on one spawn_blocking thread (thin async wrapper + `collect_pull_entries_sync`; the directory branch's inner spawn_blocking unwrapped): previously the single-file branch ran its metadata probes and — with --checksum — a full synchronous Blake3 of an arbitrarily large file inline on a runtime worker, plus the top-level is_file/is_dir probes. Review: codex **NEEDS FIXES (1 Medium, accepted — a genuinely good catch)**: chunk-only draining muted the need-list batcher's 64 KiB/5 ms early-flush triggers between chunk boundaries (they only evaluate inside push(), which chunking confines to drain time) — a slowly-enumerating client's first FilesToUpload + TCP spin-up would wait for 128 trickled entries instead of ~5 ms; fixed `768e7e3` with `manifest_drain_due(pending_len, oldest_buffered)` — drain on chunk-full OR oldest-entry age ≥ MANIFEST_CHECK_MAX_DELAY (= the batcher's own delay bound), evaluated on next arrival exactly like the batcher's push-time semantics; fast streams still hit the cap first. Tests: +4 `manifest_check_batch_tests` (decision parity incl. POSIX wire paths + order, empty-drain no-op, containment-escape rejection through the batched path, trigger contract), mutation-verified (all-true check → parity pin fails); half B rides existing pins (single_file_filter_tests; the 500-file design-5 e2e drives ≥3 chunk drains + remainder + spin-up end-to-end). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1472 → 1476/0/2 across 37 suites. All on master, unpushed. Next: strict design-queue order gives w9-3 (test-harness builder) as topmost ratified open row; filed alternatives w6-2a/b/c + design-3 + relay-1, coder's pick.

**2026-07-04 21:13:10Z** - **CODER (w6-2-progress-residue-verify, claude)**: Landed w6-2 through the codex loop (same session as w6-1; owner's standing "reviewloop with codex as each slice lands" go → next topmost open row). The ratified W6.2 spec makes this a verify-then-file slice — verification is step 1, each confirmed item becomes its own follow-on — so the deliverable is records, not code. All three §1.6 residue claims **CONFIRMED** at HEAD `8fd8978` (evidence derived twice: the w6-1 mapping workflow, then hand spot-checks): (1) delegated live progress is wire-dead — `BytesProgress` has zero production producers (proto + blit-app consumer/tests only; the dst daemon sends `Started`, silence during transfer per the deliberate 0.1.0 gap at delegated_pull.rs:363-369, then one post-hoc `ManifestBatch{file_count=files_transferred}`), while the consumer bridge sits ready and the dst daemon ALREADY meters the bytes (core.rs:667 feeds the row atomic) — the fix is a bridge, not new instrumentation; (2) daemon row byte counters stay 0 for push receive (sink built without `.with_byte_progress()`, `execute_receive_pipeline(.., None)` at push/data_plane.rs:1086) and pull_sync serve (no counter at any of the 3 send pipelines, pull_sync.rs:635/:765/:795) — `job.bytes_counter()` is wired exactly once in service code, the delegated dispatch; (3) no denominators or file counts anywhere on the daemon event stream — `TransferProgress` hardcodes bytes_total/files_completed/files_total 0 (core.rs:240-242), `TransferComplete.files` 0 (:322-325, `tcp_fallback_used` false :329), `GetState` bytes_total 0 (:994-996). Filed as three INDEPENDENT rows in REVIEW.md's pending-review section (the design-1..5 entry route, not new ratified-queue rows): **w6-2a** delegated BytesProgress producer (bridge the already-fed row atomic onto the DelegatedPullProgress stream; client needs nothing — w6-1's aggregate lane + `report_bytes_progress` are ready), **w6-2b** wire the counter through push receive + pull_sync serve, **w6-2c** denominators + files counter on ActiveJobs rows (absorbs the tcp_fallback_used=false honesty note). Slice `0aba593` (docs gate green). Review: codex **NEEDS FIXES (2 Low, both doc-coherence, both accepted)** — "no code anywhere constructs BytesProgress" overstated (blit-app unit tests construct it; clarified to zero PRODUCTION producers) and "2b is substrate for 2a" was wrong (the delegated counter is already fed — codex cited my own Claim-1 evidence back at me; slices reworded as independent, 2b→2a→2c smallest-first only); fixed `8b7829d`. No code change; workspace stays 1472/0/2 as of `8fd8978`. All on master, unpushed. Next: strict design-queue order gives w4-4 (spawn_blocking the stat/canonicalize batch) as topmost ratified open row; filed alternatives now w6-2a/b/c + design-3 + relay-1 — sequencing coder's pick unless the owner orders.

**2026-07-04 21:02:20Z** - **CODER (w6-1-progress-event-contract, claude)**: Landed w6-1 through the codex loop (owner go: "continue. reviewloop with codex as each slice lands" → topmost open row per the 14th handoff). A 5-agent mapping workflow re-derived the audit's 2026-06-11 §1.6 inventory against post-REV4 HEAD before coding (producer census / consumer census / push-delegated-daemon boundary / test inventory / gapcheck critic that re-grepped every `report_*`/`ProgressEvent` hit — zero unmapped sites): the CLI's TCP-pull byte double-count (design-1) was live at `pipeline.rs` FILE receive (`Payload{0,N}` + `FileComplete{path,N}`, same bytes twice); push rode `FileComplete` only; delegated rode `Payload{fΔ,bΔ}`; plus four quieter producer sins — gRPC pull `finalize_active_file` leaked the ABSOLUTE local dest path (every other producer emits wire-relative), tar-shard member files were never counted on either transport, TCP resume records emitted nothing, and the two dead emitters encoded contradictory semantics. Slice `8fd8978`: contract defined ON the enum in blit-core — **bytes ride `Payload` only; `FileComplete` loses its `bytes` field entirely** (the design-1 class is now unrepresentable at the type level, not avoided by convention); files count exactly once via either one byteless `FileComplete{wire-relative path}` (per-file lane) or `Payload.files` deltas (aggregate lane: delegated bridge, gRPC tar-shard applier); `ManifestBatch` documented as the direction-flavored denominator (pull full-manifest / push need-list / delegated post-hoc — unification is wire/UX scope, deliberately not this slice). All producers normalized: receive FILE double-emit fixed; receive TAR_SHARD gains per-member completions; BLOCK/BLOCK_COMPLETE lanes gain `Payload`+`FileComplete` (TCP resume was progress-invisible); send-side sink worker (push TCP + gRPC fallback share it) moves planned sizes onto `Payload`; gRPC pull TarShardComplete counts via `Payload{stats.files,0}`, BlockComplete completes per-file, finalize carries the wire path through `active_file`; dead `send_payloads_with_progress` + `transfer_payloads_via_control_plane` conformed pending w8 deletion. Consumers collapsed onto new `ProgressTotals` (blit-core, next to the enum, `started()` gate included): CLI monitor rewritten (fixes design-1's ~2× `[progress]` lines; `--json` `file_complete` keeps its key shape with constant `bytes:0`; verbose gRPC-pull lines now print wire-relative paths) and all three TUI forwarders; the TUI's three `accumulate_*` rules deleted, its 7 tests rewritten in place against `ProgressTotals`. Tests +12 in blit-core (6 contract, 4 pipeline emission incl. exact-sequence pins driven over in-memory wire records via a new `RecordingSink`, 2 finalize) — the emission tests exposed that the fuzz helper `encode_block_complete` wrote truncated records (missing mtime/perms vs the reader), fixed; two mutation checks verified the pins bite (drop FileComplete counting → 3 fail; drop the FILE arm's `report_payload` → emission pin fails). Review: codex **PASS, zero findings** (explicitly checked the W6.1/W6.2 scope split — daemon/ByteProgressSink counters untouched by design, that residue is w6-2). design-1's row closed `[x]` alongside (fixed structurally, graded in the same round). Validation: fmt/clippy clean, `cargo test --workspace` 1460 → 1472/0/2 across 37 suites. All on master, unpushed. Next: w6-2 (progress-residue verify-then-fix) tops the open queue; design-3 remains the sanctioned smaller alternative. Process note: mutation-testing uncommitted work must restore from a scratchpad copy, not `git checkout` (one self-inflicted clobber of `progress.rs` was fully re-applied and re-verified this session).

**2026-07-04 20:04:30Z** - **CODER (w3-1-memory-aware-buffer-pool, claude)**: Landed w3-1 through the codex loop (owner go: "continue" → topmost open row per the 13th handoff; its "after W2.2 settles the tuning owner" prerequisite held — owner is `engine::TransferDial`, so the spec's `for_data_plane(tuning, streams)` became `for_data_plane(chunk_bytes, streams)`, the chunk read from the dial at each site). A 5-agent audit workflow re-verified the 2026-06-11 evidence against post-REV4 HEAD before coding: exactly three formula sites survive (push client, pull_sync multistream, pull_sync resume — the daemon pull.rs copy died with the Pull RPC at ue-r2-1h); the resume site is a hand-rolled `pool_size = 4` whose pool is **inert at runtime** (resume only sends via `send_block*`, which write caller slices — `pool.acquire` is reached solely from `send_file_double_buffered`), so unifying it to the formula's 6 is behavior-neutral; the double-buffered sender holds TWO pool buffers acquired sequentially (hold-one-wait-for-second), so any memory cap admitting fewer than 2 buffers per live stream can deadlock a sender against itself; shrinking pool buffers below the session's chunk_bytes is wire-safe (file bytes travel raw under a size-carrying header — no per-chunk framing; the effective send granularity already IS the pool buffer size); and `dial.ceiling_max_streams()` is a hard 3-layer-enforced bound on live streams under resize, with ADDed streams sharing the epoch-0 pool on both elastic paths. Slice `f49f8f6`: `BufferPool::for_data_plane` in buffer.rs backed by pure `data_plane_pool_params(chunk_bytes, streams, available_memory)` — formula unchanged when memory is plentiful (pinned pure-hoist parity), budget capped at available/4 (the pool's own doc example, ignored by every call site until now), liveness floor `budget ≥ buffer_size × streams × 2` always beats the cap with buffer_size shrinking toward the shared floor instead of concurrency; elastic paths authorize `ceiling_max_streams()` up front (lazy allocation makes the ceiling authorization free until resize ADDs streams — closes both sites' "growing the pool live is a W3.1 concern" deferral without live-growth machinery); resume passes 1. **Bonus real bug fixed**: the hoisted `available_memory_bytes()` drops the old helper's `* 1024` — sysinfo 0.38 returns BYTES not KiB (verified against the vendored crate source), so available memory was over-reported 1024×, which had made BufferSizer's /10 cap vacuous forever and would have made the new /4 cap vacuous too; also `System::new()`+`refresh_memory()` replaces the process-table-walking `new_all()`, zero-report 512 MiB fallback kept. `DATA_PLANE_BUFFER_FLOOR` (64 KiB) exported and adopted at the same-semantic floor sites (session chunk clamp, receive-buffer clamp, dial inflight clamp — whose comment already said "matching the session's minimum buffer"); coincidentally-equal 64 KiB literals deliberately untouched. Comment-truth: `RECEIVE_CHUNK_SIZE`'s false "matches the send side" claim rewritten (receive is deliberately 1 MiB vs the sender's 16–64 MiB; the asymmetry is legal — no per-chunk framing); BufferPool header example now shows the constructor. Tests: +8 params-layer pins (legacy parity, floor, cap, shrink-preserving-liveness, tiny-cap liveness override, full-grid liveness/floor property sweep, zero-memory fallback, real-sysinfo smoke), mutation-verified (cap+liveness line reverted → 3 pins fail; restored → green). Review: codex **PASS, zero findings** (it independently walked the two-buffer acquisition, the audit-11 exact-size guard, tar-shard chunking, and the ceiling authorization; first invocation was killed by a session restart before output — the record is the complete re-run). Known gaps in the finding doc: receive-side dial tuning stays out of scope (rest of constants-receive-chunk-1mib-asymmetry, separate slice if wanted); resume path's inert pool + dead prefetch literal left as-is; no memory-capped-host e2e (capped regimes pinned at the params layer; the pull-sync deadlock canary passes). Validation: fmt/clippy clean, `cargo test --workspace` 1460/0/2 across 37 suites (baseline 1452). All on master, unpushed. Next: w6-1 (progress-event contract) tops the open queue; design-3 remains the sanctioned smaller alternative.

**2026-07-04 15:24:23Z** - **CODER (w2-2-stream-ladder-owner, claude)**: Landed w2-2 through the codex loop (owner go: "continue" → topmost open row per the 12th handoff). The row as filed (2026-06-11) predates REV4, which already delivered its three stream-count legs: the `determine_remote_tuning` ladder died at ue-r2-1e (live dial), daemon `desired_streams` at ue-r2-1f (`engine::initial_stream_proposal` — byte- AND file-count-keyed, satisfying the spec's "takes file_count"), and `pull_stream_count` with the Pull RPC at ue-r2-1h; D-2026-06-20-1 recorded the absorption in v1 slice IDs. The remaining leg — the transfer_plan 16/32 MiB chunk ladder — turned out to be **entirely dead policy**, established by a 5-agent audit workflow + hand verification: every remote path overrode it with `Some(dial.chunk_bytes())` (push client 5 refresh sites + ensure_dial, pull_sync both literals); the only paths where the ladder won (local engine, test callers) discarded the value (`PlanUpdate` carries payloads only); the single workspace read of `PlannedPayloads.chunk_bytes` sat behind a `chunk_bytes == 0` guard no live caller can trigger (all pass the dial value, floored ≥ 64 KiB). The spec's "make transfer_plan take chunk_bytes as input" predates the dial — with zero consumers, threading a value through the planner would be plumbing with no reader, so the honest single-owner outcome was deletion. Slice `01209bc`: ladder + `Plan` wrapper deleted (`build_plan` → `Vec<TransferTask>`); `PlannedPayloads` deleted (`plan_transfer_payloads` → `Result<Vec<TransferPayload>>`, ripple through diff_planner/streaming_plan/pipeline tests/re-exports); `PlanOptions.chunk_bytes_override` + all refresh sites deleted (push `plan_options` now immutable default; two arms keep bare `ensure_dial` calls — first-need creation and first-wins ceilings unchanged); unreachable fallback guard in `stream_fallback_from_queue` deleted; `plan_to_daemon_format` deleted (git log -S: never called in repo history — its "server pull mode" comment was never true); orphaned `TuningParams` deleted (producer died at ue-r2-1e); write-only kickoff histogram collapsed to the `total_bytes` accumulator that was its only read. Comment-truth sweep: dial.rs mutability-model doc no longer claims chunk/prefetch are "read at each use site" (consumers snapshot at session/pipeline/batch setup; steps reach epoch-N sockets and later fallback batches); buffer.rs example cites the dial, not `TuningParams`. Behavior byte-identical on every live path. Tests: +4 transfer_plan unit pins (module had zero) — tier classification/interleave, single-small-file no-tar, force_tar single-file, count-target shard splitting with the 128 clamp; deletions are compile-guarded (w2-1 evidence shape); zero tests deleted. Review: codex **NEEDS FIXES (1 Low)** — the first bare ensure_dial comment said "fallback batch" inside the `TransferMode::DataPlane` branch; accepted (mislabel sits exactly on the invariant under review), fixed `27f53a0` (one word). W3.1's "after W2.2 settles the tuning owner" prerequisite is now settled: the owner is `engine::TransferDial`. New discoveries → STATE Open questions: `725aa07` tracked a 236-file stale worktree snapshot (`.claude/worktrees/vigilant-mayer/`) into the repo; WHITEPAPER still describes the pre-dial tuning world (stale since ue-r2-1e, w10 territory). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1452/0/2 across 37 suites (baseline 1448). All on master, unpushed. Next: w3-1 (memory-aware BufferPool) tops the open queue; design-3 remains the sanctioned smaller alternative.

exec
/usr/bin/zsh -lc "rtk sed -n '1,40p' docs/bench/10gbe-2026-07-05/cmp_fair2.csv" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk sed -n '1,40p' docs/bench/10gbe-2026-07-05/cmp_fair.csv" in /home/michael/dev/Blit
 succeeded in 0ms:
transport,direction,workload,run,ms,status
rclone_webdav,push,large,1,3326,0
rclone_webdav,pull,large,1,1011,0
rclone_webdav,push,large,2,3365,0
rclone_webdav,pull,large,2,1004,0
rclone_webdav,push,small,1,314978,0
rclone_webdav,pull,small,1,108708,0

 succeeded in 0ms:
transport,direction,workload,run,ms,status
rclone_webdav,push,large,1,20,1
rclone_webdav,pull,large,1,18,1
rclone_webdav,push,large,2,18,1
rclone_webdav,pull,large,2,17,1
rclone_webdav,push,small,1,18,1
rclone_webdav,pull,small,1,17,1
rclone_webdav,push,small,2,18,1
rclone_webdav,pull,small,2,18,1
rclone_webdav,push,mixed,1,18,1
rclone_webdav,pull,mixed,1,18,1
rclone_webdav,push,mixed,2,18,1
rclone_webdav,pull,mixed,2,17,1
rclone_sftp_nohash,push,small,1,5218,0
rclone_sftp_nohash,pull,small,1,2742,0
rclone_local_nohash,local,large,1,227,0
rclone_local_nohash,local,small,1,136,0
rclone_local_nohash,local,mixed,1,127,0

exec
/usr/bin/zsh -lc 'rtk rg -n "rclone copy|rsync --daemon|tool comparison|rclone_sftp|rclone_local_nohash" DEVLOG.md docs/bench scripts' in /home/michael/dev/Blit
 succeeded in 0ms:
DEVLOG.md:10:**2026-07-05 00:51:18Z** - **BENCHMARK ADDENDUM (tool comparison + zero-copy gate data, claude)**: Owner-requested follow-ups on the same rig. **(1) Zero-copy revisit gate measured** (D-2026-06-12-1's "receive-side CPU saturation" trigger — previously unevaluated): pull receiver (client, tmpfs sink) 0.45 cores at 9.5 Gbit/s; push receiver (skippy daemon, ZFS sink) **1.43 cores** — above the eval doc's "fraction of one core" estimate but nowhere near saturation on 32-core hosts; gate NOT met on this rig, wording is rig-bound (owner reviewing options: append data + regeneralize gate / amend decision / leave). `zero_copy.rs` status re-confirmed: zero callers ever, `(zero-copy 0 bytes)` is an unfed counter, EAGAIN busy-wait defect means any revival is a rewrite; deletion (owner-ratified) still pending w8-1. **(2) blit vs rsync vs rclone** (4 transports × push/pull × 3 workloads × 2 runs, shared data root on skippy, fresh targets every run; CSV: logs/bench_10gbe_20260704T201804/tool_comparison.csv): **large 1 GiB** — blit fastest both directions (push ~950 ms, pull ~885 ms ≈ wire ceiling); rsyncd (native, unencrypted) ~1.01/1.03 s; ssh-transported rsync 1.9–2.3 s and rclone-sftp 2.9–3.1 s pay the cipher tax. **small 10k×4 KiB** — **rsync beats blit on push** (rsyncd 1.5 s, rsync-ssh 1.6 s vs blit 2.4–3.3 s; the known push-side per-file gap — pull asymmetry is 23k vs 4.4k files/s) and edges it on pull (0.37 s vs 0.45–0.48 s); rclone-sftp ~5–6 s. **mixed** — rsyncd wins push (1.24 s vs blit 1.8–2.2 s), blit wins pull (0.58 s vs 0.69 s). **local tmpfs** — blit wins all three (173/79/119 ms vs rsync 332/87/248, rclone 1011/199/557). Takeaway filed: small/mixed PUSH is blit's one loss to rsync on this rig — push-side per-file receive cost is the actionable gap (queue-candidate alongside w6-2b territory); rsyncd's r1 large-push 7.2 s was a one-time cold outlier (r2 1.01 s representative). rsyncd left running on skippy:8730 alongside blitd for further comparison work; both torn down at session close.
scripts/bench_tripwires.sh:151:                  rsync --daemon --config='$SESSION/rsyncd.conf'"
scripts/bench_tripwires.sh:218:                fresh_local "$dst"; timed_row rclone local "$workload" "$run" rclone copy "$src" "$dst" --ignore-checksum --transfers "$RCLONE_TRANSFERS"
scripts/bench_tripwires.sh:248:                timed_row rclone_sftp push "$workload" "$run" \
scripts/bench_tripwires.sh:249:                    rclone copy "$src" ":sftp,host=$SSH_HOST:$SESSION/push/rclone_${workload}_r${run}" \
scripts/bench_tripwires.sh:267:                timed_row rclone_sftp pull "$workload" "$run" \
scripts/bench_tripwires.sh:268:                    rclone copy ":sftp,host=$SSH_HOST:$SESSION/seed/$workload" "$dst" \
docs/bench/10gbe-2026-07-05/cmp_fair.csv:14:rclone_sftp_nohash,push,small,1,5218,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:15:rclone_sftp_nohash,pull,small,1,2742,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:16:rclone_local_nohash,local,large,1,227,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:17:rclone_local_nohash,local,small,1,136,0
docs/bench/10gbe-2026-07-05/cmp_fair.csv:18:rclone_local_nohash,local,mixed,1,127,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:5:rclone_sftp,push,large,1,3128,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:9:rclone_sftp,pull,large,1,2851,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:13:rclone_sftp,push,large,2,3088,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:17:rclone_sftp,pull,large,2,2884,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:21:rclone_sftp,push,small,1,5848,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:25:rclone_sftp,pull,small,1,5022,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:29:rclone_sftp,push,small,2,6012,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:33:rclone_sftp,pull,small,2,5010,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:37:rclone_sftp,push,mixed,1,3883,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:41:rclone_sftp,pull,mixed,1,3863,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:45:rclone_sftp,push,mixed,2,3966,0
docs/bench/10gbe-2026-07-05/tool_comparison.csv:49:rclone_sftp,pull,mixed,2,3946,0

codex
I’ve got enough evidence for the main review points. I’m doing one final pass over the line-numbered review records so the output can include both script defects and record-accuracy defects without over-reporting.
exec
/usr/bin/zsh -lc 'rtk git diff 7202c1a''^ 7202c1a -- scripts/bench_tripwires.sh | sed -n '"'1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
scripts/bench_tripwires.sh | 384 +++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 384 insertions(+)

Changes:

scripts/bench_tripwires.sh
  @@ -0,0 +1,384 @@
  +#!/usr/bin/env bash
  +# Tripwire + stream-scaling harness (SMALL_FILE_CEILING sf-1).
  +#
  +# Re-runs the 2026-07-05 tool-comparison matrix against any daemon host
  +# in one command, plus a stream-scaling probe (files/s vs negotiated
  +# stream count). Derived from scripts/bench_10gbe.sh and the ad-hoc
  +# session runner behind docs/bench/10gbe-2026-07-05/tool_comparison.csv
  +# (same CSV schema, so runs are directly comparable to the committed
  +# baseline).
  +#
  +# Tripwire semantics (docs/plan/SMALL_FILE_CEILING.md, Principle):
  +# the tools here are NOT targets — any cell where any tool measures
  +# faster than blit is proof blit is off its hardware ceiling and is a
  +# finding to fix. The harness matrix and the plan's tripwire list are
  +# the same set by construction.
  +#
  +# Usage (one command against a daemon host):
  +#   DAEMON_HOST=skippy \
  +#   REMOTE_ROOT=/mnt/generic-pool/video/blit-bin/bench-data \
  +#   REMOTE_BLIT_DAEMON=/mnt/generic-pool/video/blit-bin/blit-daemon \
  +#   ./scripts/bench_tripwires.sh [matrix|scale|all]     # default: all
  +#
  +#   Local-only tripwires (no DAEMON_HOST): blit vs rsync/rclone/cp on
  +#   this machine's ${TMPDIR:-/tmp}.
  +#
  +# Environment:
  +#   DAEMON_HOST        network + ssh name of the daemon host (remote cells)
  +#   SSH_HOST           ssh alias if it differs from DAEMON_HOST
  +#   REMOTE_ROOT        writable dir on the daemon host; a per-invocation
  +#                      session dir is created (and removed) under it.
  +#                      NOTE: must be exec-friendly for SPIN_DAEMONS — on
  +#                      TrueNAS /tmp and /home are noexec (session lesson).
  +#   REMOTE_BLIT_DAEMON path to blit-daemon ON the daemon host
  +#   SPIN_DAEMONS=1     spin blitd (--root, module "default") + rsyncd on
  +#                      the daemon host over ssh; 0 = daemons already run
  +#                      (then set BLIT_PORT/BLIT_MODULE/RSYNCD_PORT and
  +#                      optionally BLITD_LOG for scale-mode stream counts)
  +#   BLIT_PORT=9031  BLIT_MODULE=default  RSYNCD_PORT=8730
  +#   BLITD_LOG          remote path of blitd's stderr log (scale mode
  +#                      stream counting when SPIN_DAEMONS=0)
  +#   RUNS=2             timed runs per cell (baseline was best-of-2)
  +#   TIMEOUT_S=600      per-run cap (a wedged tool records status 124)
  +#   RCLONE_TRANSFERS=16  rclone best-config concurrency (fairness flags
  +#                      --ignore-checksum + tuned --transfers per
  +#                      docs/bench/10gbe-2026-07-05/DIAGNOSIS.md)
  +#   SIZE_MB=1024 SMALL_COUNT=10000 SMALL_SIZE=4096   workload knobs
  +#   SCALE_COUNTS="200 1000 5000 10000 25000 50000"   probe file counts
  +#                      (chosen to cross engine::initial_stream_proposal
  +#                      tiers: expected proposals 1/2/4/8/8/10)
  +#   BASELINE_CSV       committed baseline to diff blit cells against
  +#                      (default docs/bench/10gbe-2026-07-05/tool_comparison.csv)
  +#
  +# Requirements: ssh key access to the host (rsync-over-ssh and
  +# rclone-sftp cells deliberately pay the cipher tax — that is their
  +# datapoint); rsync on both ends; rclone on the client. Missing tools
  +# skip their cells with a note instead of failing the run.
  +#
  +# Methodology (matches the committed baseline): local ends on
  +# ${TMPDIR:-/tmp} (tmpfs on the rig), fresh never-seen target dirs for
  +# EVERY timed run (blit and rsync both no-op onto delivered content),
  +# pull sources seeded once per workload (write path leaves ZFS ARC
  +# warm, so pulls are warm re-reads), async writes, no sync between
  +# runs, wall-clock ms.
  +#
  +# Exit codes: 0 = ran and no tripwire tripped; 3 = at least one tool
  +# beat blit somewhere (the summary names the cells); 1 = harness error.
  +
  +set -euo pipefail
  +
  +MODE=${1:-all}
  +case "$MODE" in matrix|scale|all) ;; *) echo "usage: $0 [matrix|scale|all]" >&2; exit 1;; esac
  +
  +SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
  +REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
  +BLIT=${BLIT:-"$REPO_ROOT/target/release/blit"}
  +
  +DAEMON_HOST=${DAEMON_HOST:-}
  +SSH_HOST=${SSH_HOST:-$DAEMON_HOST}
  +REMOTE_ROOT=${REMOTE_ROOT:-}
  +REMOTE_BLIT_DAEMON=${REMOTE_BLIT_DAEMON:-}
  +SPIN_DAEMONS=${SPIN_DAEMONS:-1}
  +BLIT_PORT=${BLIT_PORT:-9031}
  +BLIT_MODULE=${BLIT_MODULE:-default}
  +RSYNCD_PORT=${RSYNCD_PORT:-8730}
  +BLITD_LOG=${BLITD_LOG:-}
  +RUNS=${RUNS:-2}
  +TIMEOUT_S=${TIMEOUT_S:-600}
  +RCLONE_TRANSFERS=${RCLONE_TRANSFERS:-16}
  +SIZE_MB=${SIZE_MB:-1024}
  +SMALL_COUNT=${SMALL_COUNT:-10000}
  +SMALL_SIZE=${SMALL_SIZE:-4096}
  +SCALE_COUNTS=${SCALE_COUNTS:-"200 1000 5000 10000 25000 50000"}
  +BASELINE_CSV=${BASELINE_CSV:-"$REPO_ROOT/docs/bench/10gbe-2026-07-05/tool_comparison.csv"}
  +
  +[[ -x "$BLIT" ]] || { echo "blit binary not found at $BLIT (build with cargo build --release or set BLIT=)" >&2; exit 1; }
  +
  +WORK=$(mktemp -d "${TMPDIR:-/tmp}/blit_tripwires.XXXXXX")
  +STAMP=$(date +%Y%m%dT%H%M%S)
  +LOG_DIR="$REPO_ROOT/logs/tripwires_$STAMP"
  +mkdir -p "$LOG_DIR"
  ... (284 lines truncated)
  +384 -0
[full diff: rtk git diff --no-compact]

exec
/usr/bin/zsh -lc "rtk bash -lc 'git show 7202c1a:.review/findings/sf-1-tripwire-harness.md | nl -ba | sed -n \"1,120p\"'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# sf-1 — Tripwire + stream-scaling harness
     2	
     3	**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-1.
     4	**Status**: implemented, codex review pending.
     5	
     6	## What
     7	
     8	`scripts/bench_tripwires.sh` — makes the 2026-07-05 tool-comparison
     9	baseline (`docs/bench/10gbe-2026-07-05/`) re-runnable against any
    10	daemon host in one command, and adds the plan's stream-scaling probe
    11	(files/s vs the stream count the transfer actually ran with). No
    12	production code.
    13	
    14	## Approach
    15	
    16	Derived from `scripts/bench_10gbe.sh` (timing/generation/fresh-target
    17	patterns) plus the session's ad-hoc comparison methodology recorded in
    18	DEVLOG 2026-07-05 00:51 and DIAGNOSIS.md — the ad-hoc runner itself
    19	was never committed, so this reconstructs it durably.
    20	
    21	- **Matrix** (schema-identical to the committed
    22	  `tool_comparison.csv`: `transport,direction,workload,run,ms,status`):
    23	  blit / rsyncd / rsync-over-ssh / rclone-sftp × push/pull, and
    24	  blit / rsync / rclone / `cp -a` × local, over the baseline's three
    25	  workloads (1 GiB large, 10k×4 KiB small, 512 MiB+5k×2 KiB mixed).
    26	  rclone runs its best measured LAN config (`--ignore-checksum`,
    27	  tuned `--transfers`, sftp transport) per DIAGNOSIS.md. The harness
    28	  matrix and the plan's tripwire list are the same set by
    29	  construction (plan acceptance criterion).
    30	- **One command**: `DAEMON_HOST=… REMOTE_ROOT=… REMOTE_BLIT_DAEMON=…
    31	  ./scripts/bench_tripwires.sh`. By default it spins both daemons on
    32	  the target host over ssh — blitd via `--root` (exports the
    33	  per-invocation session dir as module `default`, no config file
    34	  needed) and rsyncd via a generated config — and tears both down plus
    35	  the session dir on exit. `SPIN_DAEMONS=0` targets already-running
    36	  daemons. All tools share one data root (session methodology).
    37	- **Fresh targets every run** (blit and rsync both no-op onto
    38	  already-delivered content): local dests recreated, remote push
    39	  targets are per-run never-seen subdirs; pull sources seeded once per
    40	  workload (seeding writes leave the ARC warm — baseline was warm
    41	  re-reads).
    42	- **Scale probe**: fixed 4 KiB files at counts crossing
    43	  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
    44	  25k→8, 50k→10 expected); records files/s and the **measured** stream
    45	  count (per-stream `stream complete` completion lines in blitd's
    46	  stderr, `data_plane.rs:224`, delta-counted per push). Measured-vs-
    47	  table divergence is exactly the sf-2 evidence the plan wants the
    48	  curve to show.
    49	- **Tripwire verdict is the exit code**: summary prints best-of per
    50	  cell, blit vs fastest rival; any rival win → `TRIPPED` + exit 3.
    51	  Also diffs blit cells against the committed baseline CSV (the ±10%
    52	  regression criterion) when present.
    53	- Missing tools (rsync/rclone locally or remotely) skip their cells
    54	  with a note; a wedged tool is capped by `timeout` and recorded in
    55	  the status column rather than hanging the run.
    56	
    57	## Files
    58	
    59	- `scripts/bench_tripwires.sh` (new, executable)
    60	
    61	## Tests
    62	
    63	Script-only slice — cargo suite unaffected (run anyway: fmt, clippy,
    64	full workspace suite green; count vs 1479 baseline in verdict file).
    65	Script verified by execution:
    66	
    67	- `bash -n` clean.
    68	- **Local-only e2e** (`SIZE_MB=32 SMALL_COUNT=500 RUNS=2 … matrix`):
    69	  all local cells timed, CSV written, summary + baseline diff printed,
    70	  exit 3 with `cp` tripping blit on tiny local copies (harness working
    71	  as designed; rig verdicts belong to sf-4).
    72	- **Full remote-path e2e over loopback** (ssh shim executing "remote"
    73	  commands locally; real network transfers to a real spun blitd +
    74	  rsyncd on 127.0.0.1): daemon spin-up, seeding, every push/pull cell,
    75	  scale probe with stream counting (200 files → 1 stream measured),
    76	  teardown verified (no stray daemons, session dir removed).
    77	  rclone-sftp cells recorded status 1 in this rig-less test (no sftp
    78	  auth to localhost) — the status column captured it and the run
    79	  continued, which is the designed failure path.
    80	
    81	## Known gaps
    82	
    83	- The scale probe is push-only (the plan's target cell); pull scaling
    84	  can be added when a pull-side per-stream log line exists.
    85	- Stream counting needs the daemon's stderr (`SPIN_DAEMONS=1` owns it;
    86	  otherwise `BLITD_LOG`); against a foreign daemon the column is empty
    87	  rather than guessed.
    88	- rclone-sftp cells assume ssh-agent/key auth to the host (same
    89	  requirement the session had); no rclone config file is generated.
    90	- Loopback e2e cannot validate 10 GbE-scale numbers — sf-4 is the rig
    91	  re-measure slice.
    92	- Observed during loopback testing, recorded for sf-2: a 1000-file
    93	  push rode 1 stream where the proposal table says 2 — consistent
    94	  with the DIAGNOSIS.md one-stream-for-10k-files gap; the daemon-side
    95	  proposal call (`control.rs:798`) and its input manifest need the
    96	  sf-2 pins.

exec
/usr/bin/zsh -lc 'rtk git diff 7202c1a''^ 7202c1a -- scripts/bench_tripwires.sh | sed -n '"'260,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc "rtk bash -lc 'git show 7202c1a:REVIEW.md | nl -ba | sed -n \"30,55p\"'" in /home/michael/dev/Blit
 succeeded in 0ms:
    30	| ue-r2-1c | `TransferEngine` shell + `TransferOrchestrator` as local adapter; local fast paths → engine strategies | `[x]` | `7730eb1`+`dc9b0ed`+`29e210b` + review fix `15e6334` |
    31	| ue-r2-1d | Streaming plan foundation (partial-scan InitialPlan/PlanUpdate); prove ~1s start; RELIABLE exceptions | `[x]` | `c08a5c1` + review fixes |
    32	| ue-r2-1e | Live cheap dials replace the `determine_remote_tuning` ladder | `[x]` | `3be9105`..`15968f4` + review fix `46da929` |
    33	| ue-r2-1f | Push converge through the engine; retire daemon `desired_streams` ladder | `[x]` | `a4a9f70` + review fix `0c8da50` |
    34	| ue-r2-1g | PullSync multistream through the engine (absorbs MULTISTREAM_PULL) | `[x]` | `48e583e` + review fix `4a2e58d` |
    35	| ue-r2-1h | Delete deprecated `Pull` RPC (+ its `pull_stream_count` ladder) after harvest; port relay onto PullSync | `[x]` | `2a13f53` (+`9f37a7a` baseline/staging-slip, `48c5a11` win-1) + review fix `f6f52d7` |
    36	| ue-r2-2 | Stream resize: negotiated `DataPlaneResize`/`Ack`, mid-transfer add/drop — **REV4 complete** | `[x]` | `042ca4b`..`0788e83` + review fix `ec4a3fe` |
    37	
    38	## Small-file ceiling (SMALL_FILE_CEILING) — code→GPT-review→fix loop
    39	
    40	Plan: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4).
    41	Same codex loop and record formats as the REV4 section above. Slice
    42	order and gates live in the plan (sf-6 is owner-gated on wire design;
    43	sf-3b… count is set by sf-3a's analysis, rows added as filed).
    44	
    45	| ID | Title | Status | Commit(s) |
    46	|----|-------|--------|-----------|
    47	| sf-1 | Tripwire + stream-scaling harness (`scripts/bench_tripwires.sh`) — baseline re-runnable in one command | `[~]` | |
    48	
    49	## Design-review queue (ratified D-2026-06-11-2, in execution order)
    50	
    51	Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
    52	`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (per-finding evidence).
    53	Coder loop: pick the topmost `[ ]` row. W2.3 requires a `docs/plan/` doc with
    54	**Status: Active** before code.
    55	

codex
`bash -n` only proves syntax; it does not catch the contract-level problems. I’m checking with shellcheck if available, but I’ll keep the verdict grounded in the script and plan evidence either way.
exec
/usr/bin/zsh -lc 'rtk shellcheck scripts/bench_tripwires.sh' in /home/michael/dev/Blit
 exited 127 in 0ms:
[rtk: No such file or directory (os error 2)]

codex
**Findings**

- `scripts/bench_tripwires.sh:126` — **High** — Teardown unconditionally `rm -rf`s `$SESSION`, but `$SESSION` is a timestamped path created with `mkdir -p` at line 134. A pre-existing or same-second colliding remote dir would be treated as owned and deleted, violating the “never rm anything it did not create” contract.

- `scripts/bench_tripwires.sh:317` — **High** — The verdict only considers successful rows and skipped/missing tools produce no failing condition. A run without required rsync/rclone/rsyncd coverage, or with failed rclone auth rows, can still exit 0 as “clean,” so the required full tripwire matrix is not enforced.

- `scripts/bench_tripwires.sh:236` — **Medium** — `SPIN_DAEMONS=0` cannot run rsyncd cells because both rsyncd push/pull require `RSYNCD_STARTED == 1`. That contradicts the advertised already-running-daemon mode and weakens “one command against any daemon host.”

- `scripts/bench_tripwires.sh:346` — **Medium** — The baseline diff only prints percentages; it does not fail or flag cells outside ±10%, and it only iterates current cells, so missing baseline cells are invisible. A blit regression can still exit 0 if no rival beats it.

- `.review/findings/sf-1-tripwire-harness.md:63` — **Low** — The record claims full workspace validation/count evidence “in verdict file,” but this commit adds no `.review/results/sf-1...` verdict/result file. The validation record is overstated.

- `REVIEW.md:47` — **Low** — The sf-1 row is `[~]` but the `Commit(s)` column is blank; it should name `7202c1a` for the review index to be accurate.

VERDICT: NEEDS FIXES
tokens used
126,219
**Findings**

- `scripts/bench_tripwires.sh:126` — **High** — Teardown unconditionally `rm -rf`s `$SESSION`, but `$SESSION` is a timestamped path created with `mkdir -p` at line 134. A pre-existing or same-second colliding remote dir would be treated as owned and deleted, violating the “never rm anything it did not create” contract.

- `scripts/bench_tripwires.sh:317` — **High** — The verdict only considers successful rows and skipped/missing tools produce no failing condition. A run without required rsync/rclone/rsyncd coverage, or with failed rclone auth rows, can still exit 0 as “clean,” so the required full tripwire matrix is not enforced.

- `scripts/bench_tripwires.sh:236` — **Medium** — `SPIN_DAEMONS=0` cannot run rsyncd cells because both rsyncd push/pull require `RSYNCD_STARTED == 1`. That contradicts the advertised already-running-daemon mode and weakens “one command against any daemon host.”

- `scripts/bench_tripwires.sh:346` — **Medium** — The baseline diff only prints percentages; it does not fail or flag cells outside ±10%, and it only iterates current cells, so missing baseline cells are invisible. A blit regression can still exit 0 if no rival beats it.

- `.review/findings/sf-1-tripwire-harness.md:63` — **Low** — The record claims full workspace validation/count evidence “in verdict file,” but this commit adds no `.review/results/sf-1...` verdict/result file. The validation record is overstated.

- `REVIEW.md:47` — **Low** — The sf-1 row is `[~]` but the `Commit(s)` column is blank; it should name `7202c1a` for the review index to be accurate.

VERDICT: NEEDS FIXES
