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
session id: 019f28c7-8e9c-7f92-b24e-c7e3f7f75bdd
--------
user
Review the diff range 63b813a..29e210b (run: git log --oneline 63b813a..29e210b and git diff 63b813a 29e210b; the three commits are 7730eb1 behavior pins, dc9b0ed engine move, 29e210b single-file accounting). It implements slice ue-r2-1c of docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md: introduce TransferEngine (new crates/blit-core/src/engine/ module), convert TransferOrchestrator into the local adapter, move the local fast paths (journal_no_work, no_work, tiny_manifest, single_huge_file, single-file shortcut) under engine-owned strategies preserving behavior, and add the perf-history/predictor accounting the single-file shortcut lacked. Slice record: .review/findings/ue-r2-1c.md. Check: (1) behavior preservation — the move must be verbatim except the documented deltas (sink construction moved ahead of planning; compare-mode translation unified into LocalCompareMode::resolve_* resolvers; single-file accounting added); diff the moved code against its origin in the parent commit to catch silent drift; (2) public API preservation for blit-app/blit-cli/blit-tui callers; (3) the single-file accounting: correct tag/lane (single_file vs null_sink RunKind derivation), predictor not updated on null-sink runs, tuning window not contaminated; (4) test count 1391 baseline -> 1394, moved tests still meaningful; (5) FAST/SIMPLE/RELIABLE and the REV4 slice acceptance criteria; (6) module/visibility hygiene of the new engine module. Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this as a code-review pass only, with no skill invocation and no edits. I’m going to ground in the repo guidance, then inspect the specified log/diff and compare the moved engine code against its parent where needed.
exec
/usr/bin/zsh -lc 'git log --oneline 63b813a..29e210b' in /home/michael/dev/Blit
 succeeded in 0ms:
29e210b ue-r2-1c: single-file strategy gains perf-history/predictor accounting
dc9b0ed ue-r2-1c: add TransferEngine; TransferOrchestrator becomes the local adapter
7730eb1 ue-r2-1c: pin NoWork strategy behavior before the engine move

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
/usr/bin/zsh -lc 'git diff --stat 63b813a 29e210b' in /home/michael/dev/Blit
 succeeded in 0ms:
 .review/findings/ue-r2-1c.md                       |  100 ++
 .../src/{orchestrator => engine}/history.rs        |   18 +-
 crates/blit-core/src/engine/journal.rs             |   72 +
 crates/blit-core/src/engine/mirror.rs              |  198 +++
 crates/blit-core/src/engine/mod.rs                 |  757 +++++++++
 .../src/{orchestrator => engine}/options.rs        |   51 +
 crates/blit-core/src/engine/single_file.rs         |  210 +++
 .../fast_path.rs => engine/strategy.rs}            |    0
 .../src/{orchestrator => engine}/summary.rs        |    0
 crates/blit-core/src/engine/tuning.rs              |  592 +++++++
 crates/blit-core/src/lib.rs                        |    1 +
 crates/blit-core/src/local_worker.rs               |    2 +-
 crates/blit-core/src/orchestrator/mod.rs           |   10 +-
 crates/blit-core/src/orchestrator/orchestrator.rs  | 1774 +-------------------
 crates/blit-core/tests/local_transfers.rs          |  118 +-
 crates/blit-daemon/src/service/pull.rs             |    4 +-
 16 files changed, 2164 insertions(+), 1743 deletions(-)

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/repo-guidance.md" in /home/michael/dev/Blit
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
  review finding per `.review/README.md`'s coder/reviewer loop.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo does not currently use `.agents/playbooks/` —
the `.review/` two-agent review loop and `docs/agent/PROTOCOL.md` already
cover that role for review-loop work.

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
- `gitea` — `http://q:3000/michael/blit_v2.git` (LAN mirror; pushed manually
  alongside or after `origin`, not auto-synced by any hook or CI job — it can
  lag `origin` by a commit or more at any given time).
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

codex
The repo guidance points current-state work through `docs/STATE.md`, so I’m reading the active plan/review documents as review context. After that I’ll focus on the engine move, public exports/callers, and the single-file accounting delta.
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-03 (`ue-r2-1b` complete — wire dial contract landed
via the code→review→fix loop) at commits `2741dc8`+`5bd345a`; unpushed to
`origin`: `fcf3345`+`2741dc8`+`5bd345a` (gitea has `fcf3345` already —
owner pushed it 2026-07-03; this handoff commits on top).

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **`ue-r2-1b` COMPLETE** — second REV4 slice through the GPT review loop.
  Wire dial contract defined before any behavior depends on it
  (`2741dc8`): `CapacityProfile` (rich receiver→sender profile, 0 =
  unknown = stay conservative) as `DataTransferNegotiation.
  receiver_capacity = 11` (push; daemon is receiver) and
  `TransferOperationSpec.receiver_capacity = 12` (pull_sync/delegated;
  client/dst is receiver) — **spec_version stays 2, deliberately no bump**
  (exact-match gate would make old daemons reject new clients; profile is
  a skippable hint); daemon-authoritative `resize_enabled = 12` +
  `epoch0_sub_token = 13`; capability bits `PushHeader.supports_stream_
  resize = 8` / `PeerCapabilities.supports_stream_resize = 5` (false until
  `ue-r2-2`); `DataPlaneResize`/`Ack` oneof variants in all four control
  streams (prior art `d9d4ec7`; its 11–14 clash resolved —
  min/max_stream_count subsumed by `CapacityProfile.max_streams`).
  Delegated dst override now also strips CLI-supplied `receiver_capacity`
  (R25-F2 boundary). Compat tests both mixed-version directions
  (`crates/blit-core/tests/proto_wire_compat.rs`, old-shape prost
  replicas). Review: codex/GPT-5.5 **PASS, zero findings**; supplementary
  4-lens self-review found 1 Low (false deprecated-Pull claim in a proto
  comment), fixed in `5bd345a`. Validation: fmt/clippy clean,
  `cargo test --workspace` **1391 / 0 / 2** (baseline 1378, +13).
- **Active context** (settled background for the slice work):
  - REV4 (`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`) is the **Active**
    convergence plan (D-2026-06-20-5); v1/REV2/REV3 Superseded.
  - Direction (D-2026-06-20-1/-2): one src/dst-agnostic sequencer for all
    four paths; one live dial replaces the three live stream-count ladders
    (`determine_remote_tuning`, `desired_streams`, `pull_stream_count`);
    **bounded-unilateral**, **no probe phase** (start within ~1s at
    conservative defaults, tune live), workload-shape-aware planner.
  - Process (D-2026-06-20-6, `docs/agent/GPT_REVIEW_LOOP.md`): for `ue-r2-*`
    slices Claude codes+commits each slice, `codex`/GPT-5.5 reviews it,
    Claude adjudicates + fixes. Per-slice commits to `master` ungated (no
    branches, never push); per-slice code acceptance owner-delegated. Owner
    gates remaining: **push**, **10 GbE sign-off**.

## Queue (ordered)

1. **`ue-r2-1c` (engine shell + local adapter)** — next REV4 slice: add
   `TransferEngine`, convert `TransferOrchestrator` into the local
   adapter, move the local fast paths (`journal_no_work`, `no_work`,
   `tiny_manifest`, `single_huge_file`, single-file shortcut) under
   engine-owned strategies, preserving behavior and adding accounting
   where the single-file shortcut lacked it. Engine-type recommendation
   (new `TransferEngine` + local adapter) is ratified at this slice —
   owner may override before it starts. Per D-2026-06-20-6 the loop may
   continue autonomously on owner "continue"; owner may push the
   `origin..master` stack first.
2. **Then** the rest of the REV4 slice list in order —
   `1d`/`1e`/`1f` → `1g` → `1h` → `ue-r2-2`
   (deps in REV4 §"Slice dependencies"), each through the GPT review loop.
3. **Design-review queue (independent, survives the convergence)** —
   `REVIEW.md` order governs. Highest open ratified row is **w4-1**
   (AbortOnDrop family, High) — now also owns the `ue-r2-1a` hard-abort gap.
   Then w4-3, W1 socket-policy / timeout constants. May fold into `ue-r2-1c`
   or fix standalone, owner's call.
4. **10 GbE benchmark session — DEFERRED** (owner 2026-06-12). The `ue-1`
   sign-off measure (loopback parity band: local↔local / local→daemon /
   daemon→local within a tight band) AND the `ue-2` (continuous/C) gate; also
   the zero-copy revisit gate (D-2026-06-12-1). Capture before/after
   baselines there. After `ue-1`: audit Round 1, TUI rework, H10b planner.
   **Test environment (owner, 2026-07-03)**: `admin@skippy:/mnt/generic-pool/video/test`
   — scp and ssh open from this user to `admin@skippy`; if a daemon needs
   to run on skippy and can't, ping the owner. (BENCHMARK_10GBE_PLAN.md is
   `Status: Historical`; the environment note lives here until a live
   benchmark doc exists.)

## Authoritative docs right now

- **Active plan: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** — convergence
  engine (D-2026-06-20-5); absorbs `MULTISTREAM_PULL.md` as slice `ue-r2-1g`.
- Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
  `…_REV2.md`, `…_REV3.md`.
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active, D-2026-06-20-6) governs
  `ue-r2-*`; `.review/README.md` async loop governs other work.
- Review loop: `REVIEW.md` (`ue-r2-1a`/`ue-r2-1b` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (delete ratified D-2026-06-12-1,
  executes w8-1), `TUI_REWORK.md` (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (`ue-1`/`ue-2` gate).
- Decisions: D-2026-06-20-1 (direction), -5 (REV4 Active), -6 (review loop).

## Blocked / waiting

- **Owner**: (1) "continue" → I pick up `ue-r2-1c` (or push the
  `origin..master` stack — `fcf3345`+`2741dc8`+`5bd345a` — first; gitea
  additionally lacks the last two). Doesn't block autonomous continuation
  per D-2026-06-20-6. (2) RESOLVED since last handoff: `439a2a7` (Windows
  test-tuning) is now on `origin` — the Windows-CI push blocker is gone.

## Open questions

- **(OPEN)** Edit D-2026-06-20-1 to strip its superseded warmup/size-gate
  wording? Owner: not sure. (Agent rec: edit with a one-line note → -2/-5.)
- **(OPEN)** Historical audit/finding docs (`audit-13/14/15`, `drift-*`)
  still embed `/Users/...` in recorded evidence — scrub, or leave as
  evidence? Agent rec: leave; live docs are already clean.
- **(RESOLVED 2026-07-03)** Adaptive-streams prior art consumed:
  `ue-r2-1b` harvested the `d9d4ec7` proto contract (resize messages kept
  near-verbatim; its negotiation fields 11–14 renumbered/subsumed — see
  the `NOTE on field numbers` block in `proto/blit.proto`). The branch
  `origin/feat/adaptive-streams-pr3-resizable` has served its purpose for
  the wire slice; still referenced as prior art for `ue-r2-2`'s
  controller/dialer wiring. (Full audit history: DEVLOG 2026-06-21 /
  STATE history at `2c1b839`.)
- **Engine type** — agent recommends a new `TransferEngine` + local adapter;
  ratified at `ue-r2-1c`, owner may override.
- **Windows**: w9-1/w9-5/w9-4/w4-2 added ungated daemon-spawn tests,
  unverified on Windows; `439a2a7` is now on origin, so the next
  windows-latest CI run is meaningful — triage real failures into findings.

## Handoff log (newest first, keep ≤ 3)

- **2026-07-03** @ `5bd345a` — `ue-r2-1b` landed end-to-end through the
  code→GPT-review→fix loop (wire contract `2741dc8`; codex PASS zero
  findings; 1 Low self-review finding fixed `5bd345a`). fmt/clippy clean;
  tests 1391/0/2. All on master; unpushed to origin:
  `fcf3345`+`2741dc8`+`5bd345a`. In-flight: none — paused at a slice
  boundary. **Exact first action next session**: on owner "continue",
  start `ue-r2-1c` (engine shell + local adapter) through the loop —
  ratifying the engine-type recommendation unless the owner overrides;
  else owner pushes the stack / decides the D-2026-06-20-1 edit.
- **2026-06-21** @ `2c1b839` — `ue-r2-1a` landed end-to-end through the
  code→GPT-review→fix loop (substrate cherry-pick `e569eea`/`3844a15`/
  `ec561f2`, conflict resolved, tests `771a632`, codex review → 4 findings
  all fixed `90ed43d`, docs `2c1b839`). fmt/clippy clean; test 1378/0/2.
- **2026-06-20** @ `09268eb` — reviewed all three unified-transfer candidates,
  produced REV4 (code-reality corrected; REV3's "two static tables" ladder
  claim was wrong — all three ladders live), and on owner's "rev4 replaces
  v1" recorded **D-2026-06-20-5** + propagated (REV4 Active; v1/REV2/REV3
  Superseded). Then established the GPT review loop (D-2026-06-20-6) and
  ported then removed SETUP.md (folded into governance).

exec
/usr/bin/zsh -lc "sed -n '1,240p' .review/findings/ue-r2-1c.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# ue-r2-1c: TransferEngine shell + TransferOrchestrator as local adapter

**Slice**: ue-r2-1c — third slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded; under GPT review (`docs/agent/GPT_REVIEW_LOOP.md`)
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: `7730eb1` (behavior pins), `dc9b0ed` (engine move),
`29e210b` (single-file accounting)

## What

Introduce `TransferEngine` (new `crates/blit-core/src/engine/` module) and
convert `TransferOrchestrator` into the local adapter, per REV4 Design §1
(engine type ratified at this slice: new engine + adapter, not an in-place
rename). Local fast paths become engine-owned strategies with common
accounting: `journal_no_work`, `no_work`, `tiny_manifest`,
`single_huge_file`, the single-file shortcut, and the streaming pipeline.
The single intentional behavior addition: the single-file shortcut gains
the perf-history/predictor accounting it lacks today (REV4 Design §2).

## Approach (move plan)

Everything is within blit-core, so the engine/adapter seam at this slice
is module organization + ownership, not dependency inversion (traits
arrive when push/pull converge at `ue-r2-1f`/`1g` and only where needed).

- **`crates/blit-core/src/engine/`** (new; re-exported from `lib.rs`):
  - `mod.rs` — `TransferEngine` + `EngineRequest { src_root, dest_root,
    source: Arc<dyn TransferSource>, sink: Arc<dyn TransferSink>,
    options }` + `execute()`: owns strategy selection order (single-file
    → journal probe → fast-path walk → streaming), dispatch, and the
    streaming leg (tuning window → scan/collect → plan → pipeline →
    mirror deletions → journal checkpoints → history/predictor).
  - `strategy.rs` — `FastPathDecision`/`FastPathOutcome`/
    `maybe_select_fast_path` moved whole from `orchestrator/fast_path.rs`
    (tests move with it).
  - `single_file.rs` — `execute_single_file_copy` moved from
    `orchestrator.rs:1138`, **plus new accounting**: every return path
    records perf history (tag `single_file`) and updates the predictor
    (skipped for null_sink, same rule as streaming at
    `orchestrator.rs:863`). Records with `tar_shard_tasks == 0` are
    already excluded from the tuning window (`orchestrator.rs:72`), so
    the new tag cannot contaminate auto-tuning.
  - `options.rs`, `summary.rs`, `history.rs` — moved from
    `orchestrator/` unchanged (names kept; generalizing the option type
    is 1f/1g work). `LocalCompareMode` gains two small resolvers
    (`resolve_comparison_mode`, `resolve_compare_snapshot`) replacing
    the three duplicated match blocks (`orchestrator.rs:467/:520/:1159`).
  - `tuning.rs` — `select_tuning_window`/`select_tuning_window_from_history`
    + `TUNING_WINDOW_SIZE` + their 12 tests, moved from `orchestrator.rs`.
  - `mirror.rs` — `apply_mirror_deletions`; `journal.rs` —
    journal probe + `persist_journal_checkpoints` + `log_probe`.
- **`crates/blit-core/src/orchestrator/`** shrinks to the adapter:
  `TransferOrchestrator::{new, default}`, the sync runtime wrapper
  (unchanged), and an async method that checks preconditions (src
  exists, create dest parent), constructs local `FsTransferSource`/
  `FilteredSource` + `FsTransferSink`/`NullSink` (translation of
  compare-mode via the new resolver), builds the `EngineRequest`, and
  calls `TransferEngine::execute`. `orchestrator/mod.rs` keeps the
  existing six public names via `pub use crate::engine::...` so every
  external caller (blit-app `transfers/local.rs:36-57`, blit-cli,
  blit-tui, tests) compiles unchanged.
- Sink construction moves ahead of planning (adapter builds it up
  front). `FsTransferSink::new` is pure state (paths + config), so
  constructing it on runs that end in a fast path is behavior-neutral.

## Behavior pins added BEFORE the move (commit 1)

The test-inventory pass found these currently unpinned; each is cheap
and pins a strategy this slice relocates:

- empty source dir → `FastPathDecision::NoWork{examined:0}` →
  `TransferOutcome::SourceEmpty`.
- all-up-to-date second run (dir, skip_unchanged) →
  `NoWork{examined>0}` → `UpToDate`, perf-history tag `no_work`.
- (with commit 3) single-file run records history tag `single_file` —
  the new accounting's own guard.

Not pinnable here: `single_huge_file` (needs a ≥1 GiB file) and
`journal_no_work` (needs journal-capable FS state) — unchanged code
moves, existing Known gaps.

## Files changed

- `crates/blit-core/src/engine/` (new module, re-exported in `lib.rs`):
  `mod.rs` (TransferEngine + EngineRequest + execute — moved body of
  `execute_local_mirror_async`), `strategy.rs` (was
  `orchestrator/fast_path.rs`), `single_file.rs` (moved + accounting),
  `tuning.rs` (moved tuning-window helpers + 12 tests), `mirror.rs`
  (moved `apply_mirror_deletions`), `journal.rs` (moved checkpoint
  helpers), `options.rs` (+ the two compare-mode resolvers),
  `summary.rs`, `history.rs` (moved; snapshot fn delegates to the
  resolver).
- `crates/blit-core/src/orchestrator/orchestrator.rs` — rewritten as
  the local adapter (preconditions, source/sink construction, option
  translation, EngineRequest handoff); public-API tests kept in place.
- `crates/blit-core/src/orchestrator/mod.rs` — re-exports the six
  public names from `crate::engine` (external callers unchanged).
- `crates/blit-core/tests/local_transfers.rs` — 3 new tests (2 pins +
  1 accounting guard).
- Comment-path touch-ups: `local_worker.rs`, `blit-daemon/service/pull.rs`.

## Tests

Baseline entering the slice: 1391 / 0 / 2 → after: **1394 / 0 / 2**
(+2 NoWork pins, +1 single-file accounting guard; every moved test —
strategy, tuning-window, history, public-API — still runs and passes).
Accounting guard proven non-vacuous: fails with the accounting
reverted, passes restored.

## Known gaps

- `single_huge_file` and `journal_no_work` strategies move without new
  coverage (pre-existing gap; needs 1 GiB fixtures / journal-capable FS).
- The engine's option/summary types keep their local names
  (`LocalMirrorOptions`/`LocalMirrorSummary`) until push/pull converge
  (`ue-r2-1f`/`1g`) — renaming now would churn every caller twice.
- Dial creation, payload-queue ownership, and progress/telemetry wiring
  stay where they are until `ue-r2-1d`/`1e` (engine owns them per REV4,
  arriving with those slices).

exec
/usr/bin/zsh -lc 'rg -n "ue-r2-1c|R2-1c|TransferEngine|single-file|single_file|FAST|SIMPLE|RELIABLE|acceptance|Acceptance" docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md' in /home/michael/dev/Blit
 succeeded in 0ms:
13:stream-ladder acceptance criterion **corrected against the actual code**
41:be complex underneath, but the behavioral contract must be FAST, SIMPLE,
42:and RELIABLE.
77:- **FAST**: bytes begin quickly, stream scheduling avoids slow-sink
80:- **SIMPLE**: there is one transfer behavior owner. Local fast paths,
83:- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
92:- **Every stage serves FAST, SIMPLE, or RELIABLE** — a purely structural
95:## Acceptance Criteria
97:- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
102:      `single_huge_file`, and the single-file copy shortcut at
132:      the owner instead of silently weakening RELIABLE.
245:Introduce a new `TransferEngine` rather than renaming
256:  `single_huge_file`, the single-file shortcut, streaming pipeline);
268:agent recommends the new `TransferEngine` + local adapter above, not an
269:in-place rename. Ratified at the `ue-r2-1c` slice; owner may override.
274:would accidentally delete useful FAST behavior. REV4 keeps the REV2/REV3
278:`single_huge_file`, plus the single-file copy shortcut
282:This preserves SIMPLE for the operator (one command model, one behavior
283:owner) and FAST (the low-overhead tiny/single-file paths stay where they
316:resume, checksum-refusal) named in the Acceptance Criteria.
342:wire-up rather than a retrofit (see the C-ready acceptance criterion).
359:Exact field names and numbers are part of the wire slice acceptance
401:  explicit, tested, reported RELIABLE exception — not a silent miss.
407:- **RELIABLE-exception loophole.** Allowing tested first-byte exceptions
408:  risks exceptions proliferating until RELIABLE is silently eroded.
417:  Mitigation: the acceptance criterion now enumerates all three plus the
440:3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
443:   `single_huge_file`, single-file shortcut) under engine-owned
445:   single-file shortcut lacked it.
449:   document any RELIABLE exception that cannot safely move bytes before
466:   `MULTISTREAM_PULL.md` acceptance criteria.
487:- `1d` (streaming plan) blocks the 1s-start acceptance and feeds `1g`
506:  (`journal_no_work`, not `journal_skip`; no distinct `single_file`
523:  acceptance criterion; REV3 restored both; REV4 keeps them and adds the
535:  tuning-strategy choice, not an exception. Recorded in Acceptance

exec
/usr/bin/zsh -lc 'git diff --name-status --find-renames 63b813a 29e210b' in /home/michael/dev/Blit
 succeeded in 0ms:
A	.review/findings/ue-r2-1c.md
R092	crates/blit-core/src/orchestrator/history.rs	crates/blit-core/src/engine/history.rs
A	crates/blit-core/src/engine/journal.rs
A	crates/blit-core/src/engine/mirror.rs
A	crates/blit-core/src/engine/mod.rs
R066	crates/blit-core/src/orchestrator/options.rs	crates/blit-core/src/engine/options.rs
A	crates/blit-core/src/engine/single_file.rs
R100	crates/blit-core/src/orchestrator/fast_path.rs	crates/blit-core/src/engine/strategy.rs
R100	crates/blit-core/src/orchestrator/summary.rs	crates/blit-core/src/engine/summary.rs
A	crates/blit-core/src/engine/tuning.rs
M	crates/blit-core/src/lib.rs
M	crates/blit-core/src/local_worker.rs
M	crates/blit-core/src/orchestrator/mod.rs
M	crates/blit-core/src/orchestrator/orchestrator.rs
M	crates/blit-core/tests/local_transfers.rs
M	crates/blit-daemon/src/service/pull.rs

codex
The slice record claims the main intentional drifts are sink construction order, compare-mode resolver consolidation, and single-file accounting. I’m now comparing the moved bodies against `63b813a` to separate those deltas from accidental behavior changes.
exec
/usr/bin/zsh -lc "sed -n '35,135p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
## Goal

Keep the v1 direction: converge the transfer subsystem around one
src/dst-agnostic engine, one live dial, and the already-shared
byte-moving leaf. The operator should experience one simple transfer
model regardless of where the command is issued. The implementation may
be complex underneath, but the behavioral contract must be FAST, SIMPLE,
and RELIABLE.

REV4 keeps convergence, not rebuild. It tightens the plan where review
found that v1 compressed too much into one slice or left compatibility
implicit, and it corrects the code-reality errors that crept into REV2/
REV3:

- The first-byte-within-about-1s requirement is a real architecture
  change and gets its own streaming-plan slice.
- Existing local fast paths are preserved as engine-owned strategies
  unless the owner later decides to delete one; they must not remain
  side doors around the engine.
- Work-stealing is treated as a scheduling behavior change, not as
  "substrate only".
- Capacity profile and resize wire shape are designed before code that
  depends on them.
- Pull parity is measured only after PullSync is actually multistream.
- The stream-count ladders the engine must subsume are enumerated
  **accurately** (REV3 under-counted them — see Current Code Reality).

## Non-goals

- No ground-up transfer rewrite.
- No zero-copy receive revival (D-2026-06-12-1; revisit gated on the
  10 GbE benchmarks).
- No H10b merger. The engine's workload-shape-aware planner and 1s start
  requirement stand on their own; D-2026-06-04-3 remains queued after
  audit Round 1.
- The **gRPC fallback path stays single-logical-stream by design**
  (unchanged from w2-3's non-goal). "Pull is single-stream today" below
  is about PullSync's TCP data plane, not this fallback.
- No coding during this review.

## Constraints

- **FAST**: bytes begin quickly, stream scheduling avoids slow-sink
  head-of-line blocking, tuning comes from measured telemetry, and small
  local transfers keep their low-overhead path.
- **SIMPLE**: there is one transfer behavior owner. Local fast paths,
  push negotiation, pull sync, and delegated transfers become strategies
  or inputs under the engine, not separate operator-visible models.
- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
  StallGuard behavior, cancellation, byte-progress accounting, and
  byte-identical transfer tests cannot regress.
- Wire changes are allowed (proto unfrozen, D-2026-06-11-1), but mixed
  old/new peers must negotiate down to today's behavior. New fields are
  advisory until both peers advertise support.
- The 1370-test baseline must not drop.
- Windows parity remains required unless a test is genuinely platform
  specific.
- **Every stage serves FAST, SIMPLE, or RELIABLE** — a purely structural
  change with no goal payoff is out.

## Acceptance Criteria

- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
      public transfer sequencer for local↔local, push, pull, and
      delegated daemon↔daemon transfers.
- [ ] Existing local fast paths are either engine-owned strategies
      (their real names: `journal_no_work`, `no_work`, `tiny_manifest`,
      `single_huge_file`, and the single-file copy shortcut at
      `orchestrator.rs:178`) or explicitly deleted by owner decision. No
      local path bypasses the transfer behavior owner by accident.
- [ ] **The three static code-level stream/dial ladders plus the
      negotiated proto field are replaced by one dial source** (corrected
      against code — see Current Code Reality). Concretely, the dial
      subsumes:
      1. `remote/tuning.rs::determine_remote_tuning` (size-keyed
         `initial_streams`/`max_streams`/`chunk_bytes`/`tcp_buffer_size`/
         `prefetch_count`; the *client's* ladder, consumed by push and by
         the daemon pull paths);
      2. `blit-daemon .../push/control.rs::desired_streams` (the daemon
         push-negotiation ladder, already keyed on **file count** as well
         as bytes — the daemon's ladder "wins" per `tuning.rs`'s own doc
         comment);
      3. `blit-daemon .../pull.rs::pull_stream_count` (the deprecated
         Pull RPC ladder, byte-keyed, capped by `tuning.max_streams`);
      and the negotiated `DataTransferNegotiation.stream_count` (proto
      field 4) those ladders feed onto the wire. After convergence no
      static size→streams table remains in any path.
- [ ] The engine starts transfer work within about 1 second without a
      probe-then-go phase. This holds for **both** novel workloads (no
      telemetry extant — start copying immediately at conservative
      defaults and tune live from the first byte) **and** known workloads
      (telemetry extant — replay the last run if it was optimal, else
      recalculate onto the live-tune path). Novel vs known is a
      tuning-strategy choice, not an exception. The only exceptions are
      modes where moving any byte before full knowledge would itself be
      unsafe — mirror/delete (scan-completeness), resume, and
      checksum-refusal — and those are explicit, tested, and reported to
      the owner instead of silently weakening RELIABLE.
- [ ] The planner is workload-shape-aware (file count and per-file
      overhead, not bytes alone) and can emit an initial safe work batch
      from partial enumeration, then refine as more headers arrive. Note

exec
/usr/bin/zsh -lc "sed -n '235,290p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
- `data_plane.rs` carries `StallGuard`/`StallGuardWriter`
  (`remote/transfer/stall_guard.rs:75/139`) today; the `Probe` generic
  (`DataPlaneSession<P: Probe = NoProbe>`) does **not** exist at HEAD —
  it arrives with the PR1 salvage commit (`e6ef095`). The hand-resolved
  conflict at salvage is `StallGuardWriter` vs the `Probe` generic.

## Design

### 1. Engine ownership

Introduce a new `TransferEngine` rather than renaming
`TransferOrchestrator` in place (it is local-shaped: own runtime,
`LocalMirrorOptions`). `TransferOrchestrator` becomes the local adapter
that constructs local `Source`, `Sink`, options, and local strategy
inputs, then calls the engine. Push, PullSync, and delegated transfers
call the same engine with different source/sink and negotiation
adapters.

The engine owns:

- strategy selection (`journal_no_work`, `no_work`, `tiny_manifest`,
  `single_huge_file`, the single-file shortcut, streaming pipeline);
- dial creation and updates (subsuming all three ladders + local
  parallelism);
- payload work queue;
- progress and telemetry wiring;
- invariants around first work, fallback, cancellation, and finish.

Path-specific code remains only at boundaries where the protocol
differs: path resolution, module authorization, source/destination
manifest exchange, mirror/delete authority, and legacy compatibility.

**Engine type (the q3 open question, owner-deferred to the agent):** the
agent recommends the new `TransferEngine` + local adapter above, not an
in-place rename. Ratified at the `ue-r2-1c` slice; owner may override.

### 2. Local fast paths are strategies, not bypasses

V1 said "no separate small-transfer path"; review found that wording
would accidentally delete useful FAST behavior. REV4 keeps the REV2/REV3
rule: the small/huge/no-op local paths may remain, but only as
engine-owned strategies with common accounting and tests. Their real
names are `journal_no_work`, `no_work`, `tiny_manifest`,
`single_huge_file`, plus the single-file copy shortcut
(`orchestrator.rs:178`, which currently bypasses history recording —
folding it under the engine gives it accounting it lacks today).

This preserves SIMPLE for the operator (one command model, one behavior
owner) and FAST (the low-overhead tiny/single-file paths stay where they
beat the full pipeline).

### 3. Streaming plan foundation

The 1s start requirement cannot be hidden inside the sequencer-converge
slice. Today the local path collects all headers before planning, and
PullSync waits for complete client and server manifests before transfer.

exec
/usr/bin/zsh -lc "sed -n '430,455p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
   `d9d4ec7` (PR3 WIP, does not build). Resolve `data_plane.rs`
   `StallGuardWriter`-vs-`Probe`. Treat work-stealing as behavior, not
   inert substrate: add/keep slow-sink, failing-sink, cancellation,
   byte-accounting, StallGuard, and byte-identical tests. The elastic
   work-stealing stream-set exists from this slice onward (C-ready seam).
2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
   capability, and resize proto shape (`receiver_capacity = 11`,
   `DataPlaneResize`/`Ack`). Add compatibility tests for old client/new
   daemon and new client/old daemon. No behavior depends on these fields
   until this slice is green.
3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
   convert `TransferOrchestrator` into a local adapter. Move the local
   fast paths (`journal_no_work`, `no_work`, `tiny_manifest`,
   `single_huge_file`, single-file shortcut) under engine-owned
   strategies, preserving behavior and adding accounting where the
   single-file shortcut lacked it.
4. **`ue-r2-1d-streaming-plan-foundation`** — Introduce partial-scan
   initial plans and plan updates (novel vs known per Design §3). Prove
   first-byte / first-useful-work timing for local and push shapes, and
   document any RELIABLE exception that cannot safely move bytes before
   complete knowledge.
5. **`ue-r2-1e-live-cheap-dials`** — Replace the static
   `determine_remote_tuning` chunk/prefetch/TCP-buffer ladder with the
   single mutable dial. Start conservative within the receiver profile,
   then adjust cheap dials from PR1 telemetry. The dial is a mutable
   object read by both ends from this slice onward (C-ready seam).

exec
/usr/bin/zsh -lc "sed -n '500,538p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
## Review Findings Rolled In

- `ue-1c` was too large: split streaming-plan foundation, local adapter,
  push convergence, and pull convergence into separate slices.
- Local fast paths conflicted with "no separate small-transfer path":
  redefined them as engine strategies, **with their real names**
  (`journal_no_work`, not `journal_skip`; no distinct `single_file`
  strategy — it is a copy shortcut at `orchestrator.rs:178`).
- "No behavior change" on salvage was inaccurate for work-stealing:
  tests now treat scheduling as observable.
- Proto compatibility was too implicit: wire shape is now a first-class
  early slice (`receiver_capacity = 11` grounded against the proto).
- Pull parity gate was too early: moved until after PullSync multistream.
- **REV3's "two static tables, not three" correction was itself wrong
  (corrected in REV4).** All three ladders are live —
  `determine_remote_tuning`, `desired_streams` (control.rs:476),
  `pull_stream_count` (pull.rs:904) — and `tuning.rs`'s own doc comment
  confirms the daemon "runs its own ladder and wins". REV3 also said
  `determine_remote_tuning` drives "local and push"; it drives push +
  daemon pull, **not** local (local has no caller). And REV3's
  "pull is single-stream via force_grpc, not a third ladder" conflated
  the deprecated multistream `Pull` RPC with single-stream PullSync.
- REV2 dropped the Risks section and the "C-ready by construction"
  acceptance criterion; REV3 restored both; REV4 keeps them and adds the
  "under-converged ladders" risk.
- `DECISIONS.md` D-2026-06-20-1 still carries superseded warmup/size-gate
  language; D-2026-06-20-2 and this Draft carry the live-from-first-byte
  correction. If REV4 is accepted, either edit D-2026-06-20-1 with a note
  or add a superseding cleanup decision.

## Open Questions for Final Owner Decision

- **(RESOLVED 2026-06-20, owner)** First-byte timing: hard invariant for
  every mode *except* the unsafe-before-full-knowledge modes
  (mirror/delete, resume, checksum-refusal). Novel vs known workload is a
  tuning-strategy choice, not an exception. Recorded in Acceptance
  Criteria and Design §3.
- **(RESOLVED 2026-06-20, owner)** Deprecated `Pull` deletion stays
  in-plan as `ue-r2-1h`, sequenced last, gated on `ue-r2-1g` + the

codex
The plan confirms this slice should still be local-only structurally: shared engine ownership starts here, while push/pull convergence and live dials are later slices. I’m checking for accidental changes in the local behavior surface rather than expecting full REV4 convergence yet.
exec
/usr/bin/zsh -lc 'git status --short --branch' in /home/michael/dev/Blit
 succeeded in 0ms:
## master...origin/master [ahead 8]
 M docs/STATE.md
?? .review/results/ue-r2-1c.codex.md
?? .review/results/ue-r2-1c.codex.retry.md

exec
/usr/bin/zsh -lc 'git diff --find-renames --color=never 63b813a 29e210b -- crates/blit-core/src/engine/single_file.rs crates/blit-core/src/engine/options.rs crates/blit-core/src/engine/history.rs' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git diff --find-renames --color=never 63b813a 29e210b -- crates/blit-core/src/orchestrator/orchestrator.rs crates/blit-core/src/engine/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/engine/history.rs b/crates/blit-core/src/engine/history.rs
new file mode 100644
index 0000000..eb7929a
--- /dev/null
+++ b/crates/blit-core/src/engine/history.rs
@@ -0,0 +1,210 @@
+use crate::perf_history::{
+    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
+};
+use crate::perf_predictor::PerformancePredictor;
+
+use super::{LocalMirrorOptions, LocalMirrorSummary};
+
+/// Map the orchestrator's `LocalCompareMode` onto the perf-history
+/// snapshot enum so tuning records preserve the user's full intent
+/// (not just `checksum: bool`).
+fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
+    options
+        .compare_mode
+        .resolve_compare_snapshot(options.checksum)
+}
+
+pub(super) fn record_performance_history(
+    summary: &LocalMirrorSummary,
+    options: &LocalMirrorOptions,
+    fast_path: Option<&str>,
+    planner_duration_ms: u128,
+    transfer_duration_ms: u128,
+) -> Option<PerformanceRecord> {
+    if !options.perf_history {
+        return None;
+    }
+
+    let record = build_performance_record(
+        summary,
+        options,
+        fast_path,
+        planner_duration_ms,
+        transfer_duration_ms,
+    );
+
+    if let Err(err) = append_local_record(&record) {
+        if options.verbose {
+            eprintln!("Failed to update performance history: {err:?}");
+        }
+    }
+    Some(record)
+}
+
+/// Construct the `PerformanceRecord` from a summary without
+/// touching disk. Split out from `record_performance_history` so
+/// the record-shape contract — specifically R44-F1's "train and
+/// query against the same feature vector" invariant — is
+/// unit-testable without writing to the global perf history file.
+fn build_performance_record(
+    summary: &LocalMirrorSummary,
+    options: &LocalMirrorOptions,
+    fast_path: Option<&str>,
+    planner_duration_ms: u128,
+    transfer_duration_ms: u128,
+) -> PerformanceRecord {
+    let options_snapshot = OptionSnapshot {
+        dry_run: options.dry_run,
+        preserve_symlinks: options.preserve_symlinks,
+        include_symlinks: options.include_symlinks,
+        skip_unchanged: options.skip_unchanged,
+        checksum: options.checksum,
+        compare_mode: snapshot_compare_mode(options),
+        workers: options.workers,
+    };
+
+    let mode = if options.mirror {
+        TransferMode::Mirror
+    } else {
+        TransferMode::Copy
+    };
+
+    // R44-F1: train against scanned features so the predictor's
+    // training inputs match its query inputs. The orchestrator
+    // queries `predict(...)` with `all_headers.len()` (scanned
+    // count) and `total_bytes` (scanned bytes); pre-fix the record
+    // was populated with `summary.copied_files`, so the predictor
+    // saw a different feature vector at training time than at
+    // query time, and predictions drifted on every incremental
+    // workload. The `total_bytes` field on the record was already
+    // scanned-bytes by accident; this aligns both axes deliberately.
+    //
+    // `summary.copied_files` and the per-bucket counts
+    // (tar_shard_files / raw_bundle_files / large_tasks) still
+    // reflect actual writes — they're the load-bearing inputs for
+    // `derive_local_plan_tuning`'s bucket-target heuristics, which
+    // are computed from observed apply behavior, not scan size.
+    let mut record = PerformanceRecord::new(
+        mode,
+        None,
+        None,
+        summary.scanned_files,
+        summary.scanned_bytes,
+        options_snapshot,
+        fast_path.map(|s| s.to_string()),
+        planner_duration_ms,
+        transfer_duration_ms,
+        0,
+        0,
+    );
+    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
+    record.tar_shard_files = summary.tar_shard_files as u32;
+    record.tar_shard_bytes = summary.tar_shard_bytes;
+    record.raw_bundle_tasks = summary.raw_bundle_tasks as u32;
+    record.raw_bundle_files = summary.raw_bundle_files as u32;
+    record.raw_bundle_bytes = summary.raw_bundle_bytes;
+    record.large_tasks = summary.large_tasks as u32;
+    record.large_bytes = summary.large_bytes;
+
+    record
+}
+
+pub(super) fn update_predictor(
+    predictor: &mut Option<PerformancePredictor>,
+    record: &PerformanceRecord,
+    verbose: bool,
+) {
+    if let Some(ref mut predictor) = predictor {
+        predictor.observe(record);
+        if let Err(err) = predictor.save() {
+            if verbose {
+                eprintln!("Failed to persist predictor state: {err:?}");
+            }
+        }
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::orchestrator::TransferOutcome;
+    use std::time::Duration;
+
+    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
+        LocalMirrorOptions {
+            mirror,
+            ..LocalMirrorOptions::default()
+        }
+    }
+
+    /// R44-F1 contract: the record's `(file_count, total_bytes)`
+    /// must mirror the orchestrator's predictor-query features.
+    /// Pre-fix this assertion would have failed: the record was
+    /// populated from `summary.copied_files` and `summary.total_bytes`
+    /// while the query used scanned values, so on this incremental
+    /// scenario (1000 scanned, 5 actually written) the predictor
+    /// trained on (5, 100KB) but was queried with
+    /// (1000, ~10MB).
+    #[test]
+    fn record_uses_scanned_features_not_copied() {
+        let summary = LocalMirrorSummary {
+            // Mostly-unchanged incremental run: 1000 files scanned,
+            // only 5 actually written.
+            scanned_files: 1000,
+            scanned_bytes: 10 * 1024 * 1024,
+            planned_files: 5,
+            copied_files: 5,
+            total_bytes: 100 * 1024,
+            duration: Duration::from_millis(200),
+            outcome: TransferOutcome::Transferred,
+            ..LocalMirrorSummary::default()
+        };
+        let options = options_with_mirror(false);
+        let record = build_performance_record(&summary, &options, Some("streaming"), 150, 50);
+
+        assert_eq!(
+            record.file_count, 1000,
+            "record.file_count must reflect scanned (planner-side) workload, not copied count"
+        );
+        assert_eq!(
+            record.total_bytes, summary.scanned_bytes,
+            "record.total_bytes must reflect scanned bytes, not transferred bytes"
+        );
+        assert_eq!(record.planner_duration_ms, 150);
+        assert_eq!(record.transfer_duration_ms, 50);
+    }
+
+    /// Bucket-shape fields (tar_shard_*, raw_bundle_*, large_*)
+    /// must continue to reflect actual write activity — they feed
+    /// `derive_local_plan_tuning` which heuristically sizes
+    /// destination buckets from past apply behavior.
+    #[test]
+    fn bucket_counts_still_reflect_actual_writes() {
+        let summary = LocalMirrorSummary {
+            scanned_files: 100,
+            scanned_bytes: 1_000_000,
+            copied_files: 10,
+            total_bytes: 50_000,
+            tar_shard_tasks: 2,
+            tar_shard_files: 7,
+            tar_shard_bytes: 30_000,
+            raw_bundle_tasks: 1,
+            raw_bundle_files: 2,
+            raw_bundle_bytes: 15_000,
+            large_tasks: 1,
+            large_bytes: 5_000,
+            ..LocalMirrorSummary::default()
+        };
+        let options = options_with_mirror(true);
+        let record = build_performance_record(&summary, &options, Some("streaming"), 30, 70);
+
+        assert_eq!(record.tar_shard_tasks, 2);
+        assert_eq!(record.tar_shard_files, 7);
+        assert_eq!(record.tar_shard_bytes, 30_000);
+        assert_eq!(record.raw_bundle_tasks, 1);
+        assert_eq!(record.raw_bundle_files, 2);
+        assert_eq!(record.raw_bundle_bytes, 15_000);
+        assert_eq!(record.large_tasks, 1);
+        assert_eq!(record.large_bytes, 5_000);
+    }
+}
diff --git a/crates/blit-core/src/engine/options.rs b/crates/blit-core/src/engine/options.rs
new file mode 100644
index 0000000..fa257ec
--- /dev/null
+++ b/crates/blit-core/src/engine/options.rs
@@ -0,0 +1,157 @@
+use crate::fs_enum::FileFilter;
+
+/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
+/// (FilteredSubset / All) plus a `false`/`true` flag form. R58-F6:
+/// pre-fix, local mirror had no plumbing for this — `apply_mirror_deletions`
+/// always operated on whatever the transfer filter let through. The
+/// remote pull path already supports both modes via
+/// `PullSyncOptions.delete_all_scope`; this brings local up to parity.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum LocalMirrorDeleteScope {
+    /// Default: only delete destination entries that the source-side
+    /// filter would have allowed. Files matching `--exclude` patterns
+    /// at the destination are left alone, because they're not in
+    /// scope for this mirror operation.
+    #[default]
+    FilteredSubset,
+    /// Delete every destination entry not present at the source,
+    /// regardless of filter scope. Selected via `--delete-scope all`.
+    All,
+}
+
+/// Local comparison policy. Mirrors the wire-side `ComparisonMode` enum
+/// for the pull / remote-remote-direct paths so local copy/mirror
+/// behaves the same as a same-options remote run.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum LocalCompareMode {
+    /// Default size + mtime. Skip if both match.
+    #[default]
+    SizeMtime,
+    /// Compare by Blake3 checksum. Slow but content-accurate.
+    Checksum,
+    /// Compare by size only. Mtime differences are ignored.
+    SizeOnly,
+    /// Transfer regardless of target state.
+    Force,
+    /// Transfer all files unconditionally (--ignore-times). Same
+    /// outcome as Force at the planner level; kept as a separate
+    /// variant so the user's intent is preserved in summaries.
+    IgnoreTimes,
+}
+
+impl LocalCompareMode {
+    /// Resolve onto the unified wire-side `ComparisonMode`, honoring
+    /// the legacy `checksum: bool` under the default `SizeMtime`
+    /// (back-compat: `--checksum` callers that haven't migrated to
+    /// `compare_mode` keep their behavior). ue-r2-1c: single home for
+    /// a translation that was previously copy-pasted at three sites
+    /// (streaming, tuning query, single-file), which had already
+    /// diverged once (R58-F7/R58-followup).
+    pub fn resolve_comparison_mode(
+        self,
+        legacy_checksum: bool,
+    ) -> crate::generated::ComparisonMode {
+        use crate::generated::ComparisonMode;
+        match self {
+            LocalCompareMode::Checksum => ComparisonMode::Checksum,
+            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
+            LocalCompareMode::Force => ComparisonMode::Force,
+            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
+            LocalCompareMode::SizeMtime => {
+                if legacy_checksum {
+                    ComparisonMode::Checksum
+                } else {
+                    ComparisonMode::SizeMtime
+                }
+            }
+        }
+    }
+
+    /// Same resolution, onto the perf-history snapshot enum (tuning
+    /// buckets key on the full comparison policy -- R59 finding #5).
+    pub(crate) fn resolve_compare_snapshot(
+        self,
+        legacy_checksum: bool,
+    ) -> crate::perf_history::CompareModeSnapshot {
+        use crate::perf_history::CompareModeSnapshot;
+        match self {
+            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
+            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
+            LocalCompareMode::Force => CompareModeSnapshot::Force,
+            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
+            LocalCompareMode::SizeMtime => {
+                if legacy_checksum {
+                    CompareModeSnapshot::Checksum
+                } else {
+                    CompareModeSnapshot::SizeMtime
+                }
+            }
+        }
+    }
+}
+
+/// Options for executing a local mirror/copy operation.
+#[derive(Clone, Debug)]
+pub struct LocalMirrorOptions {
+    pub filter: FileFilter,
+    pub mirror: bool,
+    pub dry_run: bool,
+    pub progress: bool,
+    pub verbose: bool,
+    pub perf_history: bool,
+    pub force_tar: bool,
+    pub preserve_symlinks: bool,
+    pub include_symlinks: bool,
+    pub skip_unchanged: bool,
+    /// Skip any file the destination already has, regardless of
+    /// comparison mode. Orthogonal to `checksum`/`skip_unchanged`;
+    /// matches the `ignore_existing` field on `TransferOperationSpec`
+    /// for full pipeline parity across local/push/pull paths.
+    pub ignore_existing: bool,
+    pub checksum: bool,
+    /// R58-F7: comparison policy. The orchestrator picks
+    /// `compare_mode` based on this rather than just the `checksum`
+    /// bool, so `--size-only` / `--ignore-times` / `--force` get
+    /// honored on local copy/mirror the same way the pull path
+    /// honors them.
+    pub compare_mode: LocalCompareMode,
+    /// R58-F6: delete-scope policy for mirror. Only consulted when
+    /// `mirror == true`. Defaults to FilteredSubset so a
+    /// `mirror --exclude '*.log'` doesn't delete the destination's
+    /// `*.log` files just because they were out of scope for the
+    /// source filter.
+    pub delete_scope: LocalMirrorDeleteScope,
+    pub workers: usize,
+    pub preserve_times: bool,
+    pub debug_mode: bool,
+    /// Resume interrupted transfers using block-level comparison.
+    pub resume: bool,
+    /// Discard writes (NullSink). Measures source read + pipeline throughput.
+    pub null_sink: bool,
+}
+
+impl Default for LocalMirrorOptions {
+    fn default() -> Self {
+        Self {
+            filter: FileFilter::default(),
+            mirror: false,
+            dry_run: false,
+            progress: false,
+            verbose: false,
+            perf_history: true,
+            force_tar: false,
+            preserve_symlinks: true,
+            include_symlinks: true,
+            skip_unchanged: true,
+            ignore_existing: false,
+            checksum: false,
+            compare_mode: LocalCompareMode::default(),
+            delete_scope: LocalMirrorDeleteScope::default(),
+            workers: num_cpus::get().max(1),
+            preserve_times: true,
+            debug_mode: false,
+            resume: false,
+            null_sink: false,
+        }
+    }
+}
diff --git a/crates/blit-core/src/engine/single_file.rs b/crates/blit-core/src/engine/single_file.rs
new file mode 100644
index 0000000..bee7f0e
--- /dev/null
+++ b/crates/blit-core/src/engine/single_file.rs
@@ -0,0 +1,210 @@
+//! Single-file copy strategy. Moved from
+//! `orchestrator/orchestrator.rs` at ue-r2-1c; the same slice adds
+//! the perf-history/predictor accounting this path lacked
+//! (REV4 Design §2: engine strategies share common accounting).
+
+use std::path::{Path, PathBuf};
+use std::time::Instant;
+
+use eyre::{Context, Result};
+
+use crate::generated::ComparisonMode;
+use crate::perf_predictor::PerformancePredictor;
+
+use super::history::{record_performance_history, update_predictor};
+use super::options::LocalMirrorOptions;
+use super::summary::{LocalMirrorSummary, TransferOutcome};
+
+/// Copy a single file source directly to `dest_root` (the CLI's
+/// destination resolver has already produced the exact target path),
+/// then account for the run. ue-r2-1c: before the engine existed this
+/// shortcut bypassed perf-history/predictor recording entirely — the
+/// only strategy that did. It now records like every other strategy:
+/// tag `single_file` (or `null_sink`, matching the streaming path's
+/// lane convention so RunKind::NullSink derivation keeps working), no
+/// predictor update on null-sink runs (zero write cost would teach the
+/// predictor that transfers are faster than they really are). Records
+/// carry `tar_shard_tasks == raw_bundle_tasks == 0`, so the tuning
+/// window's signal filter already excludes them from auto-tuning.
+pub(super) fn execute_single_file_copy(
+    src_root: &Path,
+    dest_root: &Path,
+    options: &LocalMirrorOptions,
+    start_time: Instant,
+) -> Result<LocalMirrorSummary> {
+    let summary = single_file_copy_inner(src_root, dest_root, options, start_time)?;
+
+    let fast_path_label = if options.null_sink {
+        "null_sink"
+    } else {
+        "single_file"
+    };
+    if let Some(record) = record_performance_history(
+        &summary,
+        options,
+        Some(fast_path_label),
+        0,
+        summary.duration.as_millis(),
+    ) {
+        if !options.null_sink {
+            let mut predictor = PerformancePredictor::load().ok();
+            update_predictor(&mut predictor, &record, options.verbose);
+        }
+    }
+
+    Ok(summary)
+}
+
+/// The copy itself, bypassing the enumerator/planner/pipeline
+/// machinery which assumes `src_root` is a directory.
+fn single_file_copy_inner(
+    src_root: &Path,
+    dest_root: &Path,
+    options: &LocalMirrorOptions,
+    start_time: Instant,
+) -> Result<LocalMirrorSummary> {
+    use crate::buffer::BufferSizer;
+    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
+    use crate::logger::NoopLogger;
+    use filetime::FileTime;
+
+    let src_meta = std::fs::metadata(src_root)
+        .with_context(|| format!("stat source file {}", src_root.display()))?;
+    let size = src_meta.len();
+
+    // R58-followup: route compare-mode for the single-file path
+    // through the same translation the directory path uses
+    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
+    // at `options.checksum`, so `--size-only` / `--ignore-times` /
+    // `--force` were silently dropped — repro: copy src.txt dst.txt
+    // --size-only re-copied even when sizes matched.
+    let compare_mode: ComparisonMode = options
+        .compare_mode
+        .resolve_comparison_mode(options.checksum);
+
+    // R58-F5: the single-file strategy (engine dispatch)
+    // bypasses the enumerator + planner, which is where the
+    // streaming-pipeline path checks filter / ignore_existing.
+    // Apply both here so single-file copies honor the same
+    // CLI contract.
+    //
+    // Filter: the source root is itself the only entry. Run
+    // `filter.allows_entry` against the source name. If excluded,
+    // return a "scanned 1 / copied 0" summary so the user sees
+    // "no work performed" rather than the file being copied
+    // anyway.
+    let src_name = src_root.file_name().map(PathBuf::from);
+    let allows = match src_name {
+        Some(name) => {
+            let mtime = src_meta.modified().ok();
+            options
+                .filter
+                .allows_entry(Some(&name), src_root, size, mtime)
+        }
+        None => true,
+    };
+    if !allows {
+        return Ok(LocalMirrorSummary {
+            planned_files: 0,
+            copied_files: 0,
+            total_bytes: 0,
+            scanned_files: 1,
+            scanned_bytes: size,
+            duration: start_time.elapsed(),
+            outcome: TransferOutcome::UpToDate,
+            ..Default::default()
+        });
+    }
+
+    // ignore_existing: if the destination file already exists,
+    // skip the copy entirely. Matches the diff_planner behavior
+    // for the streaming-pipeline path (diff_planner.rs).
+    if options.ignore_existing && dest_root.exists() {
+        return Ok(LocalMirrorSummary {
+            planned_files: 0,
+            copied_files: 0,
+            total_bytes: 0,
+            scanned_files: 1,
+            scanned_bytes: size,
+            duration: start_time.elapsed(),
+            outcome: TransferOutcome::UpToDate,
+            ..Default::default()
+        });
+    }
+
+    if options.dry_run {
+        return Ok(LocalMirrorSummary {
+            planned_files: 1,
+            copied_files: 1,
+            total_bytes: size,
+            scanned_files: 1,
+            scanned_bytes: size,
+            dry_run: true,
+            duration: start_time.elapsed(),
+            ..Default::default()
+        });
+    }
+
+    if options.null_sink {
+        return Ok(LocalMirrorSummary {
+            planned_files: 1,
+            copied_files: 1,
+            total_bytes: size,
+            scanned_files: 1,
+            scanned_bytes: size,
+            duration: start_time.elapsed(),
+            ..Default::default()
+        });
+    }
+
+    let mut did_copy = false;
+    let mut clone_succeeded = false;
+    let mut bytes_copied = 0u64;
+
+    if options.resume {
+        let outcome = resume_copy_file(src_root, dest_root, 0)
+            .with_context(|| format!("resume copy {}", src_root.display()))?;
+        did_copy = outcome.bytes_transferred > 0;
+        bytes_copied = outcome.bytes_transferred;
+    } else {
+        let needs_copy = !options.skip_unchanged
+            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
+        if needs_copy {
+            let sizer = BufferSizer::default();
+            let logger = NoopLogger;
+            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
+                .with_context(|| format!("copy {}", src_root.display()))?;
+            did_copy = true;
+            clone_succeeded = outcome.clone_succeeded;
+            bytes_copied = outcome.bytes_copied;
+        }
+    }
+
+    if options.preserve_times && did_copy && !clone_succeeded {
+        if let Ok(modified) = src_meta.modified() {
+            let ft = FileTime::from_system_time(modified);
+            // R42-F1: warn-don't-silence (was `let _ = ...`).
+            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
+                log::warn!("set mtime on {}: {}", dest_root.display(), e);
+            }
+        }
+    }
+
+    Ok(LocalMirrorSummary {
+        planned_files: 1,
+        copied_files: if did_copy { 1 } else { 0 },
+        total_bytes: bytes_copied,
+        // Single-file path always saw exactly one entry of `size`
+        // bytes; whether we copied it or not is the
+        // copied_files/total_bytes story, but the scan saw it.
+        scanned_files: 1,
+        scanned_bytes: size,
+        duration: start_time.elapsed(),
+        outcome: if did_copy {
+            TransferOutcome::Transferred
+        } else {
+            TransferOutcome::UpToDate
+        },
+        ..Default::default()
+    })
+}

 succeeded in 0ms:
diff --git a/crates/blit-core/src/engine/mod.rs b/crates/blit-core/src/engine/mod.rs
new file mode 100644
index 0000000..3c9d2c5
--- /dev/null
+++ b/crates/blit-core/src/engine/mod.rs
@@ -0,0 +1,757 @@
+//! The unified transfer engine (`ue-r2-1c`, REV4 Design §1).
+//!
+//! `TransferEngine` owns transfer execution: strategy selection
+//! (`journal_no_work`, `no_work`, `tiny_manifest`, `single_huge_file`,
+//! the single-file shortcut, streaming pipeline), the streaming leg
+//! (plan tuning -> scan -> diff/plan -> sink pipeline -> mirror
+//! deletions), and the perf-history/predictor accounting hooks. Path
+//! adapters construct the source, sink, and options, then call
+//! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
+//! adapter today, and push/pull converge here at `ue-r2-1f`/`1g`.
+//! Dial creation and streaming plans arrive with `ue-r2-1d`/`1e`
+//! (REV4 "Slice dependencies").
+//!
+//! The option/summary types keep their `LocalMirror*` names until the
+//! remote paths converge -- renaming ahead of those slices would churn
+//! every caller twice.
+
+mod history;
+mod journal;
+mod mirror;
+mod options;
+mod single_file;
+mod strategy;
+mod summary;
+mod tuning;
+
+pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
+pub use summary::{LocalMirrorSummary, TransferOutcome};
+
+use std::collections::HashSet;
+use std::path::PathBuf;
+use std::sync::{Arc, Mutex};
+use std::time::Instant;
+
+use eyre::{bail, Context, Result};
+
+use crate::auto_tune::derive_local_plan_tuning;
+use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken};
+use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
+use crate::perf_history::{read_recent_records, TransferMode};
+use crate::perf_predictor::PerformancePredictor;
+use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
+use crate::remote::transfer::payload::DEFAULT_PAYLOAD_PREFETCH;
+use crate::remote::transfer::pipeline::execute_sink_pipeline;
+use crate::remote::transfer::sink::TransferSink;
+use crate::remote::transfer::source::TransferSource;
+use crate::transfer_plan::PlanOptions;
+use crate::CopyConfig;
+
+use self::history::{record_performance_history, update_predictor};
+use self::journal::{log_probe, persist_journal_checkpoints};
+use self::mirror::apply_mirror_deletions;
+use self::single_file::execute_single_file_copy;
+use self::strategy::{maybe_select_fast_path, FastPathDecision};
+use self::tuning::select_tuning_window_from_history;
+
+/// Everything the engine needs to run one transfer. The adapter owns
+/// path-specific construction (REV4 Design §1): it resolves roots,
+/// builds the (already filter-wrapped) source and the sink, translates
+/// its option surface, and hands over execution.
+pub struct EngineRequest {
+    pub src_root: PathBuf,
+    pub dest_root: PathBuf,
+    /// Filter-wrapped source; used by the streaming strategy's scan.
+    pub source: Arc<dyn TransferSource>,
+    /// Destination sink for the streaming strategy (`FsTransferSink`
+    /// or `NullSink` locally). Fast-path strategies use their own
+    /// blocking executors, exactly as before the engine existed.
+    pub sink: Arc<dyn TransferSink>,
+    pub options: LocalMirrorOptions,
+}
+
+/// The unified transfer engine. Stateless today (all state is
+/// per-execute); the live dial (`ue-r2-1e`) is the first field that
+/// will change that.
+pub struct TransferEngine;
+
+impl TransferEngine {
+    pub fn new() -> Self {
+        Self
+    }
+
+    /// Execute one transfer: select a strategy (single-file, journal
+    /// no-work, fast path, or streaming pipeline) and run it to a
+    /// summary. Behavior moved verbatim from
+    /// `TransferOrchestrator::execute_local_mirror_async` at
+    /// ue-r2-1c; the caller-visible contract is unchanged.
+    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
+        let EngineRequest {
+            src_root,
+            dest_root,
+            source,
+            sink,
+            options,
+        } = request;
+        let src_root = src_root.as_path();
+        let dest_root = dest_root.as_path();
+
+        let start_time = Instant::now();
+
+        // Single-file source: bypass the enumerator/planner/pipeline machinery
+        // entirely and copy the file directly. The destination resolver in the
+        // CLI has already produced the exact target path (accounting for
+        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
+        // Without this short-circuit, the enumerator would skip the depth-0
+        // root entry and the fast-path would report NoWork — silent data loss.
+        if src_root.is_file() {
+            return execute_single_file_copy(src_root, dest_root, &options, start_time);
+        }
+
+        let mut journal_tracker = ChangeTracker::load().ok();
+        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
+        let mut journal_skip = false;
+
+        let mut predictor = PerformancePredictor::load().ok();
+
+        let copy_config = CopyConfig {
+            workers: options.workers.max(1),
+            preserve_times: options.preserve_times,
+            dry_run: options.dry_run,
+            checksum: if options.checksum {
+                Some(crate::checksum::ChecksumType::Blake3)
+            } else {
+                None
+            },
+            resume: options.resume,
+            null_sink: options.null_sink,
+        };
+
+        // Journal fast-path requires BOTH source and destination to exist and
+        // report "no changes". A missing destination obviously needs a full
+        // transfer — treating it as unchanged would silently skip the work.
+        if options.skip_unchanged
+            && !options.checksum
+            && !options.force_tar
+            && !options.null_sink
+            && dest_root.exists()
+        {
+            if let Some(tracker) = journal_tracker.as_ref() {
+                match tracker.probe(src_root) {
+                    Ok(src_probe) => {
+                        let dest_probe = tracker.probe(dest_root).ok();
+
+                        if src_probe.snapshot.is_some() {
+                            journal_tokens.push(src_probe.clone());
+                        }
+                        if let Some(ref probe) = dest_probe {
+                            if probe.snapshot.is_some() {
+                                journal_tokens.push(probe.clone());
+                            }
+                        }
+
+                        if options.verbose {
+                            log_probe("src", &src_probe);
+                            if let Some(probe) = dest_probe.as_ref() {
+                                log_probe("dest", probe);
+                            } else {
+                                eprintln!("Journal probe dest unsupported; cannot take fast-path");
+                            }
+                        }
+
+                        let src_no_change = matches!(src_probe.state, ChangeState::NoChanges);
+                        // If dest_probe is None (unsupported FS), we cannot
+                        // assert "no change" — fall through to full planner.
+                        let dest_no_change = dest_probe
+                            .as_ref()
+                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
+                            .unwrap_or(false);
+
+                        if src_no_change && dest_no_change {
+                            journal_skip = true;
+                        }
+                    }
+                    Err(err) => {
+                        if options.verbose {
+                            eprintln!("Filesystem journal probe failed: {err:?}");
+                        }
+                    }
+                }
+            }
+        }
+
+        if journal_skip {
+            if options.verbose {
+                eprintln!(
+                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
+                );
+            }
+            if let Some(tracker) = journal_tracker.as_mut() {
+                persist_journal_checkpoints(
+                    tracker,
+                    journal_tokens.as_mut_slice(),
+                    options.verbose,
+                );
+            }
+
+            // Journal said both sides match, so we never enumerated.
+            // scanned_{files,bytes} stay 0 — predictor sees this as
+            // "noop with no scan cost" which is what actually happened.
+            let summary = LocalMirrorSummary {
+                dry_run: options.dry_run,
+                duration: start_time.elapsed(),
+                outcome: TransferOutcome::JournalSkip,
+                ..Default::default()
+            };
+
+            if let Some(record) = record_performance_history(
+                &summary,
+                &options,
+                Some("journal_no_work"),
+                0,
+                summary.duration.as_millis(),
+            ) {
+                update_predictor(&mut predictor, &record, options.verbose);
+            }
+
+            return Ok(summary);
+        }
+
+        // Skip fast path when using null sink — it bypasses the sink abstraction.
+        let fast_path_outcome = if options.null_sink {
+            self::strategy::FastPathOutcome::streaming()
+        } else {
+            maybe_select_fast_path(src_root, dest_root, &options)?
+        };
+        if let Some(decision) = fast_path_outcome.decision {
+            // R47-F4: propagate the fast-path scan's suppressed
+            // errors into the per-branch summary. Each fast-path
+            // outcome below clones this into `unreadable_paths`
+            // so the CLI's source-delete step can detect a
+            // partial scan even on the Tiny/Huge/NoWork paths.
+            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
+            let summary = match decision {
+                FastPathDecision::NoWork { examined } => {
+                    let outcome = if examined == 0 {
+                        TransferOutcome::SourceEmpty
+                    } else {
+                        TransferOutcome::UpToDate
+                    };
+                    if options.verbose {
+                        match outcome {
+                            TransferOutcome::SourceEmpty => {
+                                eprintln!("Fast-path routing: source yielded no file entries")
+                            }
+                            _ => eprintln!(
+                                "Fast-path routing: {} files examined, all up to date",
+                                examined
+                            ),
+                        }
+                    }
+                    // NoWork ran a real fast-path scan but copied nothing.
+                    // scanned_files = examined captures the planner-side
+                    // workload; scanned_bytes is 0 because the fast-path
+                    // scanner only resolves names + identity, not sizes.
+                    let summary = LocalMirrorSummary {
+                        planned_files: examined,
+                        scanned_files: examined,
+                        dry_run: options.dry_run,
+                        duration: start_time.elapsed(),
+                        outcome,
+                        unreadable_paths: fast_path_unreadable.clone(),
+                        ..Default::default()
+                    };
+                    if let Some(record) = record_performance_history(
+                        &summary,
+                        &options,
+                        Some("no_work"),
+                        0,
+                        summary.duration.as_millis(),
+                    ) {
+                        update_predictor(&mut predictor, &record, options.verbose);
+                    }
+                    summary
+                }
+                FastPathDecision::Tiny { files } => {
+                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
+                    if options.verbose {
+                        eprintln!(
+                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
+                            files.len(),
+                            total_bytes
+                        );
+                    }
+                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
+                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
+                    // Tiny copies everything it scanned, so scanned ==
+                    // copied here. Setting both lets the predictor
+                    // train on the actual workload size for the
+                    // tiny_manifest fast-path key.
+                    let summary = LocalMirrorSummary {
+                        planned_files: files.len(),
+                        copied_files: files.len(),
+                        total_bytes,
+                        scanned_files: files.len(),
+                        scanned_bytes: total_bytes,
+                        dry_run: options.dry_run,
+                        duration: start_time.elapsed(),
+                        unreadable_paths: fast_path_unreadable.clone(),
+                        ..Default::default()
+                    };
+                    if let Some(record) = record_performance_history(
+                        &summary,
+                        &options,
+                        Some("tiny_manifest"),
+                        0,
+                        summary.duration.as_millis(),
+                    ) {
+                        update_predictor(&mut predictor, &record, options.verbose);
+                    }
+                    summary
+                }
+                FastPathDecision::Huge { file, size } => {
+                    if options.verbose {
+                        eprintln!(
+                            "Fast-path routing: huge file {} ({} bytes)",
+                            file.display(),
+                            size
+                        );
+                    }
+                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
+                    // Huge fast-path copies a single file: scan size
+                    // and copy size are identical (one file, `size`
+                    // bytes).
+                    let summary = LocalMirrorSummary {
+                        planned_files: 1,
+                        copied_files: 1,
+                        total_bytes: size,
+                        scanned_files: 1,
+                        scanned_bytes: size,
+                        dry_run: options.dry_run,
+                        duration: start_time.elapsed(),
+                        large_tasks: 1,
+                        large_bytes: size,
+                        unreadable_paths: fast_path_unreadable.clone(),
+                        ..Default::default()
+                    };
+                    if let Some(record) = record_performance_history(
+                        &summary,
+                        &options,
+                        Some("single_huge_file"),
+                        0,
+                        summary.duration.as_millis(),
+                    ) {
+                        update_predictor(&mut predictor, &record, options.verbose);
+                    }
+                    summary
+                }
+            };
+
+            if let Some(tracker) = journal_tracker.as_mut() {
+                persist_journal_checkpoints(
+                    tracker,
+                    journal_tokens.as_mut_slice(),
+                    options.verbose,
+                );
+            }
+
+            if options.verbose {
+                eprintln!(
+                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
+                    if options.mirror { "mirror" } else { "copy" },
+                    summary.copied_files,
+                    summary.total_bytes,
+                    summary.duration
+                );
+            }
+
+            return Ok(summary);
+        }
+
+        // --- Unified pipeline: same path as remote transfers ---
+        let mut plan_options = PlanOptions {
+            force_tar: options.force_tar,
+            ..PlanOptions::default()
+        };
+
+        if options.perf_history {
+            // R57-F1: read ALL history, not a pre-cap window. The
+            // R56-F2 fix correctly filtered run_kind before the
+            // 20-record cap inside `select_tuning_window`, but the
+            // caller was still pre-capping at 50 records from the
+            // JSONL — so 50 recent non-real records could still
+            // hide older real records one layer up. The file is
+            // already size-capped at ~1 MiB upstream
+            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
+            // all records is bounded; `read_recent_records(0)`
+            // means "all" per its limit semantics.
+            let target_mode = if options.mirror {
+                TransferMode::Mirror
+            } else {
+                TransferMode::Copy
+            };
+            // R59 finding #5: tuning window keys on full compare_mode,
+            // not just options.checksum. Translate via the same enum
+            // the history snapshot uses so the bucket lookup matches
+            // what the writer recorded.
+            let query_compare_mode = options
+                .compare_mode
+                .resolve_compare_snapshot(options.checksum);
+            if let Some(filtered) = select_tuning_window_from_history(
+                read_recent_records,
+                target_mode,
+                query_compare_mode,
+                options.skip_unchanged,
+            ) {
+                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
+                    plan_options.small_target = Some(tuning.small_target_bytes);
+                    plan_options.small_count_target = Some(tuning.small_count_target);
+                    plan_options.medium_target = Some(tuning.medium_target_bytes);
+                }
+            }
+        }
+
+        let planning_start = Instant::now();
+
+        let src_root_buf = src_root.to_path_buf();
+        let dest_root_buf = dest_root.to_path_buf();
+        let skip_unchanged = options.skip_unchanged;
+        let ignore_existing = options.ignore_existing;
+        // R58-F7: translate the orchestrator's `compare_mode` (set by
+        // the CLI from --size-only / --ignore-times / --force /
+        // --checksum / default) onto the unified ComparisonMode enum.
+        // Pre-fix this hardcoded a bool→Checksum-or-SizeMtime mapping
+        // and ignored the other flags entirely; remote pull already
+        // honored all five variants, so behavior diverged by direction.
+        //
+        // Backward-compat: the old `options.checksum` bool still
+        // wins if it's set without `compare_mode` being explicitly
+        // changed — preserves the existing `--checksum` behavior
+        // for any caller that hasn't migrated yet.
+        let compare_mode = options
+            .compare_mode
+            .resolve_comparison_mode(options.checksum);
+
+        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
+        //    the user filter applies through the universal pipeline chokepoint
+        //    (identical to push/pull/remote-remote behavior — full parity).
+        // ue-r2-1c: the adapter built the (filter-wrapped) source; the
+        // engine owns running the scan.
+        let unreadable = Arc::new(Mutex::new(Vec::new()));
+        let (mut header_rx, scan_handle) = source.scan(None, unreadable.clone());
+
+        // 2. Collect all headers
+        let mut all_headers = Vec::new();
+        while let Some(h) = header_rx.recv().await {
+            all_headers.push(h);
+        }
+        let _total_scanned = scan_handle
+            .await
+            .context("scan task panicked")?
+            .context("scan failed")?;
+
+        // 3. Diff + plan via the shared DiffPlanner stage. Combines
+        //    the comparison-filter and payload-planning steps that
+        //    were previously inline. Behavior preserved bit-for-bit
+        //    (size+mtime or Blake3 hash, then tar/large/raw planning).
+        let src = src_root_buf.clone();
+        let dst = dest_root_buf.clone();
+        let plan_opts = plan_options;
+        let headers = all_headers.clone();
+        let planned = tokio::task::spawn_blocking(move || {
+            plan_local_mirror(
+                headers,
+                LocalDiffInputs {
+                    src_root: &src,
+                    dst_root: &dst,
+                    compare_mode,
+                    ignore_existing,
+                    plan_options: plan_opts,
+                    skip_unchanged,
+                },
+            )
+        })
+        .await
+        .context("diff_planner task panicked")??;
+
+        // 5. Execute the unified pipeline against the adapter-built
+        // sink (FsTransferSink with the translated compare_mode, or
+        // NullSink -- see TransferOrchestrator).
+
+        // Boundary between planner and transfer phases. `planning_start`
+        // covers scan + diff + plan; everything after this `Instant`
+        // is the transfer pipeline. §2.8 phase 2 split: pre-fix the
+        // record's `planner_duration_ms` field was set to whole-run
+        // time, so the v1 predictor effectively trained on `planner =
+        // total` for both targets and couldn't distinguish them.
+        let plan_done = Instant::now();
+        let planner_duration_ms = plan_done.duration_since(planning_start).as_millis();
+
+        // §2.8 phase 2: query the predictor BEFORE running the
+        // pipeline. Surfaces in summary.predictor_estimate so
+        // `--verbose` and `blit profile --json` can compare
+        // predicted vs actual.
+        //
+        // R44-F1: query and observation must use the same feature
+        // vector. We query with `(scanned_files, scanned_bytes)`
+        // here; `record_performance_history` populates the matching
+        // `PerformanceRecord.{file_count,total_bytes}` from
+        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
+        // record was populated from `summary.copied_files`, so on
+        // any incremental run the predictor was queried with one
+        // workload size and trained against another.
+        //
+        // src_fs/dest_fs are left None for 0.1.0 — wiring
+        // `fs_capability` per-path probes into the predictor query
+        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
+        let scanned_files = all_headers.len();
+        let scanned_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
+        // R45 follow-up to R44-F1: never alias `total_bytes` to
+        // `scanned_bytes`. `summary.total_bytes` is the
+        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
+        // rustdoc); the predictor uses scan features only. Pre-fix
+        // this aliased the two so `summary.total_bytes` reported
+        // scanned bytes as bytes-written, overcounting throughput
+        // on incremental runs.
+        let predictor_estimate = predictor.as_ref().and_then(|p| {
+            let kind_total = crate::perf_predictor::DurationKind::Total;
+            let mode = if options.mirror {
+                crate::perf_history::TransferMode::Mirror
+            } else {
+                crate::perf_history::TransferMode::Copy
+            };
+            let total_pred = p.predict(
+                kind_total,
+                mode.clone(),
+                None,
+                None,
+                None,
+                options.skip_unchanged,
+                options.checksum,
+                scanned_files,
+                scanned_bytes,
+            )?;
+            // Pull planner + transfer separately too so the verbose
+            // line and the JSON profile can break down the estimate.
+            // All three predictor calls share the same
+            // (scanned_files, scanned_bytes) feature vector — both
+            // for consistency with the recording side, and so a
+            // future maintainer can't accidentally reintroduce a
+            // train/query mismatch by editing one branch and
+            // missing another.
+            let planner_pred = p
+                .predict(
+                    crate::perf_predictor::DurationKind::Planner,
+                    mode.clone(),
+                    None,
+                    None,
+                    None,
+                    options.skip_unchanged,
+                    options.checksum,
+                    scanned_files,
+                    scanned_bytes,
+                )
+                .map(|p| p.predicted_ms)
+                .unwrap_or(0.0);
+            let transfer_pred = p
+                .predict(
+                    crate::perf_predictor::DurationKind::Transfer,
+                    mode,
+                    None,
+                    None,
+                    None,
+                    options.skip_unchanged,
+                    options.checksum,
+                    scanned_files,
+                    scanned_bytes,
+                )
+                .map(|p| p.predicted_ms)
+                .unwrap_or(0.0);
+            Some(self::summary::PredictorEstimate {
+                planner_ms: planner_pred.max(0.0) as u128,
+                transfer_ms: transfer_pred.max(0.0) as u128,
+                total_ms: total_pred.predicted_ms.max(0.0) as u128,
+                observations: total_pred.observations,
+                fallback_depth: total_pred.fallback_depth,
+            })
+        });
+        if options.verbose {
+            if let Some(est) = predictor_estimate.as_ref() {
+                eprintln!(
+                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
+                     total ~{} ms (n={}, fallback_depth={})",
+                    est.planner_ms,
+                    est.transfer_ms,
+                    est.total_ms,
+                    est.observations,
+                    est.fallback_depth
+                );
+            } else {
+                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
+            }
+        }
+
+        let pipeline_outcome = execute_sink_pipeline(
+            source,
+            vec![sink],
+            planned.payloads,
+            DEFAULT_PAYLOAD_PREFETCH,
+            None,
+        )
+        .await
+        .context("transfer pipeline failed")?;
+        let transfer_duration_ms = plan_done.elapsed().as_millis();
+
+        // R47-F4: snapshot unreadable paths so the CLI's source-
+        // delete step (in `blit move`) can refuse to remove a
+        // source it couldn't fully scan. The R46-F2 gate inside
+        // the orchestrator only fires on `options.mirror`, but
+        // move uses mirror=false — without this surface, an
+        // unreadable source file would get skipped during the
+        // copy and then silently deleted from the source by the
+        // CLI's `remove_dir_all` step.
+        let unreadable_snapshot: Vec<String> = unreadable
+            .lock()
+            .map(|guard| guard.clone())
+            .unwrap_or_default();
+
+        let mut summary = LocalMirrorSummary {
+            planned_files: pipeline_outcome.files_written,
+            copied_files: pipeline_outcome.files_written,
+            // R45: bytes the pipeline actually wrote, not scanned
+            // bytes. Distinct on incremental runs.
+            total_bytes: pipeline_outcome.bytes_written,
+            scanned_files,
+            scanned_bytes,
+            dry_run: options.dry_run,
+            duration: start_time.elapsed(),
+            predictor_estimate: predictor_estimate.clone(),
+            unreadable_paths: unreadable_snapshot.clone(),
+            ..Default::default()
+        };
+
+        if options.mirror {
+            // R46-F2: refuse to mirror-delete when the source scan
+            // was incomplete. The `unreadable_snapshot` captured
+            // above (R47-F4) covers the per-file open path
+            // (PermissionDenied / NotFound on individual files) and
+            // the walkdir non-root error path (unreadable
+            // subdirectories). Either case means the header set
+            // we're about to use as the source-of-truth for "what
+            // the destination should contain" is missing entries,
+            // and a delete pass would silently remove matching
+            // destination subtrees.
+            if !unreadable_snapshot.is_empty() {
+                bail!(
+                    "refusing to mirror-delete from {}: source scan was \
+                     incomplete ({} unreadable entr{}); the first {} \
+                     reported: {}. Resolve the scan errors (typically \
+                     permissions) or run as a non-mirror copy.",
+                    dest_root.display(),
+                    unreadable_snapshot.len(),
+                    if unreadable_snapshot.len() == 1 {
+                        "y"
+                    } else {
+                        "ies"
+                    },
+                    unreadable_snapshot.len().min(5),
+                    unreadable_snapshot
+                        .iter()
+                        .take(5)
+                        .cloned()
+                        .collect::<Vec<_>>()
+                        .join("; "),
+                );
+            }
+
+            let source_paths: HashSet<String> = all_headers
+                .iter()
+                .map(|h| h.relative_path.clone())
+                .collect();
+            let deletions = apply_mirror_deletions(
+                &source_paths,
+                dest_root,
+                &options.filter,
+                options.delete_scope,
+                !options.dry_run,
+                options.verbose,
+            )?;
+            summary.deleted_files = deletions.0;
+            summary.deleted_dirs = deletions.1;
+        }
+
+        if let Some(tracker) = journal_tracker.as_mut() {
+            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
+        }
+
+        if options.verbose {
+            eprintln!(
+                "Planning enumerated {} file(s), {} bytes",
+                scanned_files, scanned_bytes
+            );
+            eprintln!(
+                "Completed local {}: {} file(s), {} bytes in {:.2?} (plan {} ms, xfer {} ms)",
+                if options.mirror { "mirror" } else { "copy" },
+                summary.copied_files,
+                summary.total_bytes,
+                summary.duration,
+                planner_duration_ms,
+                transfer_duration_ms,
+            );
+            // §2.8: side-by-side predicted-vs-actual so operators
+            // can audit the predictor against this run's actual
+            // numbers. The bare percentage error per phase is the
+            // most useful single number; we keep absolute ms in the
+            // line above for context.
+            if let Some(est) = summary.predictor_estimate.as_ref() {
+                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
+                    if actual_ms == 0 {
+                        "n/a".to_string()
+                    } else {
+                        let pred = predicted_ms as f64;
+                        let act = actual_ms as f64;
+                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
+                    }
+                };
+                eprintln!(
+                    "Predictor delta: planner {} ({} vs {} ms), \
+                     transfer {} ({} vs {} ms)",
+                    pct(est.planner_ms, planner_duration_ms),
+                    est.planner_ms,
+                    planner_duration_ms,
+                    pct(est.transfer_ms, transfer_duration_ms),
+                    est.transfer_ms,
+                    transfer_duration_ms,
+                );
+            }
+        }
+
+        let fast_path_label = if options.null_sink {
+            Some("null_sink")
+        } else {
+            None
+        };
+        if let Some(record) = record_performance_history(
+            &summary,
+            &options,
+            fast_path_label,
+            planner_duration_ms,
+            transfer_duration_ms,
+        ) {
+            // Don't update the predictor from null-sink runs — the zero
+            // write cost would teach it that transfers are faster than
+            // they really are.
+            if !options.null_sink {
+                update_predictor(&mut predictor, &record, options.verbose);
+            }
+        }
+
+        Ok(summary)
+    }
+}
+
+impl Default for TransferEngine {
+    fn default() -> Self {
+        Self::new()
+    }
+}
diff --git a/crates/blit-core/src/orchestrator/orchestrator.rs b/crates/blit-core/src/orchestrator/orchestrator.rs
index 0d15d14..49b4593 100644
--- a/crates/blit-core/src/orchestrator/orchestrator.rs
+++ b/crates/blit-core/src/orchestrator/orchestrator.rs
@@ -1,118 +1,23 @@
-use std::collections::HashSet;
-use std::path::{Path, PathBuf};
-use std::sync::{Arc, Mutex};
-use std::time::Instant;
+use std::path::Path;
+use std::sync::Arc;
 
-use eyre::{bail, eyre, Context, Result};
+use eyre::{eyre, Context, Result};
 use tokio::runtime::Builder;
 
-use crate::auto_tune::derive_local_plan_tuning;
-use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken, StoredSnapshot};
-use crate::fs_enum::FileFilter;
-use crate::generated::ComparisonMode;
-use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
-use crate::perf_history::{read_recent_records, TransferMode};
-use crate::perf_predictor::PerformancePredictor;
-use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
-use crate::remote::transfer::payload::DEFAULT_PAYLOAD_PREFETCH;
-use crate::remote::transfer::pipeline::execute_sink_pipeline;
+use crate::engine::{EngineRequest, TransferEngine};
 use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, NullSink, TransferSink};
 use crate::remote::transfer::source::{FilteredSource, FsTransferSource, TransferSource};
-use crate::transfer_plan::PlanOptions;
-use crate::CopyConfig;
-
-use super::fast_path::{maybe_select_fast_path, FastPathDecision};
-use super::history::{record_performance_history, update_predictor};
-use super::options::LocalMirrorOptions;
-use super::summary::{LocalMirrorSummary, TransferOutcome};
-
-/// Maximum number of recent eligible records the local tuner looks
-/// at. The cap exists so a recent regime change (new disk, fresh
-/// install) propagates into tuning within ~20 transfers instead of
-/// being diluted by older history.
-const TUNING_WINDOW_SIZE: usize = 20;
-
-/// R56-F2: select the window of recent records that should feed
-/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
-/// FIRST, then the per-operation discriminants, THEN takes the
-/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
-/// run_kind filter, so 20 recent dry-run / null-sink records with
-/// matching mode could fill the window and force tuning to fall
-/// back to defaults even when older real records existed.
-///
-/// Extracted so the contract is unit-testable without touching
-/// the global perf-history JSONL.
-fn select_tuning_window(
-    history: &[crate::perf_history::PerformanceRecord],
-    target_mode: TransferMode,
-    compare_mode: crate::perf_history::CompareModeSnapshot,
-    skip_unchanged: bool,
-) -> Vec<crate::perf_history::PerformanceRecord> {
-    history
-        .iter()
-        .rev()
-        .filter(|record| record.run_kind.is_real_transfer())
-        .filter(|record| record.mode == target_mode)
-        // R59 finding #5: key on the full comparison policy
-        // (not just `checksum: bool`) so SizeMtime / SizeOnly /
-        // Force / IgnoreTimes runs don't mix into the same tuning
-        // bucket. Pre-fix a session of `--size-only` runs trained
-        // the SizeMtime bucket (and vice versa).
-        .filter(|record| record.options.compare_mode == compare_mode)
-        .filter(|record| record.options.skip_unchanged == skip_unchanged)
-        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
-        // R58-followup: require a tuning signal. `derive_local_plan_tuning`
-        // only aggregates `tar_shard_*` + `raw_bundle_*`; records with
-        // `tar_shard_tasks == 0 && raw_bundle_tasks == 0` (no_work,
-        // journal_no_work, single_huge_file, streaming no-ops) are
-        // RunKind::Real and pass every other gate but contribute
-        // nothing. Pre-fix they could fill the 20-slot window and
-        // hide older bucket-bearing records. If the tuner ever
-        // starts consuming `large_tasks`, add it here too.
-        .filter(|record| record.tar_shard_tasks > 0 || record.raw_bundle_tasks > 0)
-        .take(TUNING_WINDOW_SIZE)
-        .cloned()
-        .collect()
-}
 
-/// R57-F1: wrapper that always reads the FULL history before
-/// applying the run_kind filter. The caller used to pass
-/// `read_recent_records(50)`, which pre-capped the input slice
-/// at 50 records — so 50 recent non-real records could hide
-/// older real records before `select_tuning_window` ever saw
-/// them. Baking the "ask for all records" invariant into the
-/// wrapper means the limit can't drift back to a finite value.
-/// The history file is already size-capped at ~1 MiB upstream
-/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
-/// records is bounded.
-///
-/// Generic over the reader so unit tests can inject a synthetic
-/// history; production passes `read_recent_records` directly.
-/// Returns `None` if the reader errored OR no eligible records
-/// were found; the caller treats either case as "fall back to
-/// defaults."
-fn select_tuning_window_from_history<F>(
-    reader: F,
-    target_mode: TransferMode,
-    compare_mode: crate::perf_history::CompareModeSnapshot,
-    skip_unchanged: bool,
-) -> Option<Vec<crate::perf_history::PerformanceRecord>>
-where
-    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
-{
-    // `0` means "all records" per read_recent_records' contract
-    // (see read_records_from_path in perf_history.rs:298). This
-    // is the load-bearing literal — passing anything else
-    // reintroduces R57-F1.
-    let history = reader(0).ok()?;
-    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
-    if window.is_empty() {
-        None
-    } else {
-        Some(window)
-    }
-}
+use super::{LocalMirrorOptions, LocalMirrorSummary};
 
+/// The LOCAL adapter for [`TransferEngine`] (ue-r2-1c, REV4 Design §1).
+///
+/// Owns exactly the path-specific boundary work: precondition checks,
+/// construction of the local filesystem source (filter-wrapped) and
+/// sink, and option translation. Everything else -- strategy selection
+/// (journal / fast paths / single-file / streaming), execution, and
+/// accounting -- lives in the engine. The public API is unchanged from
+/// the pre-engine orchestrator.
 pub struct TransferOrchestrator;
 
 impl TransferOrchestrator {
@@ -124,9 +29,11 @@ impl TransferOrchestrator {
     /// new multi-thread Tokio runtime and blocks on it. Use this from
     /// non-async callers (CLI commands, tests). Callers already
     /// inside an async runtime must use `execute_local_mirror_async`
-    /// directly — calling this from inside a Tokio context will
+    /// directly -- calling this from inside a Tokio context will
     /// panic at `Runtime::new` (closes F9 of
     /// `docs/reviews/codebase_review_2026-05-01.md`).
+    ///
+    /// [`execute_local_mirror_async`]: Self::execute_local_mirror_async
     pub fn execute_local_mirror(
         &self,
         src_root: &Path,
@@ -142,12 +49,9 @@ impl TransferOrchestrator {
         runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
     }
 
-    /// Async core of the local-mirror orchestrator. Callable from
-    /// any async context. Closes F9 of the 2026-05-01 baseline
-    /// review: previously `execute_local_mirror` built and owned its
-    /// own Tokio runtime, which panicked when called from an async
-    /// caller. The sync wrapper above is now a thin convenience for
-    /// blocking callers.
+    /// Async local-transfer entry point: validate the local
+    /// preconditions, construct the local source/sink pair, and hand
+    /// execution to the engine.
     pub async fn execute_local_mirror_async(
         &self,
         src_root: &Path,
@@ -166,428 +70,39 @@ impl TransferOrchestrator {
             }
         }
 
-        let start_time = Instant::now();
-
-        // Single-file source: bypass the enumerator/planner/pipeline machinery
-        // entirely and copy the file directly. The destination resolver in the
-        // CLI has already produced the exact target path (accounting for
-        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
-        // Without this short-circuit, the enumerator would skip the depth-0
-        // root entry and the fast-path would report NoWork — silent data loss.
-        if src_root.is_file() {
-            return execute_single_file_copy(src_root, dest_root, &options, start_time);
-        }
-
-        let mut journal_tracker = ChangeTracker::load().ok();
-        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
-        let mut journal_skip = false;
-
-        let mut predictor = PerformancePredictor::load().ok();
-
-        let copy_config = CopyConfig {
-            workers: options.workers.max(1),
-            preserve_times: options.preserve_times,
-            dry_run: options.dry_run,
-            checksum: if options.checksum {
-                Some(crate::checksum::ChecksumType::Blake3)
-            } else {
-                None
-            },
-            resume: options.resume,
-            null_sink: options.null_sink,
-        };
-
-        // Journal fast-path requires BOTH source and destination to exist and
-        // report "no changes". A missing destination obviously needs a full
-        // transfer — treating it as unchanged would silently skip the work.
-        if options.skip_unchanged
-            && !options.checksum
-            && !options.force_tar
-            && !options.null_sink
-            && dest_root.exists()
-        {
-            if let Some(tracker) = journal_tracker.as_ref() {
-                match tracker.probe(src_root) {
-                    Ok(src_probe) => {
-                        let dest_probe = tracker.probe(dest_root).ok();
-
-                        if src_probe.snapshot.is_some() {
-                            journal_tokens.push(src_probe.clone());
-                        }
-                        if let Some(ref probe) = dest_probe {
-                            if probe.snapshot.is_some() {
-                                journal_tokens.push(probe.clone());
-                            }
-                        }
-
-                        if options.verbose {
-                            log_probe("src", &src_probe);
-                            if let Some(probe) = dest_probe.as_ref() {
-                                log_probe("dest", probe);
-                            } else {
-                                eprintln!("Journal probe dest unsupported; cannot take fast-path");
-                            }
-                        }
-
-                        let src_no_change = matches!(src_probe.state, ChangeState::NoChanges);
-                        // If dest_probe is None (unsupported FS), we cannot
-                        // assert "no change" — fall through to full planner.
-                        let dest_no_change = dest_probe
-                            .as_ref()
-                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
-                            .unwrap_or(false);
-
-                        if src_no_change && dest_no_change {
-                            journal_skip = true;
-                        }
-                    }
-                    Err(err) => {
-                        if options.verbose {
-                            eprintln!("Filesystem journal probe failed: {err:?}");
-                        }
-                    }
-                }
-            }
-        }
-
-        if journal_skip {
-            if options.verbose {
-                eprintln!(
-                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
-                );
-            }
-            if let Some(tracker) = journal_tracker.as_mut() {
-                persist_journal_checkpoints(
-                    tracker,
-                    journal_tokens.as_mut_slice(),
-                    options.verbose,
-                );
-            }
-
-            // Journal said both sides match, so we never enumerated.
-            // scanned_{files,bytes} stay 0 — predictor sees this as
-            // "noop with no scan cost" which is what actually happened.
-            let summary = LocalMirrorSummary {
-                dry_run: options.dry_run,
-                duration: start_time.elapsed(),
-                outcome: TransferOutcome::JournalSkip,
-                ..Default::default()
-            };
-
-            if let Some(record) = record_performance_history(
-                &summary,
-                &options,
-                Some("journal_no_work"),
-                0,
-                summary.duration.as_millis(),
-            ) {
-                update_predictor(&mut predictor, &record, options.verbose);
-            }
-
-            return Ok(summary);
-        }
-
-        // Skip fast path when using null sink — it bypasses the sink abstraction.
-        let fast_path_outcome = if options.null_sink {
-            super::fast_path::FastPathOutcome::streaming()
-        } else {
-            maybe_select_fast_path(src_root, dest_root, &options)?
-        };
-        if let Some(decision) = fast_path_outcome.decision {
-            // R47-F4: propagate the fast-path scan's suppressed
-            // errors into the per-branch summary. Each fast-path
-            // outcome below clones this into `unreadable_paths`
-            // so the CLI's source-delete step can detect a
-            // partial scan even on the Tiny/Huge/NoWork paths.
-            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
-            let summary = match decision {
-                FastPathDecision::NoWork { examined } => {
-                    let outcome = if examined == 0 {
-                        TransferOutcome::SourceEmpty
-                    } else {
-                        TransferOutcome::UpToDate
-                    };
-                    if options.verbose {
-                        match outcome {
-                            TransferOutcome::SourceEmpty => {
-                                eprintln!("Fast-path routing: source yielded no file entries")
-                            }
-                            _ => eprintln!(
-                                "Fast-path routing: {} files examined, all up to date",
-                                examined
-                            ),
-                        }
-                    }
-                    // NoWork ran a real fast-path scan but copied nothing.
-                    // scanned_files = examined captures the planner-side
-                    // workload; scanned_bytes is 0 because the fast-path
-                    // scanner only resolves names + identity, not sizes.
-                    let summary = LocalMirrorSummary {
-                        planned_files: examined,
-                        scanned_files: examined,
-                        dry_run: options.dry_run,
-                        duration: start_time.elapsed(),
-                        outcome,
-                        unreadable_paths: fast_path_unreadable.clone(),
-                        ..Default::default()
-                    };
-                    if let Some(record) = record_performance_history(
-                        &summary,
-                        &options,
-                        Some("no_work"),
-                        0,
-                        summary.duration.as_millis(),
-                    ) {
-                        update_predictor(&mut predictor, &record, options.verbose);
-                    }
-                    summary
-                }
-                FastPathDecision::Tiny { files } => {
-                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
-                    if options.verbose {
-                        eprintln!(
-                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
-                            files.len(),
-                            total_bytes
-                        );
-                    }
-                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
-                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
-                    // Tiny copies everything it scanned, so scanned ==
-                    // copied here. Setting both lets the predictor
-                    // train on the actual workload size for the
-                    // tiny_manifest fast-path key.
-                    let summary = LocalMirrorSummary {
-                        planned_files: files.len(),
-                        copied_files: files.len(),
-                        total_bytes,
-                        scanned_files: files.len(),
-                        scanned_bytes: total_bytes,
-                        dry_run: options.dry_run,
-                        duration: start_time.elapsed(),
-                        unreadable_paths: fast_path_unreadable.clone(),
-                        ..Default::default()
-                    };
-                    if let Some(record) = record_performance_history(
-                        &summary,
-                        &options,
-                        Some("tiny_manifest"),
-                        0,
-                        summary.duration.as_millis(),
-                    ) {
-                        update_predictor(&mut predictor, &record, options.verbose);
-                    }
-                    summary
-                }
-                FastPathDecision::Huge { file, size } => {
-                    if options.verbose {
-                        eprintln!(
-                            "Fast-path routing: huge file {} ({} bytes)",
-                            file.display(),
-                            size
-                        );
-                    }
-                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
-                    // Huge fast-path copies a single file: scan size
-                    // and copy size are identical (one file, `size`
-                    // bytes).
-                    let summary = LocalMirrorSummary {
-                        planned_files: 1,
-                        copied_files: 1,
-                        total_bytes: size,
-                        scanned_files: 1,
-                        scanned_bytes: size,
-                        dry_run: options.dry_run,
-                        duration: start_time.elapsed(),
-                        large_tasks: 1,
-                        large_bytes: size,
-                        unreadable_paths: fast_path_unreadable.clone(),
-                        ..Default::default()
-                    };
-                    if let Some(record) = record_performance_history(
-                        &summary,
-                        &options,
-                        Some("single_huge_file"),
-                        0,
-                        summary.duration.as_millis(),
-                    ) {
-                        update_predictor(&mut predictor, &record, options.verbose);
-                    }
-                    summary
-                }
-            };
-
-            if let Some(tracker) = journal_tracker.as_mut() {
-                persist_journal_checkpoints(
-                    tracker,
-                    journal_tokens.as_mut_slice(),
-                    options.verbose,
-                );
-            }
-
-            if options.verbose {
-                eprintln!(
-                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
-                    if options.mirror { "mirror" } else { "copy" },
-                    summary.copied_files,
-                    summary.total_bytes,
-                    summary.duration
-                );
-            }
-
-            return Ok(summary);
-        }
-
-        // --- Unified pipeline: same path as remote transfers ---
-        let mut plan_options = PlanOptions {
-            force_tar: options.force_tar,
-            ..PlanOptions::default()
-        };
-
-        if options.perf_history {
-            // R57-F1: read ALL history, not a pre-cap window. The
-            // R56-F2 fix correctly filtered run_kind before the
-            // 20-record cap inside `select_tuning_window`, but the
-            // caller was still pre-capping at 50 records from the
-            // JSONL — so 50 recent non-real records could still
-            // hide older real records one layer up. The file is
-            // already size-capped at ~1 MiB upstream
-            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
-            // all records is bounded; `read_recent_records(0)`
-            // means "all" per its limit semantics.
-            let target_mode = if options.mirror {
-                TransferMode::Mirror
-            } else {
-                TransferMode::Copy
-            };
-            // R59 finding #5: tuning window keys on full compare_mode,
-            // not just options.checksum. Translate via the same enum
-            // the history snapshot uses so the bucket lookup matches
-            // what the writer recorded.
-            let query_compare_mode = match options.compare_mode {
-                crate::orchestrator::LocalCompareMode::Checksum => {
-                    crate::perf_history::CompareModeSnapshot::Checksum
-                }
-                crate::orchestrator::LocalCompareMode::SizeOnly => {
-                    crate::perf_history::CompareModeSnapshot::SizeOnly
-                }
-                crate::orchestrator::LocalCompareMode::Force => {
-                    crate::perf_history::CompareModeSnapshot::Force
-                }
-                crate::orchestrator::LocalCompareMode::IgnoreTimes => {
-                    crate::perf_history::CompareModeSnapshot::IgnoreTimes
-                }
-                crate::orchestrator::LocalCompareMode::SizeMtime => {
-                    if options.checksum {
-                        crate::perf_history::CompareModeSnapshot::Checksum
-                    } else {
-                        crate::perf_history::CompareModeSnapshot::SizeMtime
-                    }
-                }
-            };
-            if let Some(filtered) = select_tuning_window_from_history(
-                read_recent_records,
-                target_mode,
-                query_compare_mode,
-                options.skip_unchanged,
-            ) {
-                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
-                    plan_options.small_target = Some(tuning.small_target_bytes);
-                    plan_options.small_count_target = Some(tuning.small_count_target);
-                    plan_options.medium_target = Some(tuning.medium_target_bytes);
-                }
-            }
-        }
-
-        let planning_start = Instant::now();
-
-        let src_root_buf = src_root.to_path_buf();
-        let dest_root_buf = dest_root.to_path_buf();
-        let filter = options.filter.clone_without_cache();
-        let skip_unchanged = options.skip_unchanged;
-        let ignore_existing = options.ignore_existing;
-        // R58-F7: translate the orchestrator's `compare_mode` (set by
-        // the CLI from --size-only / --ignore-times / --force /
-        // --checksum / default) onto the unified ComparisonMode enum.
-        // Pre-fix this hardcoded a bool→Checksum-or-SizeMtime mapping
-        // and ignored the other flags entirely; remote pull already
-        // honored all five variants, so behavior diverged by direction.
-        //
-        // Backward-compat: the old `options.checksum` bool still
-        // wins if it's set without `compare_mode` being explicitly
-        // changed — preserves the existing `--checksum` behavior
-        // for any caller that hasn't migrated yet.
-        let compare_mode = match options.compare_mode {
-            crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
-            crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
-            crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
-            crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
-            crate::orchestrator::LocalCompareMode::SizeMtime => {
-                if options.checksum {
-                    ComparisonMode::Checksum
-                } else {
-                    ComparisonMode::SizeMtime
-                }
-            }
-        };
-
-        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
-        //    the user filter applies through the universal pipeline chokepoint
-        //    (identical to push/pull/remote-remote behavior — full parity).
-        let inner: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root_buf.clone()));
-        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(inner, filter));
-        let unreadable = Arc::new(Mutex::new(Vec::new()));
-        let (mut header_rx, scan_handle) = source.scan(None, unreadable.clone());
-
-        // 2. Collect all headers
-        let mut all_headers = Vec::new();
-        while let Some(h) = header_rx.recv().await {
-            all_headers.push(h);
-        }
-        let _total_scanned = scan_handle
-            .await
-            .context("scan task panicked")?
-            .context("scan failed")?;
-
-        // 3. Diff + plan via the shared DiffPlanner stage. Combines
-        //    the comparison-filter and payload-planning steps that
-        //    were previously inline. Behavior preserved bit-for-bit
-        //    (size+mtime or Blake3 hash, then tar/large/raw planning).
-        let src = src_root_buf.clone();
-        let dst = dest_root_buf.clone();
-        let plan_opts = plan_options;
-        let headers = all_headers.clone();
-        let planned = tokio::task::spawn_blocking(move || {
-            plan_local_mirror(
-                headers,
-                LocalDiffInputs {
-                    src_root: &src,
-                    dst_root: &dst,
-                    compare_mode,
-                    ignore_existing,
-                    plan_options: plan_opts,
-                    skip_unchanged,
-                },
-            )
-        })
-        .await
-        .context("diff_planner task panicked")??;
+        // Local source, wrapped in FilteredSource so the user filter
+        // applies through the universal pipeline chokepoint (identical
+        // to push/pull/remote-remote behavior -- full parity).
+        let inner: Arc<dyn TransferSource> =
+            Arc::new(FsTransferSource::new(src_root.to_path_buf()));
+        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(
+            inner,
+            options.filter.clone_without_cache(),
+        ));
 
-        // 5. Create sink and execute unified pipeline
-        let sink: Arc<dyn TransferSink> = if copy_config.null_sink {
+        // Local sink. Construction is pure state (paths + config), so
+        // building it up front -- even for runs the engine resolves via
+        // a fast path that never touches it -- is behavior-neutral.
+        let compare_mode = options
+            .compare_mode
+            .resolve_comparison_mode(options.checksum);
+        let sink: Arc<dyn TransferSink> = if options.null_sink {
             Arc::new(NullSink::new())
         } else {
             Arc::new(FsTransferSink::new(
-                src_root_buf.clone(),
-                dest_root_buf.clone(),
+                src_root.to_path_buf(),
+                dest_root.to_path_buf(),
                 FsSinkConfig {
-                    preserve_times: copy_config.preserve_times,
-                    dry_run: copy_config.dry_run,
-                    checksum: copy_config.checksum,
-                    resume: copy_config.resume,
-                    // R58-followup: thread the orchestrator's
-                    // compare_mode into the sink. Pre-fix the sink
-                    // hard-coded SizeMtime via
+                    preserve_times: options.preserve_times,
+                    dry_run: options.dry_run,
+                    checksum: if options.checksum {
+                        Some(crate::checksum::ChecksumType::Blake3)
+                    } else {
+                        None
+                    },
+                    resume: options.resume,
+                    // R58-followup: thread the compare_mode into the
+                    // sink. Pre-fix the sink hard-coded SizeMtime via
                     // file_needs_copy_with_checksum_type, defeating
                     // --force / --ignore-times: the planner emitted
                     // the file but the sink decided "skip" when
@@ -597,531 +112,15 @@ impl TransferOrchestrator {
             ))
         };
 
-        // Boundary between planner and transfer phases. `planning_start`
-        // covers scan + diff + plan; everything after this `Instant`
-        // is the transfer pipeline. §2.8 phase 2 split: pre-fix the
-        // record's `planner_duration_ms` field was set to whole-run
-        // time, so the v1 predictor effectively trained on `planner =
-        // total` for both targets and couldn't distinguish them.
-        let plan_done = Instant::now();
-        let planner_duration_ms = plan_done.duration_since(planning_start).as_millis();
-
-        // §2.8 phase 2: query the predictor BEFORE running the
-        // pipeline. Surfaces in summary.predictor_estimate so
-        // `--verbose` and `blit profile --json` can compare
-        // predicted vs actual.
-        //
-        // R44-F1: query and observation must use the same feature
-        // vector. We query with `(scanned_files, scanned_bytes)`
-        // here; `record_performance_history` populates the matching
-        // `PerformanceRecord.{file_count,total_bytes}` from
-        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
-        // record was populated from `summary.copied_files`, so on
-        // any incremental run the predictor was queried with one
-        // workload size and trained against another.
-        //
-        // src_fs/dest_fs are left None for 0.1.0 — wiring
-        // `fs_capability` per-path probes into the predictor query
-        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
-        let scanned_files = all_headers.len();
-        let scanned_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
-        // R45 follow-up to R44-F1: never alias `total_bytes` to
-        // `scanned_bytes`. `summary.total_bytes` is the
-        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
-        // rustdoc); the predictor uses scan features only. Pre-fix
-        // this aliased the two so `summary.total_bytes` reported
-        // scanned bytes as bytes-written, overcounting throughput
-        // on incremental runs.
-        let predictor_estimate = predictor.as_ref().and_then(|p| {
-            let kind_total = crate::perf_predictor::DurationKind::Total;
-            let mode = if options.mirror {
-                crate::perf_history::TransferMode::Mirror
-            } else {
-                crate::perf_history::TransferMode::Copy
-            };
-            let total_pred = p.predict(
-                kind_total,
-                mode.clone(),
-                None,
-                None,
-                None,
-                options.skip_unchanged,
-                options.checksum,
-                scanned_files,
-                scanned_bytes,
-            )?;
-            // Pull planner + transfer separately too so the verbose
-            // line and the JSON profile can break down the estimate.
-            // All three predictor calls share the same
-            // (scanned_files, scanned_bytes) feature vector — both
-            // for consistency with the recording side, and so a
-            // future maintainer can't accidentally reintroduce a
-            // train/query mismatch by editing one branch and
-            // missing another.
-            let planner_pred = p
-                .predict(
-                    crate::perf_predictor::DurationKind::Planner,
-                    mode.clone(),
-                    None,
-                    None,
-                    None,
-                    options.skip_unchanged,
-                    options.checksum,
-                    scanned_files,
-                    scanned_bytes,
-                )
-                .map(|p| p.predicted_ms)
-                .unwrap_or(0.0);
-            let transfer_pred = p
-                .predict(
-                    crate::perf_predictor::DurationKind::Transfer,
-                    mode,
-                    None,
-                    None,
-                    None,
-                    options.skip_unchanged,
-                    options.checksum,
-                    scanned_files,
-                    scanned_bytes,
-                )
-                .map(|p| p.predicted_ms)
-                .unwrap_or(0.0);
-            Some(super::summary::PredictorEstimate {
-                planner_ms: planner_pred.max(0.0) as u128,
-                transfer_ms: transfer_pred.max(0.0) as u128,
-                total_ms: total_pred.predicted_ms.max(0.0) as u128,
-                observations: total_pred.observations,
-                fallback_depth: total_pred.fallback_depth,
+        TransferEngine::new()
+            .execute(EngineRequest {
+                src_root: src_root.to_path_buf(),
+                dest_root: dest_root.to_path_buf(),
+                source,
+                sink,
+                options,
             })
-        });
-        if options.verbose {
-            if let Some(est) = predictor_estimate.as_ref() {
-                eprintln!(
-                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
-                     total ~{} ms (n={}, fallback_depth={})",
-                    est.planner_ms,
-                    est.transfer_ms,
-                    est.total_ms,
-                    est.observations,
-                    est.fallback_depth
-                );
-            } else {
-                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
-            }
-        }
-
-        let pipeline_outcome = execute_sink_pipeline(
-            source,
-            vec![sink],
-            planned.payloads,
-            DEFAULT_PAYLOAD_PREFETCH,
-            None,
-        )
-        .await
-        .context("transfer pipeline failed")?;
-        let transfer_duration_ms = plan_done.elapsed().as_millis();
-
-        // R47-F4: snapshot unreadable paths so the CLI's source-
-        // delete step (in `blit move`) can refuse to remove a
-        // source it couldn't fully scan. The R46-F2 gate inside
-        // the orchestrator only fires on `options.mirror`, but
-        // move uses mirror=false — without this surface, an
-        // unreadable source file would get skipped during the
-        // copy and then silently deleted from the source by the
-        // CLI's `remove_dir_all` step.
-        let unreadable_snapshot: Vec<String> = unreadable
-            .lock()
-            .map(|guard| guard.clone())
-            .unwrap_or_default();
-
-        let mut summary = LocalMirrorSummary {
-            planned_files: pipeline_outcome.files_written,
-            copied_files: pipeline_outcome.files_written,
-            // R45: bytes the pipeline actually wrote, not scanned
-            // bytes. Distinct on incremental runs.
-            total_bytes: pipeline_outcome.bytes_written,
-            scanned_files,
-            scanned_bytes,
-            dry_run: options.dry_run,
-            duration: start_time.elapsed(),
-            predictor_estimate: predictor_estimate.clone(),
-            unreadable_paths: unreadable_snapshot.clone(),
-            ..Default::default()
-        };
-
-        if options.mirror {
-            // R46-F2: refuse to mirror-delete when the source scan
-            // was incomplete. The `unreadable_snapshot` captured
-            // above (R47-F4) covers the per-file open path
-            // (PermissionDenied / NotFound on individual files) and
-            // the walkdir non-root error path (unreadable
-            // subdirectories). Either case means the header set
-            // we're about to use as the source-of-truth for "what
-            // the destination should contain" is missing entries,
-            // and a delete pass would silently remove matching
-            // destination subtrees.
-            if !unreadable_snapshot.is_empty() {
-                bail!(
-                    "refusing to mirror-delete from {}: source scan was \
-                     incomplete ({} unreadable entr{}); the first {} \
-                     reported: {}. Resolve the scan errors (typically \
-                     permissions) or run as a non-mirror copy.",
-                    dest_root.display(),
-                    unreadable_snapshot.len(),
-                    if unreadable_snapshot.len() == 1 {
-                        "y"
-                    } else {
-                        "ies"
-                    },
-                    unreadable_snapshot.len().min(5),
-                    unreadable_snapshot
-                        .iter()
-                        .take(5)
-                        .cloned()
-                        .collect::<Vec<_>>()
-                        .join("; "),
-                );
-            }
-
-            let source_paths: HashSet<String> = all_headers
-                .iter()
-                .map(|h| h.relative_path.clone())
-                .collect();
-            let deletions = apply_mirror_deletions(
-                &source_paths,
-                dest_root,
-                &options.filter,
-                options.delete_scope,
-                !options.dry_run,
-                options.verbose,
-            )?;
-            summary.deleted_files = deletions.0;
-            summary.deleted_dirs = deletions.1;
-        }
-
-        if let Some(tracker) = journal_tracker.as_mut() {
-            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
-        }
-
-        if options.verbose {
-            eprintln!(
-                "Planning enumerated {} file(s), {} bytes",
-                scanned_files, scanned_bytes
-            );
-            eprintln!(
-                "Completed local {}: {} file(s), {} bytes in {:.2?} (plan {} ms, xfer {} ms)",
-                if options.mirror { "mirror" } else { "copy" },
-                summary.copied_files,
-                summary.total_bytes,
-                summary.duration,
-                planner_duration_ms,
-                transfer_duration_ms,
-            );
-            // §2.8: side-by-side predicted-vs-actual so operators
-            // can audit the predictor against this run's actual
-            // numbers. The bare percentage error per phase is the
-            // most useful single number; we keep absolute ms in the
-            // line above for context.
-            if let Some(est) = summary.predictor_estimate.as_ref() {
-                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
-                    if actual_ms == 0 {
-                        "n/a".to_string()
-                    } else {
-                        let pred = predicted_ms as f64;
-                        let act = actual_ms as f64;
-                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
-                    }
-                };
-                eprintln!(
-                    "Predictor delta: planner {} ({} vs {} ms), \
-                     transfer {} ({} vs {} ms)",
-                    pct(est.planner_ms, planner_duration_ms),
-                    est.planner_ms,
-                    planner_duration_ms,
-                    pct(est.transfer_ms, transfer_duration_ms),
-                    est.transfer_ms,
-                    transfer_duration_ms,
-                );
-            }
-        }
-
-        let fast_path_label = if options.null_sink {
-            Some("null_sink")
-        } else {
-            None
-        };
-        if let Some(record) = record_performance_history(
-            &summary,
-            &options,
-            fast_path_label,
-            planner_duration_ms,
-            transfer_duration_ms,
-        ) {
-            // Don't update the predictor from null-sink runs — the zero
-            // write cost would teach it that transfers are faster than
-            // they really are.
-            if !options.null_sink {
-                update_predictor(&mut predictor, &record, options.verbose);
-            }
-        }
-
-        Ok(summary)
-    }
-}
-
-/// Delete destination files/dirs not present in the source header set.
-///
-/// R58-F6: `delete_scope` controls which destination entries are
-/// even considered for deletion:
-///   - `FilteredSubset` (default): enumerate the destination
-///     *through the user's filter*, then delete entries not in
-///     the source set. Excluded files (e.g. `*.log` when
-///     `--exclude '*.log'`) are out of scope — they're not
-///     candidates for deletion, and their parent directories are
-///     therefore non-empty from the user's perspective. When
-///     `remove_dir` fails with ENOTEMPTY on a parent whose only
-///     remaining contents are out-of-scope, we treat it as
-///     expected, not as an error.
-///   - `All`: enumerate the destination *without* the filter so
-///     every entry is in scope. ENOTEMPTY is a genuine error
-///     here (we did walk everything, so something other than
-///     filter-excluded content must be in the way).
-fn apply_mirror_deletions(
-    source_paths: &HashSet<String>,
-    dest_root: &Path,
-    filter: &FileFilter,
-    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
-    perform: bool,
-    verbose: bool,
-) -> Result<(usize, usize)> {
-    use crate::enumeration::{EntryKind, FileEnumerator};
-    use crate::orchestrator::LocalMirrorDeleteScope;
-
-    // R58-F6: FilteredSubset uses the user's filter for the
-    // enumeration (only in-scope entries become deletion
-    // candidates). All bypasses the filter so every destination
-    // entry is considered.
-    let enum_filter = match delete_scope {
-        LocalMirrorDeleteScope::FilteredSubset => filter.clone_without_cache(),
-        LocalMirrorDeleteScope::All => FileFilter::default(),
-    };
-    let enumerator = FileEnumerator::new(enum_filter);
-    let dest_entries = enumerator.enumerate_local(dest_root)?;
-
-    // R48-F1: source.scan() only emits file headers, so
-    // `source_paths` is a set of *files*. Pre-fix this meant every
-    // destination directory was "not in source_paths" and got
-    // queued for deletion. Combined with R46-F5's hard-error
-    // policy on remove_* failures, a normal mirror containing
-    // `sub/file.txt` would keep `sub/file.txt`, then try
-    // `remove_dir("sub")` and fail the whole operation with
-    // ENOTEMPTY. Derive `source_dirs` from each file's parent
-    // chain so dest dirs that exist implicitly on the source
-    // side (because they contain a source file) get preserved.
-    let mut source_dirs: HashSet<String> = HashSet::new();
-    for path in source_paths {
-        let p = std::path::Path::new(path);
-        let mut cur = p.parent();
-        while let Some(parent) = cur {
-            if parent.as_os_str().is_empty() {
-                break;
-            }
-            let parent_str = crate::path_posix::relative_path_to_posix(parent);
-            // Insert and keep walking up; if already present every
-            // shallower ancestor is too, so we could break — but
-            // the walk is cheap and the eager form is simpler to
-            // reason about.
-            source_dirs.insert(parent_str);
-            cur = parent.parent();
-        }
-    }
-
-    let mut files_to_delete = Vec::new();
-    let mut dirs_to_delete = Vec::new();
-
-    for entry in &dest_entries {
-        let rel = crate::path_posix::relative_path_to_posix(&entry.relative_path);
-        let absent_at_source = match entry.kind {
-            EntryKind::Directory => !source_dirs.contains(&rel),
-            _ => !source_paths.contains(&rel),
-        };
-        if absent_at_source {
-            let abs = dest_root.join(&entry.relative_path);
-            match entry.kind {
-                EntryKind::Directory => dirs_to_delete.push(abs),
-                _ => files_to_delete.push(abs),
-            }
-        }
-    }
-
-    // Sort dirs deepest-first so children are deleted before parents.
-    dirs_to_delete.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
-
-    let mut deleted_files = 0usize;
-    let mut deleted_dirs = 0usize;
-    // R46-F5: collect deletion failures and bail at the end. Pre-fix
-    // each `remove_file` / `remove_dir` error was printed as a
-    // warning and the function returned Ok, so a mirror could
-    // succeed-on-paper while leaving stale destination content
-    // behind. Now we still attempt every deletion (better partial
-    // progress than abort-on-first-failure), but we bail with an
-    // aggregated error if any failed — the caller's mirror operation
-    // returns Err, the user sees the failed entries, and the summary
-    // line doesn't claim "complete".
-    let mut failures: Vec<String> = Vec::new();
-
-    for path in files_to_delete {
-        #[cfg(windows)]
-        crate::win_fs::clear_readonly_recursive(&path);
-
-        if perform {
-            match std::fs::remove_file(&path) {
-                Ok(_) => {
-                    deleted_files += 1;
-                    if verbose {
-                        eprintln!("Deleted file: {}", path.display());
-                    }
-                }
-                Err(err) => {
-                    eprintln!("Failed to delete file {}: {}", path.display(), err);
-                    failures.push(format!("{}: {}", path.display(), err));
-                }
-            }
-        } else {
-            deleted_files += 1;
-        }
-    }
-
-    for path in dirs_to_delete {
-        #[cfg(windows)]
-        crate::win_fs::clear_readonly_recursive(&path);
-
-        if perform {
-            match std::fs::remove_dir(&path) {
-                Ok(_) => {
-                    deleted_dirs += 1;
-                    if verbose {
-                        eprintln!("Deleted directory: {}", path.display());
-                    }
-                }
-                Err(err) => {
-                    // R58-F6: in FilteredSubset mode, ENOTEMPTY on
-                    // a destination dir means the dir contains
-                    // out-of-scope content (files matching the
-                    // user's exclude rules). Those files
-                    // intentionally aren't candidates for
-                    // deletion, so the dir genuinely can't be
-                    // empty — that's not a failure, it's the
-                    // expected behavior of the scope contract.
-                    // Skip silently in that case; surface the
-                    // error in `All` mode where the dir really
-                    // should have been empty.
-                    let is_not_empty = err.kind() == std::io::ErrorKind::DirectoryNotEmpty
-                        || err.raw_os_error() == Some(66); // ENOTEMPTY on macOS/BSD
-                    if matches!(delete_scope, LocalMirrorDeleteScope::FilteredSubset)
-                        && is_not_empty
-                    {
-                        if verbose {
-                            eprintln!(
-                                "Kept directory {} (contains out-of-scope contents)",
-                                path.display()
-                            );
-                        }
-                    } else {
-                        eprintln!("Failed to delete directory {}: {}", path.display(), err);
-                        failures.push(format!("{}: {}", path.display(), err));
-                    }
-                }
-            }
-        } else {
-            deleted_dirs += 1;
-        }
-    }
-
-    if !failures.is_empty() {
-        let preview = failures
-            .iter()
-            .take(5)
-            .cloned()
-            .collect::<Vec<_>>()
-            .join("; ");
-        bail!(
-            "mirror-delete left {} entr{} in place at {} ({} succeeded): {}",
-            failures.len(),
-            if failures.len() == 1 { "y" } else { "ies" },
-            dest_root.display(),
-            deleted_files + deleted_dirs,
-            preview
-        );
-    }
-
-    Ok((deleted_files, deleted_dirs))
-}
-
-fn persist_journal_checkpoints(
-    tracker: &mut ChangeTracker,
-    tokens: &mut [ProbeToken],
-    verbose: bool,
-) {
-    if tokens.is_empty() {
-        return;
-    }
-
-    for token in tokens.iter_mut() {
-        match tracker.reprobe_canonical(&token.canonical_path) {
-            Ok(snapshot) => token.snapshot = snapshot,
-            Err(err) => {
-                token.snapshot = None;
-                if verbose {
-                    eprintln!(
-                        "Failed to refresh journal snapshot for {}: {err:?}",
-                        token.canonical_path.display()
-                    );
-                }
-            }
-        }
-    }
-
-    if let Err(err) = tracker.refresh_and_persist(tokens) {
-        if verbose {
-            eprintln!("Failed to update journal checkpoint: {err:?}");
-        }
-    }
-}
-
-fn log_probe(label: &str, probe: &ProbeToken) {
-    eprintln!(
-        "Journal probe {label} state={:?} snapshot={} path={}",
-        probe.state,
-        probe.snapshot.is_some(),
-        probe.canonical_path.display()
-    );
-
-    if let Some(snapshot) = &probe.snapshot {
-        match snapshot {
-            StoredSnapshot::Windows(snap) => {
-                eprintln!(
-                    "  {label} windows: volume={} journal_id={} next_usn={} mtime={:?}",
-                    snap.volume, snap.journal_id, snap.next_usn, snap.root_mtime_epoch_ms
-                );
-            }
-            StoredSnapshot::MacOs(snap) => {
-                eprintln!(
-                    "  {label} macOS: fsid={} event_id={}",
-                    snap.fsid, snap.event_id
-                );
-            }
-            StoredSnapshot::Linux(snap) => {
-                eprintln!(
-                    "  {label} linux: device={} inode={} ctime={}s+{}ns mtime={:?}",
-                    snap.device,
-                    snap.inode,
-                    snap.ctime_sec,
-                    snap.ctime_nsec,
-                    snap.root_mtime_epoch_ms
-                );
-            }
-        }
+            .await
     }
 }
 
@@ -1131,172 +130,6 @@ impl Default for TransferOrchestrator {
     }
 }
 
-/// Copy a single file source directly to `dest_root`, bypassing the
-/// enumerator/planner/pipeline machinery which assumes `src_root` is a
-/// directory. The CLI's destination resolver has already produced the final
-/// target path, so this is a simple `copy_file` call.
-fn execute_single_file_copy(
-    src_root: &Path,
-    dest_root: &Path,
-    options: &LocalMirrorOptions,
-    start_time: Instant,
-) -> Result<LocalMirrorSummary> {
-    use crate::buffer::BufferSizer;
-    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
-    use crate::logger::NoopLogger;
-    use filetime::FileTime;
-
-    let src_meta = std::fs::metadata(src_root)
-        .with_context(|| format!("stat source file {}", src_root.display()))?;
-    let size = src_meta.len();
-
-    // R58-followup: route compare-mode for the single-file path
-    // through the same translation the directory path uses
-    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
-    // at `options.checksum`, so `--size-only` / `--ignore-times` /
-    // `--force` were silently dropped — repro: copy src.txt dst.txt
-    // --size-only re-copied even when sizes matched.
-    let compare_mode = match options.compare_mode {
-        crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
-        crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
-        crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
-        crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
-        crate::orchestrator::LocalCompareMode::SizeMtime => {
-            if options.checksum {
-                ComparisonMode::Checksum
-            } else {
-                ComparisonMode::SizeMtime
-            }
-        }
-    };
-
-    // R58-F5: the single-file short-circuit (orchestrator.rs:125)
-    // bypasses the enumerator + planner, which is where the
-    // streaming-pipeline path checks filter / ignore_existing.
-    // Apply both here so single-file copies honor the same
-    // CLI contract.
-    //
-    // Filter: the source root is itself the only entry. Run
-    // `filter.allows_entry` against the source name. If excluded,
-    // return a "scanned 1 / copied 0" summary so the user sees
-    // "no work performed" rather than the file being copied
-    // anyway.
-    let src_name = src_root.file_name().map(PathBuf::from);
-    let allows = match src_name {
-        Some(name) => {
-            let mtime = src_meta.modified().ok();
-            options
-                .filter
-                .allows_entry(Some(&name), src_root, size, mtime)
-        }
-        None => true,
-    };
-    if !allows {
-        return Ok(LocalMirrorSummary {
-            planned_files: 0,
-            copied_files: 0,
-            total_bytes: 0,
-            scanned_files: 1,
-            scanned_bytes: size,
-            duration: start_time.elapsed(),
-            outcome: TransferOutcome::UpToDate,
-            ..Default::default()
-        });
-    }
-
-    // ignore_existing: if the destination file already exists,
-    // skip the copy entirely. Matches the diff_planner behavior
-    // for the streaming-pipeline path (diff_planner.rs).
-    if options.ignore_existing && dest_root.exists() {
-        return Ok(LocalMirrorSummary {
-            planned_files: 0,
-            copied_files: 0,
-            total_bytes: 0,
-            scanned_files: 1,
-            scanned_bytes: size,
-            duration: start_time.elapsed(),
-            outcome: TransferOutcome::UpToDate,
-            ..Default::default()
-        });
-    }
-
-    if options.dry_run {
-        return Ok(LocalMirrorSummary {
-            planned_files: 1,
-            copied_files: 1,
-            total_bytes: size,
-            scanned_files: 1,
-            scanned_bytes: size,
-            dry_run: true,
-            duration: start_time.elapsed(),
-            ..Default::default()
-        });
-    }
-
-    if options.null_sink {
-        return Ok(LocalMirrorSummary {
-            planned_files: 1,
-            copied_files: 1,
-            total_bytes: size,
-            scanned_files: 1,
-            scanned_bytes: size,
-            duration: start_time.elapsed(),
-            ..Default::default()
-        });
-    }
-
-    let mut did_copy = false;
-    let mut clone_succeeded = false;
-    let mut bytes_copied = 0u64;
-
-    if options.resume {
-        let outcome = resume_copy_file(src_root, dest_root, 0)
-            .with_context(|| format!("resume copy {}", src_root.display()))?;
-        did_copy = outcome.bytes_transferred > 0;
-        bytes_copied = outcome.bytes_transferred;
-    } else {
-        let needs_copy = !options.skip_unchanged
-            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
-        if needs_copy {
-            let sizer = BufferSizer::default();
-            let logger = NoopLogger;
-            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
-                .with_context(|| format!("copy {}", src_root.display()))?;
-            did_copy = true;
-            clone_succeeded = outcome.clone_succeeded;
-            bytes_copied = outcome.bytes_copied;
-        }
-    }
-
-    if options.preserve_times && did_copy && !clone_succeeded {
-        if let Ok(modified) = src_meta.modified() {
-            let ft = FileTime::from_system_time(modified);
-            // R42-F1: warn-don't-silence (was `let _ = ...`).
-            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
-                log::warn!("set mtime on {}: {}", dest_root.display(), e);
-            }
-        }
-    }
-
-    Ok(LocalMirrorSummary {
-        planned_files: 1,
-        copied_files: if did_copy { 1 } else { 0 },
-        total_bytes: bytes_copied,
-        // Single-file path always saw exactly one entry of `size`
-        // bytes; whether we copied it or not is the
-        // copied_files/total_bytes story, but the scan saw it.
-        scanned_files: 1,
-        scanned_bytes: size,
-        duration: start_time.elapsed(),
-        outcome: if did_copy {
-            TransferOutcome::Transferred
-        } else {
-            TransferOutcome::UpToDate
-        },
-        ..Default::default()
-    })
-}
-
 #[cfg(test)]
 mod async_runtime_tests {
     //! F9 regression: `execute_local_mirror_async` must be callable
@@ -1967,500 +800,3 @@ mod async_runtime_tests {
         }
     }
 }
-
-#[cfg(test)]
-mod select_tuning_window_tests {
-    //! R56-F2: ensure non-real records are filtered BEFORE the
-    //! 20-record window, not after. Pre-fix, recent
-    //! dry-run/null-sink records with matching mode could fill the
-    //! window and force tuning to fall back to defaults even when
-    //! older real records existed.
-
-    use super::*;
-    use crate::perf_history::{
-        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
-    };
-
-    fn record(
-        kind: RunKind,
-        mode: TransferMode,
-        tar_tasks: u32,
-        tar_bytes: u64,
-        timestamp_ms: u128,
-    ) -> PerformanceRecord {
-        let mut r = PerformanceRecord::new(
-            mode,
-            None,
-            None,
-            10,
-            1024,
-            OptionSnapshot {
-                dry_run: false,
-                preserve_symlinks: true,
-                include_symlinks: false,
-                skip_unchanged: true,
-                checksum: false,
-                compare_mode: CompareModeSnapshot::SizeMtime,
-                workers: 4,
-            },
-            None,
-            10,
-            100,
-            0,
-            0,
-        );
-        r.run_kind = kind;
-        r.tar_shard_tasks = tar_tasks;
-        r.tar_shard_files = tar_tasks * 100;
-        r.tar_shard_bytes = tar_bytes;
-        r.timestamp_epoch_ms = timestamp_ms;
-        r
-    }
-
-    /// 30 recent NullSink records (matching the target operation
-    /// shape) followed by 5 older Real records. Pre-fix .take(20)
-    /// ran first, grabbed 20 NullSinks, derive_local_plan_tuning
-    /// skipped them all internally and returned None — tuning
-    /// fell back to defaults despite real history being available.
-    /// Post-fix, the filter eats the NullSinks before the take, so
-    /// the 5 Real records make it through and tuning succeeds.
-    #[test]
-    fn null_sink_records_do_not_crowd_out_older_real_records() {
-        let mut history = Vec::new();
-        // Older real records (timestamps lowest = oldest).
-        for i in 0..5 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Copy,
-                4,
-                16 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        // Recent null-sink records (higher timestamps = more recent).
-        for i in 0..30 {
-            history.push(record(
-                RunKind::NullSink,
-                TransferMode::Copy,
-                4,
-                512 * 1024 * 1024,
-                10_000 + i,
-            ));
-        }
-
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert!(
-            !window.is_empty(),
-            "real records must reach the window; 30 NullSink records crowded them out pre-R56-F2"
-        );
-        assert!(
-            window.iter().all(|r| r.run_kind.is_real_transfer()),
-            "only Real records should land in the tuning window"
-        );
-        // derive_local_plan_tuning succeeds → tuner sees its 5 Real
-        // records with 16 MiB tar bytes / 4 tar tasks = 4 MiB avg
-        // (clamped to the 4 MiB floor).
-        let tuning = derive_local_plan_tuning(&window).expect("tuning must succeed");
-        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
-        assert!(tuning.small_target_bytes <= 16 * 1024 * 1024);
-    }
-
-    #[test]
-    fn dry_run_records_do_not_crowd_out_real_records() {
-        let mut history = Vec::new();
-        for i in 0..3 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Copy,
-                2,
-                8 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        for i in 0..25 {
-            history.push(record(
-                RunKind::DryRun,
-                TransferMode::Copy,
-                10,
-                1024 * 1024 * 1024,
-                10_000 + i,
-            ));
-        }
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(
-            window.len(),
-            3,
-            "expected the 3 real records, got {} entries",
-            window.len()
-        );
-        assert!(derive_local_plan_tuning(&window).is_some());
-    }
-
-    #[test]
-    fn bench_records_do_not_crowd_out_real_records() {
-        let mut history = Vec::new();
-        for i in 0..2 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Copy,
-                1,
-                4 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        for i in 0..50 {
-            history.push(record(
-                RunKind::BenchTransfer,
-                TransferMode::Copy,
-                100,
-                512 * 1024 * 1024,
-                10_000 + i,
-            ));
-        }
-        for i in 0..50 {
-            history.push(record(
-                RunKind::BenchWire,
-                TransferMode::Copy,
-                100,
-                512 * 1024 * 1024,
-                20_000 + i,
-            ));
-        }
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(window.len(), 2);
-        assert!(window.iter().all(|r| r.run_kind == RunKind::Real));
-    }
-
-    /// Sanity: with abundant real records, the window caps at 20.
-    #[test]
-    fn window_caps_at_20_real_records() {
-        let history: Vec<_> = (0..50)
-            .map(|i| {
-                record(
-                    RunKind::Real,
-                    TransferMode::Copy,
-                    2,
-                    8 * 1024 * 1024,
-                    100 + i,
-                )
-            })
-            .collect();
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(window.len(), 20, "expected the 20 most recent real records");
-    }
-
-    /// R57-F1 regression: the call site is now
-    /// `select_tuning_window_from_history` which bakes the
-    /// "ask for all records" invariant into the wrapper — see
-    /// the dedicated tests below for the synthetic-reader
-    /// regression that catches a future drift back to a finite
-    /// limit. The pure-helper test below verifies that the
-    /// in-function logic copes with arbitrarily large histories
-    /// even if the wrapper were bypassed.
-    #[test]
-    fn handles_large_history_with_non_real_records_at_the_front() {
-        let mut history = Vec::new();
-        // 200 recent NullSink records (would have fit inside the
-        // old 50-record pre-cap with room to spare).
-        for i in 0..200 {
-            history.push(record(
-                RunKind::NullSink,
-                TransferMode::Copy,
-                4,
-                512 * 1024 * 1024,
-                10_000 + i,
-            ));
-        }
-        // 5 older Real records (would never have been seen with
-        // pre-cap=50, since the 200 NullSinks alone exceed it).
-        for i in 0..5 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Copy,
-                4,
-                16 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        // Real records were appended last (highest timestamps);
-        // select_tuning_window iterates .rev() so they come first.
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(
-            window.len(),
-            5,
-            "expected the 5 real records to survive a flood of non-real history"
-        );
-        assert!(window.iter().all(|r| r.run_kind.is_real_transfer()));
-        assert!(derive_local_plan_tuning(&window).is_some());
-    }
-
-    /// R58-followup: Real records with no tuning signal
-    /// (`tar_shard_tasks == 0 && raw_bundle_tasks == 0`) must not
-    /// crowd out older bucket-bearing records. These exist when a
-    /// run took the no_work / journal_no_work / single_huge_file
-    /// fast-path or was a streaming run that copied nothing — they
-    /// pass `is_real_transfer`, pass the per-operation discriminants,
-    /// pass the !=tiny_manifest gate, but contribute zero to
-    /// `derive_local_plan_tuning`. Pre-fix the 20-record window
-    /// could fill with them and the tuner fell back to defaults.
-    #[test]
-    fn no_signal_real_records_do_not_crowd_out_bucket_bearing_records() {
-        let mut history = Vec::new();
-        // 5 older Real records WITH bucket signal (timestamps lowest).
-        for i in 0..5 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Copy,
-                4,
-                16 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        // 30 recent Real records WITHOUT bucket signal: tar_tasks=0,
-        // bytes=0 — same shape `single_huge_file` / `no_work` /
-        // `journal_no_work` / streaming-no-op records produce.
-        for i in 0..30 {
-            let mut r = record(RunKind::Real, TransferMode::Copy, 0, 0, 10_000 + i);
-            // Vary fast_path across the no-signal categories to
-            // mirror real history. None of these exclude the record
-            // from the existing gates.
-            r.fast_path = match i % 4 {
-                0 => Some("no_work".to_string()),
-                1 => Some("journal_no_work".to_string()),
-                2 => Some("single_huge_file".to_string()),
-                _ => None,
-            };
-            history.push(r);
-        }
-
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert!(
-            !window.is_empty(),
-            "older bucket-bearing records must reach the window; \
-             30 no-signal Real records crowded them out pre-fix"
-        );
-        assert!(
-            window
-                .iter()
-                .all(|r| r.tar_shard_tasks > 0 || r.raw_bundle_tasks > 0),
-            "every record in the window must carry a tuning signal"
-        );
-        assert!(
-            derive_local_plan_tuning(&window).is_some(),
-            "tuner must return a value, not fall back to defaults"
-        );
-    }
-
-    // ── R57-F1: wrapper's "ask for all records" invariant ────────────
-    //
-    // The bug class isn't about what `select_tuning_window` does
-    // with a slice; it's about which slice the caller passes in.
-    // `select_tuning_window_from_history` wraps the reader call so
-    // a future maintainer can't drift the limit back to a finite
-    // value. These tests catch that drift by asserting on the
-    // limit value the wrapper passes to its reader.
-
-    use std::cell::Cell;
-    use std::rc::Rc;
-
-    /// Captures the `limit` argument every call to the reader.
-    /// The reader returns a fixed slice; we just want to see what
-    /// the wrapper asks for.
-    fn recording_reader(
-        captured_limit: Rc<Cell<Option<usize>>>,
-        records: Vec<PerformanceRecord>,
-    ) -> impl FnOnce(usize) -> Result<Vec<PerformanceRecord>> {
-        move |limit| {
-            captured_limit.set(Some(limit));
-            Ok(records)
-        }
-    }
-
-    #[test]
-    fn wrapper_passes_zero_to_reader() {
-        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
-        let reader = recording_reader(captured.clone(), vec![]);
-        let _ = select_tuning_window_from_history(
-            reader,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(
-            captured.get(),
-            Some(0),
-            "R57-F1: the wrapper must ask for all records (limit=0); any \
-             finite limit reintroduces the JSONL-layer crowd-out bug"
-        );
-    }
-
-    #[test]
-    fn wrapper_returns_none_when_reader_errors() {
-        let reader = |_limit: usize| -> Result<Vec<PerformanceRecord>> {
-            Err(eyre!("simulated read failure"))
-        };
-        let result = select_tuning_window_from_history(
-            reader,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert!(result.is_none());
-    }
-
-    #[test]
-    fn wrapper_returns_none_when_no_eligible_records() {
-        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
-        let reader = recording_reader(
-            captured,
-            vec![
-                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 100),
-                record(RunKind::NullSink, TransferMode::Copy, 4, 1024 * 1024, 200),
-            ],
-        );
-        let result = select_tuning_window_from_history(
-            reader,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert!(result.is_none());
-    }
-
-    #[test]
-    fn wrapper_returns_some_window_when_real_records_present() {
-        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
-        let reader = recording_reader(
-            captured.clone(),
-            vec![
-                record(RunKind::Real, TransferMode::Copy, 4, 16 * 1024 * 1024, 100),
-                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 200),
-            ],
-        );
-        let result = select_tuning_window_from_history(
-            reader,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        )
-        .unwrap();
-        assert_eq!(result.len(), 1);
-        assert_eq!(result[0].run_kind, RunKind::Real);
-        assert_eq!(captured.get(), Some(0));
-    }
-
-    /// Sanity: mode and option filters still apply post-R56-F2.
-    /// A Real record with the wrong mode/checksum/skip_unchanged
-    /// must NOT land in the window.
-    #[test]
-    fn mode_and_option_filters_still_apply() {
-        let mut history = Vec::new();
-        // Real Mirror records (wrong mode).
-        for i in 0..10 {
-            history.push(record(
-                RunKind::Real,
-                TransferMode::Mirror,
-                4,
-                16 * 1024 * 1024,
-                100 + i,
-            ));
-        }
-        // Real Copy record.
-        history.push(record(
-            RunKind::Real,
-            TransferMode::Copy,
-            2,
-            8 * 1024 * 1024,
-            500,
-        ));
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(window.len(), 1);
-        assert_eq!(window[0].mode, TransferMode::Copy);
-    }
-
-    /// R59 finding #5: SizeOnly / Force / IgnoreTimes runs must
-    /// not contaminate the SizeMtime tuning bucket. Pre-fix the
-    /// window filtered on `options.checksum == checksum_bool`, so a
-    /// `--size-only` run (checksum=false) landed in the same bucket
-    /// as a default `SizeMtime` run.
-    #[test]
-    fn compare_mode_buckets_are_separate() {
-        let mut history = Vec::new();
-        // 10 SizeOnly Real records (signal-bearing).
-        for i in 0..10 {
-            let mut r = record(
-                RunKind::Real,
-                TransferMode::Copy,
-                4,
-                16 * 1024 * 1024,
-                100 + i,
-            );
-            r.options.compare_mode = CompareModeSnapshot::SizeOnly;
-            history.push(r);
-        }
-        // One SizeMtime Real record.
-        let mut sm = record(RunKind::Real, TransferMode::Copy, 2, 8 * 1024 * 1024, 500);
-        sm.options.compare_mode = CompareModeSnapshot::SizeMtime;
-        history.push(sm);
-
-        // Querying SizeMtime must NOT pick up the 10 SizeOnly records.
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeMtime,
-            true,
-        );
-        assert_eq!(window.len(), 1);
-        assert_eq!(
-            window[0].options.compare_mode,
-            CompareModeSnapshot::SizeMtime
-        );
-
-        // Querying SizeOnly returns the SizeOnly records.
-        let window = select_tuning_window(
-            &history,
-            TransferMode::Copy,
-            CompareModeSnapshot::SizeOnly,
-            true,
-        );
-        assert_eq!(window.len(), 10);
-        assert!(window
-            .iter()
-            .all(|r| r.options.compare_mode == CompareModeSnapshot::SizeOnly));
-    }
-}

exec
/usr/bin/zsh -lc 'git diff --find-renames --color=never 63b813a 29e210b -- crates/blit-core/tests/local_transfers.rs crates/blit-core/src/orchestrator/mod.rs crates/blit-core/src/lib.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/lib.rs b/crates/blit-core/src/lib.rs
index 033ea3b..828a43b 100644
--- a/crates/blit-core/src/lib.rs
+++ b/crates/blit-core/src/lib.rs
@@ -5,6 +5,7 @@ pub mod checksum;
 pub mod config;
 pub mod copy;
 pub mod delete;
+pub mod engine;
 pub mod enumeration;
 pub mod fs_capability;
 pub mod fs_enum;
diff --git a/crates/blit-core/src/orchestrator/mod.rs b/crates/blit-core/src/orchestrator/mod.rs
index 1389c3e..60c8e49 100644
--- a/crates/blit-core/src/orchestrator/mod.rs
+++ b/crates/blit-core/src/orchestrator/mod.rs
@@ -1,9 +1,7 @@
-mod fast_path;
-mod history;
-mod options;
 mod orchestrator;
-mod summary;
 
-pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
+pub use crate::engine::{
+    LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions, LocalMirrorSummary,
+    TransferOutcome,
+};
 pub use orchestrator::TransferOrchestrator;
-pub use summary::{LocalMirrorSummary, TransferOutcome};
diff --git a/crates/blit-core/tests/local_transfers.rs b/crates/blit-core/tests/local_transfers.rs
index 91d86a9..79aec42 100644
--- a/crates/blit-core/tests/local_transfers.rs
+++ b/crates/blit-core/tests/local_transfers.rs
@@ -68,6 +68,122 @@ fn tiny_manifest_records_fast_path() -> Result<()> {
     Ok(())
 }
 
+/// ue-r2-1c behavior pin (added before the engine move): a second run
+/// over an already-synced directory routes through
+/// `FastPathDecision::NoWork{examined > 0}`, reports
+/// `TransferOutcome::UpToDate`, and records the `no_work` perf-history
+/// tag. Previously this strategy had no test at all.
+#[test]
+fn up_to_date_second_run_records_no_work() -> Result<()> {
+    use blit_core::orchestrator::TransferOutcome;
+
+    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
+    let _guard = ConfigDirGuard::new()?;
+    perf_history::set_perf_history_enabled(true)?;
+    let _ = perf_history::clear_history()?;
+
+    let tmp = tempdir()?;
+    let src = tmp.path().join("src");
+    let dest = tmp.path().join("dest");
+    fs::create_dir_all(&src)?;
+    fs::write(src.join("a.txt"), b"one")?;
+    fs::write(src.join("b.txt"), b"two")?;
+
+    let options = || LocalMirrorOptions {
+        progress: false,
+        perf_history: true,
+        // preserve_times keeps mtimes matching so the second run's
+        // size+mtime comparison sees both files as unchanged.
+        preserve_times: true,
+        ..Default::default()
+    };
+
+    let orchestrator = TransferOrchestrator::new();
+    let first = orchestrator.execute_local_mirror(&src, &dest, options())?;
+    assert_eq!(first.copied_files, 2);
+
+    let second = orchestrator.execute_local_mirror(&src, &dest, options())?;
+    assert_eq!(second.copied_files, 0);
+    assert_eq!(second.outcome, TransferOutcome::UpToDate);
+    assert!(
+        second.scanned_files >= 2,
+        "NoWork must report examined files"
+    );
+
+    let records = perf_history::read_recent_records(0)?;
+    let last = records.last().expect("expected perf history record");
+    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
+    Ok(())
+}
+
+/// ue-r2-1c behavior pin (added before the engine move): an empty
+/// source directory routes through `NoWork{examined: 0}` and reports
+/// `TransferOutcome::SourceEmpty`. Previously untested.
+#[test]
+fn empty_source_dir_reports_source_empty() -> Result<()> {
+    use blit_core::orchestrator::TransferOutcome;
+
+    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
+    let _guard = ConfigDirGuard::new()?;
+    perf_history::set_perf_history_enabled(true)?;
+    let _ = perf_history::clear_history()?;
+
+    let tmp = tempdir()?;
+    let src = tmp.path().join("src");
+    let dest = tmp.path().join("dest");
+    fs::create_dir_all(&src)?;
+
+    let options = LocalMirrorOptions {
+        progress: false,
+        perf_history: true,
+        ..Default::default()
+    };
+
+    let orchestrator = TransferOrchestrator::new();
+    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
+    assert_eq!(summary.copied_files, 0);
+    assert_eq!(summary.outcome, TransferOutcome::SourceEmpty);
+
+    let records = perf_history::read_recent_records(0)?;
+    let last = records.last().expect("expected perf history record");
+    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
+    Ok(())
+}
+
+/// ue-r2-1c: the single-file shortcut historically bypassed
+/// perf-history recording entirely — the only strategy that did. It
+/// now records with the `single_file` tag and scanned-feature
+/// accounting (REV4 Design §2: strategies share common accounting).
+#[test]
+fn single_file_copy_records_history() -> Result<()> {
+    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
+    let _guard = ConfigDirGuard::new()?;
+    perf_history::set_perf_history_enabled(true)?;
+    let _ = perf_history::clear_history()?;
+
+    let tmp = tempdir()?;
+    let src = tmp.path().join("one.bin");
+    let dest = tmp.path().join("dest.bin");
+    fs::write(&src, b"payload-bytes")?;
+
+    let options = LocalMirrorOptions {
+        progress: false,
+        perf_history: true,
+        ..Default::default()
+    };
+
+    let orchestrator = TransferOrchestrator::new();
+    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
+    assert_eq!(summary.copied_files, 1);
+
+    let records = perf_history::read_recent_records(0)?;
+    let last = records.last().expect("expected perf history record");
+    assert_eq!(last.fast_path.as_deref(), Some("single_file"));
+    assert_eq!(last.file_count, 1);
+    assert_eq!(last.total_bytes, b"payload-bytes".len() as u64);
+    Ok(())
+}
+
 #[test]
 fn larger_manifest_records_streaming_path() -> Result<()> {
     let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
@@ -81,7 +197,7 @@ fn larger_manifest_records_streaming_path() -> Result<()> {
     fs::create_dir_all(&src)?;
     fs::create_dir_all(&dest)?;
     // Must exceed the fast-path tiny budget (TINY_FILE_LIMIT = 256
-    // in orchestrator/fast_path.rs) so the streaming planner runs.
+    // in engine/strategy.rs) so the streaming planner runs.
     // The original 32-file version predates that threshold.
     for idx in 0..300 {
         let file = src.join(format!("file-{idx}.txt"));

codex
There are already uncommitted review/state artifacts in the tree; I’m ignoring them for this range review and using explicit commit refs throughout. Next I’m drilling into line-numbered files so any finding can point at the landed code, not just the diff.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! The unified transfer engine (`ue-r2-1c`, REV4 Design §1).
     2	//!
     3	//! `TransferEngine` owns transfer execution: strategy selection
     4	//! (`journal_no_work`, `no_work`, `tiny_manifest`, `single_huge_file`,
     5	//! the single-file shortcut, streaming pipeline), the streaming leg
     6	//! (plan tuning -> scan -> diff/plan -> sink pipeline -> mirror
     7	//! deletions), and the perf-history/predictor accounting hooks. Path
     8	//! adapters construct the source, sink, and options, then call
     9	//! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
    10	//! adapter today, and push/pull converge here at `ue-r2-1f`/`1g`.
    11	//! Dial creation and streaming plans arrive with `ue-r2-1d`/`1e`
    12	//! (REV4 "Slice dependencies").
    13	//!
    14	//! The option/summary types keep their `LocalMirror*` names until the
    15	//! remote paths converge -- renaming ahead of those slices would churn
    16	//! every caller twice.
    17	
    18	mod history;
    19	mod journal;
    20	mod mirror;
    21	mod options;
    22	mod single_file;
    23	mod strategy;
    24	mod summary;
    25	mod tuning;
    26	
    27	pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
    28	pub use summary::{LocalMirrorSummary, TransferOutcome};
    29	
    30	use std::collections::HashSet;
    31	use std::path::PathBuf;
    32	use std::sync::{Arc, Mutex};
    33	use std::time::Instant;
    34	
    35	use eyre::{bail, Context, Result};
    36	
    37	use crate::auto_tune::derive_local_plan_tuning;
    38	use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken};
    39	use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
    40	use crate::perf_history::{read_recent_records, TransferMode};
    41	use crate::perf_predictor::PerformancePredictor;
    42	use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
    43	use crate::remote::transfer::payload::DEFAULT_PAYLOAD_PREFETCH;
    44	use crate::remote::transfer::pipeline::execute_sink_pipeline;
    45	use crate::remote::transfer::sink::TransferSink;
    46	use crate::remote::transfer::source::TransferSource;
    47	use crate::transfer_plan::PlanOptions;
    48	use crate::CopyConfig;
    49	
    50	use self::history::{record_performance_history, update_predictor};
    51	use self::journal::{log_probe, persist_journal_checkpoints};
    52	use self::mirror::apply_mirror_deletions;
    53	use self::single_file::execute_single_file_copy;
    54	use self::strategy::{maybe_select_fast_path, FastPathDecision};
    55	use self::tuning::select_tuning_window_from_history;
    56	
    57	/// Everything the engine needs to run one transfer. The adapter owns
    58	/// path-specific construction (REV4 Design §1): it resolves roots,
    59	/// builds the (already filter-wrapped) source and the sink, translates
    60	/// its option surface, and hands over execution.
    61	pub struct EngineRequest {
    62	    pub src_root: PathBuf,
    63	    pub dest_root: PathBuf,
    64	    /// Filter-wrapped source; used by the streaming strategy's scan.
    65	    pub source: Arc<dyn TransferSource>,
    66	    /// Destination sink for the streaming strategy (`FsTransferSink`
    67	    /// or `NullSink` locally). Fast-path strategies use their own
    68	    /// blocking executors, exactly as before the engine existed.
    69	    pub sink: Arc<dyn TransferSink>,
    70	    pub options: LocalMirrorOptions,
    71	}
    72	
    73	/// The unified transfer engine. Stateless today (all state is
    74	/// per-execute); the live dial (`ue-r2-1e`) is the first field that
    75	/// will change that.
    76	pub struct TransferEngine;
    77	
    78	impl TransferEngine {
    79	    pub fn new() -> Self {
    80	        Self
    81	    }
    82	
    83	    /// Execute one transfer: select a strategy (single-file, journal
    84	    /// no-work, fast path, or streaming pipeline) and run it to a
    85	    /// summary. Behavior moved verbatim from
    86	    /// `TransferOrchestrator::execute_local_mirror_async` at
    87	    /// ue-r2-1c; the caller-visible contract is unchanged.
    88	    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
    89	        let EngineRequest {
    90	            src_root,
    91	            dest_root,
    92	            source,
    93	            sink,
    94	            options,
    95	        } = request;
    96	        let src_root = src_root.as_path();
    97	        let dest_root = dest_root.as_path();
    98	
    99	        let start_time = Instant::now();
   100	
   101	        // Single-file source: bypass the enumerator/planner/pipeline machinery
   102	        // entirely and copy the file directly. The destination resolver in the
   103	        // CLI has already produced the exact target path (accounting for
   104	        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
   105	        // Without this short-circuit, the enumerator would skip the depth-0
   106	        // root entry and the fast-path would report NoWork — silent data loss.
   107	        if src_root.is_file() {
   108	            return execute_single_file_copy(src_root, dest_root, &options, start_time);
   109	        }
   110	
   111	        let mut journal_tracker = ChangeTracker::load().ok();
   112	        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
   113	        let mut journal_skip = false;
   114	
   115	        let mut predictor = PerformancePredictor::load().ok();
   116	
   117	        let copy_config = CopyConfig {
   118	            workers: options.workers.max(1),
   119	            preserve_times: options.preserve_times,
   120	            dry_run: options.dry_run,
   121	            checksum: if options.checksum {
   122	                Some(crate::checksum::ChecksumType::Blake3)
   123	            } else {
   124	                None
   125	            },
   126	            resume: options.resume,
   127	            null_sink: options.null_sink,
   128	        };
   129	
   130	        // Journal fast-path requires BOTH source and destination to exist and
   131	        // report "no changes". A missing destination obviously needs a full
   132	        // transfer — treating it as unchanged would silently skip the work.
   133	        if options.skip_unchanged
   134	            && !options.checksum
   135	            && !options.force_tar
   136	            && !options.null_sink
   137	            && dest_root.exists()
   138	        {
   139	            if let Some(tracker) = journal_tracker.as_ref() {
   140	                match tracker.probe(src_root) {
   141	                    Ok(src_probe) => {
   142	                        let dest_probe = tracker.probe(dest_root).ok();
   143	
   144	                        if src_probe.snapshot.is_some() {
   145	                            journal_tokens.push(src_probe.clone());
   146	                        }
   147	                        if let Some(ref probe) = dest_probe {
   148	                            if probe.snapshot.is_some() {
   149	                                journal_tokens.push(probe.clone());
   150	                            }
   151	                        }
   152	
   153	                        if options.verbose {
   154	                            log_probe("src", &src_probe);
   155	                            if let Some(probe) = dest_probe.as_ref() {
   156	                                log_probe("dest", probe);
   157	                            } else {
   158	                                eprintln!("Journal probe dest unsupported; cannot take fast-path");
   159	                            }
   160	                        }
   161	
   162	                        let src_no_change = matches!(src_probe.state, ChangeState::NoChanges);
   163	                        // If dest_probe is None (unsupported FS), we cannot
   164	                        // assert "no change" — fall through to full planner.
   165	                        let dest_no_change = dest_probe
   166	                            .as_ref()
   167	                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
   168	                            .unwrap_or(false);
   169	
   170	                        if src_no_change && dest_no_change {
   171	                            journal_skip = true;
   172	                        }
   173	                    }
   174	                    Err(err) => {
   175	                        if options.verbose {
   176	                            eprintln!("Filesystem journal probe failed: {err:?}");
   177	                        }
   178	                    }
   179	                }
   180	            }
   181	        }
   182	
   183	        if journal_skip {
   184	            if options.verbose {
   185	                eprintln!(
   186	                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
   187	                );
   188	            }
   189	            if let Some(tracker) = journal_tracker.as_mut() {
   190	                persist_journal_checkpoints(
   191	                    tracker,
   192	                    journal_tokens.as_mut_slice(),
   193	                    options.verbose,
   194	                );
   195	            }
   196	
   197	            // Journal said both sides match, so we never enumerated.
   198	            // scanned_{files,bytes} stay 0 — predictor sees this as
   199	            // "noop with no scan cost" which is what actually happened.
   200	            let summary = LocalMirrorSummary {
   201	                dry_run: options.dry_run,
   202	                duration: start_time.elapsed(),
   203	                outcome: TransferOutcome::JournalSkip,
   204	                ..Default::default()
   205	            };
   206	
   207	            if let Some(record) = record_performance_history(
   208	                &summary,
   209	                &options,
   210	                Some("journal_no_work"),
   211	                0,
   212	                summary.duration.as_millis(),
   213	            ) {
   214	                update_predictor(&mut predictor, &record, options.verbose);
   215	            }
   216	
   217	            return Ok(summary);
   218	        }
   219	
   220	        // Skip fast path when using null sink — it bypasses the sink abstraction.
   221	        let fast_path_outcome = if options.null_sink {
   222	            self::strategy::FastPathOutcome::streaming()
   223	        } else {
   224	            maybe_select_fast_path(src_root, dest_root, &options)?
   225	        };
   226	        if let Some(decision) = fast_path_outcome.decision {
   227	            // R47-F4: propagate the fast-path scan's suppressed
   228	            // errors into the per-branch summary. Each fast-path
   229	            // outcome below clones this into `unreadable_paths`
   230	            // so the CLI's source-delete step can detect a
   231	            // partial scan even on the Tiny/Huge/NoWork paths.
   232	            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
   233	            let summary = match decision {
   234	                FastPathDecision::NoWork { examined } => {
   235	                    let outcome = if examined == 0 {
   236	                        TransferOutcome::SourceEmpty
   237	                    } else {
   238	                        TransferOutcome::UpToDate
   239	                    };
   240	                    if options.verbose {
   241	                        match outcome {
   242	                            TransferOutcome::SourceEmpty => {
   243	                                eprintln!("Fast-path routing: source yielded no file entries")
   244	                            }
   245	                            _ => eprintln!(
   246	                                "Fast-path routing: {} files examined, all up to date",
   247	                                examined
   248	                            ),
   249	                        }
   250	                    }
   251	                    // NoWork ran a real fast-path scan but copied nothing.
   252	                    // scanned_files = examined captures the planner-side
   253	                    // workload; scanned_bytes is 0 because the fast-path
   254	                    // scanner only resolves names + identity, not sizes.
   255	                    let summary = LocalMirrorSummary {
   256	                        planned_files: examined,
   257	                        scanned_files: examined,
   258	                        dry_run: options.dry_run,
   259	                        duration: start_time.elapsed(),
   260	                        outcome,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '260,560p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   260	                        outcome,
   261	                        unreadable_paths: fast_path_unreadable.clone(),
   262	                        ..Default::default()
   263	                    };
   264	                    if let Some(record) = record_performance_history(
   265	                        &summary,
   266	                        &options,
   267	                        Some("no_work"),
   268	                        0,
   269	                        summary.duration.as_millis(),
   270	                    ) {
   271	                        update_predictor(&mut predictor, &record, options.verbose);
   272	                    }
   273	                    summary
   274	                }
   275	                FastPathDecision::Tiny { files } => {
   276	                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
   277	                    if options.verbose {
   278	                        eprintln!(
   279	                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
   280	                            files.len(),
   281	                            total_bytes
   282	                        );
   283	                    }
   284	                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
   285	                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
   286	                    // Tiny copies everything it scanned, so scanned ==
   287	                    // copied here. Setting both lets the predictor
   288	                    // train on the actual workload size for the
   289	                    // tiny_manifest fast-path key.
   290	                    let summary = LocalMirrorSummary {
   291	                        planned_files: files.len(),
   292	                        copied_files: files.len(),
   293	                        total_bytes,
   294	                        scanned_files: files.len(),
   295	                        scanned_bytes: total_bytes,
   296	                        dry_run: options.dry_run,
   297	                        duration: start_time.elapsed(),
   298	                        unreadable_paths: fast_path_unreadable.clone(),
   299	                        ..Default::default()
   300	                    };
   301	                    if let Some(record) = record_performance_history(
   302	                        &summary,
   303	                        &options,
   304	                        Some("tiny_manifest"),
   305	                        0,
   306	                        summary.duration.as_millis(),
   307	                    ) {
   308	                        update_predictor(&mut predictor, &record, options.verbose);
   309	                    }
   310	                    summary
   311	                }
   312	                FastPathDecision::Huge { file, size } => {
   313	                    if options.verbose {
   314	                        eprintln!(
   315	                            "Fast-path routing: huge file {} ({} bytes)",
   316	                            file.display(),
   317	                            size
   318	                        );
   319	                    }
   320	                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
   321	                    // Huge fast-path copies a single file: scan size
   322	                    // and copy size are identical (one file, `size`
   323	                    // bytes).
   324	                    let summary = LocalMirrorSummary {
   325	                        planned_files: 1,
   326	                        copied_files: 1,
   327	                        total_bytes: size,
   328	                        scanned_files: 1,
   329	                        scanned_bytes: size,
   330	                        dry_run: options.dry_run,
   331	                        duration: start_time.elapsed(),
   332	                        large_tasks: 1,
   333	                        large_bytes: size,
   334	                        unreadable_paths: fast_path_unreadable.clone(),
   335	                        ..Default::default()
   336	                    };
   337	                    if let Some(record) = record_performance_history(
   338	                        &summary,
   339	                        &options,
   340	                        Some("single_huge_file"),
   341	                        0,
   342	                        summary.duration.as_millis(),
   343	                    ) {
   344	                        update_predictor(&mut predictor, &record, options.verbose);
   345	                    }
   346	                    summary
   347	                }
   348	            };
   349	
   350	            if let Some(tracker) = journal_tracker.as_mut() {
   351	                persist_journal_checkpoints(
   352	                    tracker,
   353	                    journal_tokens.as_mut_slice(),
   354	                    options.verbose,
   355	                );
   356	            }
   357	
   358	            if options.verbose {
   359	                eprintln!(
   360	                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
   361	                    if options.mirror { "mirror" } else { "copy" },
   362	                    summary.copied_files,
   363	                    summary.total_bytes,
   364	                    summary.duration
   365	                );
   366	            }
   367	
   368	            return Ok(summary);
   369	        }
   370	
   371	        // --- Unified pipeline: same path as remote transfers ---
   372	        let mut plan_options = PlanOptions {
   373	            force_tar: options.force_tar,
   374	            ..PlanOptions::default()
   375	        };
   376	
   377	        if options.perf_history {
   378	            // R57-F1: read ALL history, not a pre-cap window. The
   379	            // R56-F2 fix correctly filtered run_kind before the
   380	            // 20-record cap inside `select_tuning_window`, but the
   381	            // caller was still pre-capping at 50 records from the
   382	            // JSONL — so 50 recent non-real records could still
   383	            // hide older real records one layer up. The file is
   384	            // already size-capped at ~1 MiB upstream
   385	            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
   386	            // all records is bounded; `read_recent_records(0)`
   387	            // means "all" per its limit semantics.
   388	            let target_mode = if options.mirror {
   389	                TransferMode::Mirror
   390	            } else {
   391	                TransferMode::Copy
   392	            };
   393	            // R59 finding #5: tuning window keys on full compare_mode,
   394	            // not just options.checksum. Translate via the same enum
   395	            // the history snapshot uses so the bucket lookup matches
   396	            // what the writer recorded.
   397	            let query_compare_mode = options
   398	                .compare_mode
   399	                .resolve_compare_snapshot(options.checksum);
   400	            if let Some(filtered) = select_tuning_window_from_history(
   401	                read_recent_records,
   402	                target_mode,
   403	                query_compare_mode,
   404	                options.skip_unchanged,
   405	            ) {
   406	                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
   407	                    plan_options.small_target = Some(tuning.small_target_bytes);
   408	                    plan_options.small_count_target = Some(tuning.small_count_target);
   409	                    plan_options.medium_target = Some(tuning.medium_target_bytes);
   410	                }
   411	            }
   412	        }
   413	
   414	        let planning_start = Instant::now();
   415	
   416	        let src_root_buf = src_root.to_path_buf();
   417	        let dest_root_buf = dest_root.to_path_buf();
   418	        let skip_unchanged = options.skip_unchanged;
   419	        let ignore_existing = options.ignore_existing;
   420	        // R58-F7: translate the orchestrator's `compare_mode` (set by
   421	        // the CLI from --size-only / --ignore-times / --force /
   422	        // --checksum / default) onto the unified ComparisonMode enum.
   423	        // Pre-fix this hardcoded a bool→Checksum-or-SizeMtime mapping
   424	        // and ignored the other flags entirely; remote pull already
   425	        // honored all five variants, so behavior diverged by direction.
   426	        //
   427	        // Backward-compat: the old `options.checksum` bool still
   428	        // wins if it's set without `compare_mode` being explicitly
   429	        // changed — preserves the existing `--checksum` behavior
   430	        // for any caller that hasn't migrated yet.
   431	        let compare_mode = options
   432	            .compare_mode
   433	            .resolve_comparison_mode(options.checksum);
   434	
   435	        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
   436	        //    the user filter applies through the universal pipeline chokepoint
   437	        //    (identical to push/pull/remote-remote behavior — full parity).
   438	        // ue-r2-1c: the adapter built the (filter-wrapped) source; the
   439	        // engine owns running the scan.
   440	        let unreadable = Arc::new(Mutex::new(Vec::new()));
   441	        let (mut header_rx, scan_handle) = source.scan(None, unreadable.clone());
   442	
   443	        // 2. Collect all headers
   444	        let mut all_headers = Vec::new();
   445	        while let Some(h) = header_rx.recv().await {
   446	            all_headers.push(h);
   447	        }
   448	        let _total_scanned = scan_handle
   449	            .await
   450	            .context("scan task panicked")?
   451	            .context("scan failed")?;
   452	
   453	        // 3. Diff + plan via the shared DiffPlanner stage. Combines
   454	        //    the comparison-filter and payload-planning steps that
   455	        //    were previously inline. Behavior preserved bit-for-bit
   456	        //    (size+mtime or Blake3 hash, then tar/large/raw planning).
   457	        let src = src_root_buf.clone();
   458	        let dst = dest_root_buf.clone();
   459	        let plan_opts = plan_options;
   460	        let headers = all_headers.clone();
   461	        let planned = tokio::task::spawn_blocking(move || {
   462	            plan_local_mirror(
   463	                headers,
   464	                LocalDiffInputs {
   465	                    src_root: &src,
   466	                    dst_root: &dst,
   467	                    compare_mode,
   468	                    ignore_existing,
   469	                    plan_options: plan_opts,
   470	                    skip_unchanged,
   471	                },
   472	            )
   473	        })
   474	        .await
   475	        .context("diff_planner task panicked")??;
   476	
   477	        // 5. Execute the unified pipeline against the adapter-built
   478	        // sink (FsTransferSink with the translated compare_mode, or
   479	        // NullSink -- see TransferOrchestrator).
   480	
   481	        // Boundary between planner and transfer phases. `planning_start`
   482	        // covers scan + diff + plan; everything after this `Instant`
   483	        // is the transfer pipeline. §2.8 phase 2 split: pre-fix the
   484	        // record's `planner_duration_ms` field was set to whole-run
   485	        // time, so the v1 predictor effectively trained on `planner =
   486	        // total` for both targets and couldn't distinguish them.
   487	        let plan_done = Instant::now();
   488	        let planner_duration_ms = plan_done.duration_since(planning_start).as_millis();
   489	
   490	        // §2.8 phase 2: query the predictor BEFORE running the
   491	        // pipeline. Surfaces in summary.predictor_estimate so
   492	        // `--verbose` and `blit profile --json` can compare
   493	        // predicted vs actual.
   494	        //
   495	        // R44-F1: query and observation must use the same feature
   496	        // vector. We query with `(scanned_files, scanned_bytes)`
   497	        // here; `record_performance_history` populates the matching
   498	        // `PerformanceRecord.{file_count,total_bytes}` from
   499	        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
   500	        // record was populated from `summary.copied_files`, so on
   501	        // any incremental run the predictor was queried with one
   502	        // workload size and trained against another.
   503	        //
   504	        // src_fs/dest_fs are left None for 0.1.0 — wiring
   505	        // `fs_capability` per-path probes into the predictor query
   506	        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
   507	        let scanned_files = all_headers.len();
   508	        let scanned_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
   509	        // R45 follow-up to R44-F1: never alias `total_bytes` to
   510	        // `scanned_bytes`. `summary.total_bytes` is the
   511	        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
   512	        // rustdoc); the predictor uses scan features only. Pre-fix
   513	        // this aliased the two so `summary.total_bytes` reported
   514	        // scanned bytes as bytes-written, overcounting throughput
   515	        // on incremental runs.
   516	        let predictor_estimate = predictor.as_ref().and_then(|p| {
   517	            let kind_total = crate::perf_predictor::DurationKind::Total;
   518	            let mode = if options.mirror {
   519	                crate::perf_history::TransferMode::Mirror
   520	            } else {
   521	                crate::perf_history::TransferMode::Copy
   522	            };
   523	            let total_pred = p.predict(
   524	                kind_total,
   525	                mode.clone(),
   526	                None,
   527	                None,
   528	                None,
   529	                options.skip_unchanged,
   530	                options.checksum,
   531	                scanned_files,
   532	                scanned_bytes,
   533	            )?;
   534	            // Pull planner + transfer separately too so the verbose
   535	            // line and the JSON profile can break down the estimate.
   536	            // All three predictor calls share the same
   537	            // (scanned_files, scanned_bytes) feature vector — both
   538	            // for consistency with the recording side, and so a
   539	            // future maintainer can't accidentally reintroduce a
   540	            // train/query mismatch by editing one branch and
   541	            // missing another.
   542	            let planner_pred = p
   543	                .predict(
   544	                    crate::perf_predictor::DurationKind::Planner,
   545	                    mode.clone(),
   546	                    None,
   547	                    None,
   548	                    None,
   549	                    options.skip_unchanged,
   550	                    options.checksum,
   551	                    scanned_files,
   552	                    scanned_bytes,
   553	                )
   554	                .map(|p| p.predicted_ms)
   555	                .unwrap_or(0.0);
   556	            let transfer_pred = p
   557	                .predict(
   558	                    crate::perf_predictor::DurationKind::Transfer,
   559	                    mode,
   560	                    None,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '560,820p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   560	                    None,
   561	                    None,
   562	                    None,
   563	                    options.skip_unchanged,
   564	                    options.checksum,
   565	                    scanned_files,
   566	                    scanned_bytes,
   567	                )
   568	                .map(|p| p.predicted_ms)
   569	                .unwrap_or(0.0);
   570	            Some(self::summary::PredictorEstimate {
   571	                planner_ms: planner_pred.max(0.0) as u128,
   572	                transfer_ms: transfer_pred.max(0.0) as u128,
   573	                total_ms: total_pred.predicted_ms.max(0.0) as u128,
   574	                observations: total_pred.observations,
   575	                fallback_depth: total_pred.fallback_depth,
   576	            })
   577	        });
   578	        if options.verbose {
   579	            if let Some(est) = predictor_estimate.as_ref() {
   580	                eprintln!(
   581	                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
   582	                     total ~{} ms (n={}, fallback_depth={})",
   583	                    est.planner_ms,
   584	                    est.transfer_ms,
   585	                    est.total_ms,
   586	                    est.observations,
   587	                    est.fallback_depth
   588	                );
   589	            } else {
   590	                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
   591	            }
   592	        }
   593	
   594	        let pipeline_outcome = execute_sink_pipeline(
   595	            source,
   596	            vec![sink],
   597	            planned.payloads,
   598	            DEFAULT_PAYLOAD_PREFETCH,
   599	            None,
   600	        )
   601	        .await
   602	        .context("transfer pipeline failed")?;
   603	        let transfer_duration_ms = plan_done.elapsed().as_millis();
   604	
   605	        // R47-F4: snapshot unreadable paths so the CLI's source-
   606	        // delete step (in `blit move`) can refuse to remove a
   607	        // source it couldn't fully scan. The R46-F2 gate inside
   608	        // the orchestrator only fires on `options.mirror`, but
   609	        // move uses mirror=false — without this surface, an
   610	        // unreadable source file would get skipped during the
   611	        // copy and then silently deleted from the source by the
   612	        // CLI's `remove_dir_all` step.
   613	        let unreadable_snapshot: Vec<String> = unreadable
   614	            .lock()
   615	            .map(|guard| guard.clone())
   616	            .unwrap_or_default();
   617	
   618	        let mut summary = LocalMirrorSummary {
   619	            planned_files: pipeline_outcome.files_written,
   620	            copied_files: pipeline_outcome.files_written,
   621	            // R45: bytes the pipeline actually wrote, not scanned
   622	            // bytes. Distinct on incremental runs.
   623	            total_bytes: pipeline_outcome.bytes_written,
   624	            scanned_files,
   625	            scanned_bytes,
   626	            dry_run: options.dry_run,
   627	            duration: start_time.elapsed(),
   628	            predictor_estimate: predictor_estimate.clone(),
   629	            unreadable_paths: unreadable_snapshot.clone(),
   630	            ..Default::default()
   631	        };
   632	
   633	        if options.mirror {
   634	            // R46-F2: refuse to mirror-delete when the source scan
   635	            // was incomplete. The `unreadable_snapshot` captured
   636	            // above (R47-F4) covers the per-file open path
   637	            // (PermissionDenied / NotFound on individual files) and
   638	            // the walkdir non-root error path (unreadable
   639	            // subdirectories). Either case means the header set
   640	            // we're about to use as the source-of-truth for "what
   641	            // the destination should contain" is missing entries,
   642	            // and a delete pass would silently remove matching
   643	            // destination subtrees.
   644	            if !unreadable_snapshot.is_empty() {
   645	                bail!(
   646	                    "refusing to mirror-delete from {}: source scan was \
   647	                     incomplete ({} unreadable entr{}); the first {} \
   648	                     reported: {}. Resolve the scan errors (typically \
   649	                     permissions) or run as a non-mirror copy.",
   650	                    dest_root.display(),
   651	                    unreadable_snapshot.len(),
   652	                    if unreadable_snapshot.len() == 1 {
   653	                        "y"
   654	                    } else {
   655	                        "ies"
   656	                    },
   657	                    unreadable_snapshot.len().min(5),
   658	                    unreadable_snapshot
   659	                        .iter()
   660	                        .take(5)
   661	                        .cloned()
   662	                        .collect::<Vec<_>>()
   663	                        .join("; "),
   664	                );
   665	            }
   666	
   667	            let source_paths: HashSet<String> = all_headers
   668	                .iter()
   669	                .map(|h| h.relative_path.clone())
   670	                .collect();
   671	            let deletions = apply_mirror_deletions(
   672	                &source_paths,
   673	                dest_root,
   674	                &options.filter,
   675	                options.delete_scope,
   676	                !options.dry_run,
   677	                options.verbose,
   678	            )?;
   679	            summary.deleted_files = deletions.0;
   680	            summary.deleted_dirs = deletions.1;
   681	        }
   682	
   683	        if let Some(tracker) = journal_tracker.as_mut() {
   684	            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
   685	        }
   686	
   687	        if options.verbose {
   688	            eprintln!(
   689	                "Planning enumerated {} file(s), {} bytes",
   690	                scanned_files, scanned_bytes
   691	            );
   692	            eprintln!(
   693	                "Completed local {}: {} file(s), {} bytes in {:.2?} (plan {} ms, xfer {} ms)",
   694	                if options.mirror { "mirror" } else { "copy" },
   695	                summary.copied_files,
   696	                summary.total_bytes,
   697	                summary.duration,
   698	                planner_duration_ms,
   699	                transfer_duration_ms,
   700	            );
   701	            // §2.8: side-by-side predicted-vs-actual so operators
   702	            // can audit the predictor against this run's actual
   703	            // numbers. The bare percentage error per phase is the
   704	            // most useful single number; we keep absolute ms in the
   705	            // line above for context.
   706	            if let Some(est) = summary.predictor_estimate.as_ref() {
   707	                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
   708	                    if actual_ms == 0 {
   709	                        "n/a".to_string()
   710	                    } else {
   711	                        let pred = predicted_ms as f64;
   712	                        let act = actual_ms as f64;
   713	                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
   714	                    }
   715	                };
   716	                eprintln!(
   717	                    "Predictor delta: planner {} ({} vs {} ms), \
   718	                     transfer {} ({} vs {} ms)",
   719	                    pct(est.planner_ms, planner_duration_ms),
   720	                    est.planner_ms,
   721	                    planner_duration_ms,
   722	                    pct(est.transfer_ms, transfer_duration_ms),
   723	                    est.transfer_ms,
   724	                    transfer_duration_ms,
   725	                );
   726	            }
   727	        }
   728	
   729	        let fast_path_label = if options.null_sink {
   730	            Some("null_sink")
   731	        } else {
   732	            None
   733	        };
   734	        if let Some(record) = record_performance_history(
   735	            &summary,
   736	            &options,
   737	            fast_path_label,
   738	            planner_duration_ms,
   739	            transfer_duration_ms,
   740	        ) {
   741	            // Don't update the predictor from null-sink runs — the zero
   742	            // write cost would teach it that transfers are faster than
   743	            // they really are.
   744	            if !options.null_sink {
   745	                update_predictor(&mut predictor, &record, options.verbose);
   746	            }
   747	        }
   748	
   749	        Ok(summary)
   750	    }
   751	}
   752	
   753	impl Default for TransferEngine {
   754	    fn default() -> Self {
   755	        Self::new()
   756	    }
   757	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/orchestrator/orchestrator.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::path::Path;
     2	use std::sync::Arc;
     3	
     4	use eyre::{eyre, Context, Result};
     5	use tokio::runtime::Builder;
     6	
     7	use crate::engine::{EngineRequest, TransferEngine};
     8	use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, NullSink, TransferSink};
     9	use crate::remote::transfer::source::{FilteredSource, FsTransferSource, TransferSource};
    10	
    11	use super::{LocalMirrorOptions, LocalMirrorSummary};
    12	
    13	/// The LOCAL adapter for [`TransferEngine`] (ue-r2-1c, REV4 Design §1).
    14	///
    15	/// Owns exactly the path-specific boundary work: precondition checks,
    16	/// construction of the local filesystem source (filter-wrapped) and
    17	/// sink, and option translation. Everything else -- strategy selection
    18	/// (journal / fast paths / single-file / streaming), execution, and
    19	/// accounting -- lives in the engine. The public API is unchanged from
    20	/// the pre-engine orchestrator.
    21	pub struct TransferOrchestrator;
    22	
    23	impl TransferOrchestrator {
    24	    pub fn new() -> Self {
    25	        Self
    26	    }
    27	
    28	    /// Sync wrapper around [`execute_local_mirror_async`]. Builds a
    29	    /// new multi-thread Tokio runtime and blocks on it. Use this from
    30	    /// non-async callers (CLI commands, tests). Callers already
    31	    /// inside an async runtime must use `execute_local_mirror_async`
    32	    /// directly -- calling this from inside a Tokio context will
    33	    /// panic at `Runtime::new` (closes F9 of
    34	    /// `docs/reviews/codebase_review_2026-05-01.md`).
    35	    ///
    36	    /// [`execute_local_mirror_async`]: Self::execute_local_mirror_async
    37	    pub fn execute_local_mirror(
    38	        &self,
    39	        src_root: &Path,
    40	        dest_root: &Path,
    41	        options: LocalMirrorOptions,
    42	    ) -> Result<LocalMirrorSummary> {
    43	        let workers = options.workers.max(1);
    44	        let runtime = Builder::new_multi_thread()
    45	            .worker_threads(workers)
    46	            .enable_all()
    47	            .build()
    48	            .context("build tokio runtime")?;
    49	        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
    50	    }
    51	
    52	    /// Async local-transfer entry point: validate the local
    53	    /// preconditions, construct the local source/sink pair, and hand
    54	    /// execution to the engine.
    55	    pub async fn execute_local_mirror_async(
    56	        &self,
    57	        src_root: &Path,
    58	        dest_root: &Path,
    59	        options: LocalMirrorOptions,
    60	    ) -> Result<LocalMirrorSummary> {
    61	        if !src_root.exists() {
    62	            return Err(eyre!("source path does not exist: {}", src_root.display()));
    63	        }
    64	
    65	        if !options.dry_run {
    66	            if let Some(parent) = dest_root.parent() {
    67	                std::fs::create_dir_all(parent).with_context(|| {
    68	                    format!("failed to create destination parent {}", parent.display())
    69	                })?;
    70	            }
    71	        }
    72	
    73	        // Local source, wrapped in FilteredSource so the user filter
    74	        // applies through the universal pipeline chokepoint (identical
    75	        // to push/pull/remote-remote behavior -- full parity).
    76	        let inner: Arc<dyn TransferSource> =
    77	            Arc::new(FsTransferSource::new(src_root.to_path_buf()));
    78	        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(
    79	            inner,
    80	            options.filter.clone_without_cache(),
    81	        ));
    82	
    83	        // Local sink. Construction is pure state (paths + config), so
    84	        // building it up front -- even for runs the engine resolves via
    85	        // a fast path that never touches it -- is behavior-neutral.
    86	        let compare_mode = options
    87	            .compare_mode
    88	            .resolve_comparison_mode(options.checksum);
    89	        let sink: Arc<dyn TransferSink> = if options.null_sink {
    90	            Arc::new(NullSink::new())
    91	        } else {
    92	            Arc::new(FsTransferSink::new(
    93	                src_root.to_path_buf(),
    94	                dest_root.to_path_buf(),
    95	                FsSinkConfig {
    96	                    preserve_times: options.preserve_times,
    97	                    dry_run: options.dry_run,
    98	                    checksum: if options.checksum {
    99	                        Some(crate::checksum::ChecksumType::Blake3)
   100	                    } else {
   101	                        None
   102	                    },
   103	                    resume: options.resume,
   104	                    // R58-followup: thread the compare_mode into the
   105	                    // sink. Pre-fix the sink hard-coded SizeMtime via
   106	                    // file_needs_copy_with_checksum_type, defeating
   107	                    // --force / --ignore-times: the planner emitted
   108	                    // the file but the sink decided "skip" when
   109	                    // mtime+size matched.
   110	                    compare_mode,
   111	                },
   112	            ))
   113	        };
   114	
   115	        TransferEngine::new()
   116	            .execute(EngineRequest {
   117	                src_root: src_root.to_path_buf(),
   118	                dest_root: dest_root.to_path_buf(),
   119	                source,
   120	                sink,
   121	                options,
   122	            })
   123	            .await
   124	    }
   125	}
   126	
   127	impl Default for TransferOrchestrator {
   128	    fn default() -> Self {
   129	        Self::new()
   130	    }
   131	}
   132	
   133	#[cfg(test)]
   134	mod async_runtime_tests {
   135	    //! F9 regression: `execute_local_mirror_async` must be callable
   136	    //! from inside an existing Tokio runtime without panicking. The
   137	    //! sync `execute_local_mirror` wrapper builds its own runtime
   138	    //! and would panic with "Cannot start a runtime from within a
   139	    //! runtime" if called from `#[tokio::test]`.
   140	    use super::*;
   141	    use tempfile::tempdir;
   142	
   143	    fn write_file(path: &std::path::Path, body: &[u8]) {
   144	        if let Some(parent) = path.parent() {
   145	            std::fs::create_dir_all(parent).unwrap();
   146	        }
   147	        std::fs::write(path, body).unwrap();
   148	    }
   149	
   150	    fn opts() -> LocalMirrorOptions {
   151	        LocalMirrorOptions {
   152	            workers: 2,
   153	            preserve_times: false,
   154	            dry_run: false,
   155	            checksum: false,
   156	            ..Default::default()
   157	        }
   158	    }
   159	
   160	    #[tokio::test]
   161	    async fn async_version_callable_from_async_context() {
   162	        // The whole point of F9 — calling the async version from
   163	        // within #[tokio::test]'s runtime must not build a nested
   164	        // runtime or panic.
   165	        let tmp = tempdir().unwrap();
   166	        let src = tmp.path().join("src");
   167	        let dst = tmp.path().join("dst");
   168	        write_file(&src.join("a.txt"), b"hello");
   169	        let orch = TransferOrchestrator::new();
   170	        let summary = orch
   171	            .execute_local_mirror_async(&src, &dst, opts())
   172	            .await
   173	            .unwrap();
   174	        assert!(
   175	            summary.copied_files >= 1,
   176	            "expected at least one file copied, got {:?}",
   177	            summary
   178	        );
   179	        assert!(dst.join("a.txt").exists());
   180	    }
   181	
   182	    #[test]
   183	    fn sync_wrapper_still_works() {
   184	        // The sync API must keep working for non-async callers
   185	        // (CLI commands today).
   186	        let tmp = tempdir().unwrap();
   187	        let src = tmp.path().join("src");
   188	        let dst = tmp.path().join("dst");
   189	        write_file(&src.join("a.txt"), b"hello-sync");
   190	        let orch = TransferOrchestrator::new();
   191	        let summary = orch.execute_local_mirror(&src, &dst, opts()).unwrap();
   192	        assert!(summary.copied_files >= 1);
   193	        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello-sync");
   194	    }
   195	
   196	    /// R45 regression: `summary.total_bytes` must report bytes the
   197	    /// pipeline actually wrote, not bytes the source scan saw. The
   198	    /// pre-fix R44 commit aliased `let total_bytes = scanned_bytes`
   199	    /// and fed that into the summary — so on this skip-unchanged
   200	    /// incremental run the second run would have reported the full
   201	    /// scanned size as bytes-written even though zero bytes were
   202	    /// actually written.
   203	    ///
   204	    /// The fast-path branches (NoWork / Tiny / Huge / JournalSkip)
   205	    /// don't exhibit the bug because they construct their summary
   206	    /// directly without going through the aliased local. We force
   207	    /// the streaming-pipeline path by enabling `mirror = true`,
   208	    /// which disables fast-path selection (see
   209	    /// `maybe_select_fast_path`'s mirror short-circuit).
   210	    #[tokio::test]
   211	    async fn incremental_run_total_bytes_excludes_skipped_files() {
   212	        let tmp = tempdir().unwrap();
   213	        let src = tmp.path().join("src");
   214	        let dst = tmp.path().join("dst");
   215	        let body_a = vec![b'a'; 2 * 1024];
   216	        let body_b = vec![b'b'; 2 * 1024];
   217	        write_file(&src.join("a.txt"), &body_a);
   218	        write_file(&src.join("b.txt"), &body_b);
   219	        let total_payload = (body_a.len() + body_b.len()) as u64;
   220	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/single_file.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Single-file copy strategy. Moved from
     2	//! `orchestrator/orchestrator.rs` at ue-r2-1c; the same slice adds
     3	//! the perf-history/predictor accounting this path lacked
     4	//! (REV4 Design §2: engine strategies share common accounting).
     5	
     6	use std::path::{Path, PathBuf};
     7	use std::time::Instant;
     8	
     9	use eyre::{Context, Result};
    10	
    11	use crate::generated::ComparisonMode;
    12	use crate::perf_predictor::PerformancePredictor;
    13	
    14	use super::history::{record_performance_history, update_predictor};
    15	use super::options::LocalMirrorOptions;
    16	use super::summary::{LocalMirrorSummary, TransferOutcome};
    17	
    18	/// Copy a single file source directly to `dest_root` (the CLI's
    19	/// destination resolver has already produced the exact target path),
    20	/// then account for the run. ue-r2-1c: before the engine existed this
    21	/// shortcut bypassed perf-history/predictor recording entirely — the
    22	/// only strategy that did. It now records like every other strategy:
    23	/// tag `single_file` (or `null_sink`, matching the streaming path's
    24	/// lane convention so RunKind::NullSink derivation keeps working), no
    25	/// predictor update on null-sink runs (zero write cost would teach the
    26	/// predictor that transfers are faster than they really are). Records
    27	/// carry `tar_shard_tasks == raw_bundle_tasks == 0`, so the tuning
    28	/// window's signal filter already excludes them from auto-tuning.
    29	pub(super) fn execute_single_file_copy(
    30	    src_root: &Path,
    31	    dest_root: &Path,
    32	    options: &LocalMirrorOptions,
    33	    start_time: Instant,
    34	) -> Result<LocalMirrorSummary> {
    35	    let summary = single_file_copy_inner(src_root, dest_root, options, start_time)?;
    36	
    37	    let fast_path_label = if options.null_sink {
    38	        "null_sink"
    39	    } else {
    40	        "single_file"
    41	    };
    42	    if let Some(record) = record_performance_history(
    43	        &summary,
    44	        options,
    45	        Some(fast_path_label),
    46	        0,
    47	        summary.duration.as_millis(),
    48	    ) {
    49	        if !options.null_sink {
    50	            let mut predictor = PerformancePredictor::load().ok();
    51	            update_predictor(&mut predictor, &record, options.verbose);
    52	        }
    53	    }
    54	
    55	    Ok(summary)
    56	}
    57	
    58	/// The copy itself, bypassing the enumerator/planner/pipeline
    59	/// machinery which assumes `src_root` is a directory.
    60	fn single_file_copy_inner(
    61	    src_root: &Path,
    62	    dest_root: &Path,
    63	    options: &LocalMirrorOptions,
    64	    start_time: Instant,
    65	) -> Result<LocalMirrorSummary> {
    66	    use crate::buffer::BufferSizer;
    67	    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
    68	    use crate::logger::NoopLogger;
    69	    use filetime::FileTime;
    70	
    71	    let src_meta = std::fs::metadata(src_root)
    72	        .with_context(|| format!("stat source file {}", src_root.display()))?;
    73	    let size = src_meta.len();
    74	
    75	    // R58-followup: route compare-mode for the single-file path
    76	    // through the same translation the directory path uses
    77	    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
    78	    // at `options.checksum`, so `--size-only` / `--ignore-times` /
    79	    // `--force` were silently dropped — repro: copy src.txt dst.txt
    80	    // --size-only re-copied even when sizes matched.
    81	    let compare_mode: ComparisonMode = options
    82	        .compare_mode
    83	        .resolve_comparison_mode(options.checksum);
    84	
    85	    // R58-F5: the single-file strategy (engine dispatch)
    86	    // bypasses the enumerator + planner, which is where the
    87	    // streaming-pipeline path checks filter / ignore_existing.
    88	    // Apply both here so single-file copies honor the same
    89	    // CLI contract.
    90	    //
    91	    // Filter: the source root is itself the only entry. Run
    92	    // `filter.allows_entry` against the source name. If excluded,
    93	    // return a "scanned 1 / copied 0" summary so the user sees
    94	    // "no work performed" rather than the file being copied
    95	    // anyway.
    96	    let src_name = src_root.file_name().map(PathBuf::from);
    97	    let allows = match src_name {
    98	        Some(name) => {
    99	            let mtime = src_meta.modified().ok();
   100	            options
   101	                .filter
   102	                .allows_entry(Some(&name), src_root, size, mtime)
   103	        }
   104	        None => true,
   105	    };
   106	    if !allows {
   107	        return Ok(LocalMirrorSummary {
   108	            planned_files: 0,
   109	            copied_files: 0,
   110	            total_bytes: 0,
   111	            scanned_files: 1,
   112	            scanned_bytes: size,
   113	            duration: start_time.elapsed(),
   114	            outcome: TransferOutcome::UpToDate,
   115	            ..Default::default()
   116	        });
   117	    }
   118	
   119	    // ignore_existing: if the destination file already exists,
   120	    // skip the copy entirely. Matches the diff_planner behavior
   121	    // for the streaming-pipeline path (diff_planner.rs).
   122	    if options.ignore_existing && dest_root.exists() {
   123	        return Ok(LocalMirrorSummary {
   124	            planned_files: 0,
   125	            copied_files: 0,
   126	            total_bytes: 0,
   127	            scanned_files: 1,
   128	            scanned_bytes: size,
   129	            duration: start_time.elapsed(),
   130	            outcome: TransferOutcome::UpToDate,
   131	            ..Default::default()
   132	        });
   133	    }
   134	
   135	    if options.dry_run {
   136	        return Ok(LocalMirrorSummary {
   137	            planned_files: 1,
   138	            copied_files: 1,
   139	            total_bytes: size,
   140	            scanned_files: 1,
   141	            scanned_bytes: size,
   142	            dry_run: true,
   143	            duration: start_time.elapsed(),
   144	            ..Default::default()
   145	        });
   146	    }
   147	
   148	    if options.null_sink {
   149	        return Ok(LocalMirrorSummary {
   150	            planned_files: 1,
   151	            copied_files: 1,
   152	            total_bytes: size,
   153	            scanned_files: 1,
   154	            scanned_bytes: size,
   155	            duration: start_time.elapsed(),
   156	            ..Default::default()
   157	        });
   158	    }
   159	
   160	    let mut did_copy = false;
   161	    let mut clone_succeeded = false;
   162	    let mut bytes_copied = 0u64;
   163	
   164	    if options.resume {
   165	        let outcome = resume_copy_file(src_root, dest_root, 0)
   166	            .with_context(|| format!("resume copy {}", src_root.display()))?;
   167	        did_copy = outcome.bytes_transferred > 0;
   168	        bytes_copied = outcome.bytes_transferred;
   169	    } else {
   170	        let needs_copy = !options.skip_unchanged
   171	            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
   172	        if needs_copy {
   173	            let sizer = BufferSizer::default();
   174	            let logger = NoopLogger;
   175	            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
   176	                .with_context(|| format!("copy {}", src_root.display()))?;
   177	            did_copy = true;
   178	            clone_succeeded = outcome.clone_succeeded;
   179	            bytes_copied = outcome.bytes_copied;
   180	        }
   181	    }
   182	
   183	    if options.preserve_times && did_copy && !clone_succeeded {
   184	        if let Ok(modified) = src_meta.modified() {
   185	            let ft = FileTime::from_system_time(modified);
   186	            // R42-F1: warn-don't-silence (was `let _ = ...`).
   187	            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
   188	                log::warn!("set mtime on {}: {}", dest_root.display(), e);
   189	            }
   190	        }
   191	    }
   192	
   193	    Ok(LocalMirrorSummary {
   194	        planned_files: 1,
   195	        copied_files: if did_copy { 1 } else { 0 },
   196	        total_bytes: bytes_copied,
   197	        // Single-file path always saw exactly one entry of `size`
   198	        // bytes; whether we copied it or not is the
   199	        // copied_files/total_bytes story, but the scan saw it.
   200	        scanned_files: 1,
   201	        scanned_bytes: size,
   202	        duration: start_time.elapsed(),
   203	        outcome: if did_copy {
   204	            TransferOutcome::Transferred
   205	        } else {
   206	            TransferOutcome::UpToDate
   207	        },
   208	        ..Default::default()
   209	    })
   210	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/history.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::perf_history::{
     2	    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
     3	};
     4	use crate::perf_predictor::PerformancePredictor;
     5	
     6	use super::{LocalMirrorOptions, LocalMirrorSummary};
     7	
     8	/// Map the orchestrator's `LocalCompareMode` onto the perf-history
     9	/// snapshot enum so tuning records preserve the user's full intent
    10	/// (not just `checksum: bool`).
    11	fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
    12	    options
    13	        .compare_mode
    14	        .resolve_compare_snapshot(options.checksum)
    15	}
    16	
    17	pub(super) fn record_performance_history(
    18	    summary: &LocalMirrorSummary,
    19	    options: &LocalMirrorOptions,
    20	    fast_path: Option<&str>,
    21	    planner_duration_ms: u128,
    22	    transfer_duration_ms: u128,
    23	) -> Option<PerformanceRecord> {
    24	    if !options.perf_history {
    25	        return None;
    26	    }
    27	
    28	    let record = build_performance_record(
    29	        summary,
    30	        options,
    31	        fast_path,
    32	        planner_duration_ms,
    33	        transfer_duration_ms,
    34	    );
    35	
    36	    if let Err(err) = append_local_record(&record) {
    37	        if options.verbose {
    38	            eprintln!("Failed to update performance history: {err:?}");
    39	        }
    40	    }
    41	    Some(record)
    42	}
    43	
    44	/// Construct the `PerformanceRecord` from a summary without
    45	/// touching disk. Split out from `record_performance_history` so
    46	/// the record-shape contract — specifically R44-F1's "train and
    47	/// query against the same feature vector" invariant — is
    48	/// unit-testable without writing to the global perf history file.
    49	fn build_performance_record(
    50	    summary: &LocalMirrorSummary,
    51	    options: &LocalMirrorOptions,
    52	    fast_path: Option<&str>,
    53	    planner_duration_ms: u128,
    54	    transfer_duration_ms: u128,
    55	) -> PerformanceRecord {
    56	    let options_snapshot = OptionSnapshot {
    57	        dry_run: options.dry_run,
    58	        preserve_symlinks: options.preserve_symlinks,
    59	        include_symlinks: options.include_symlinks,
    60	        skip_unchanged: options.skip_unchanged,
    61	        checksum: options.checksum,
    62	        compare_mode: snapshot_compare_mode(options),
    63	        workers: options.workers,
    64	    };
    65	
    66	    let mode = if options.mirror {
    67	        TransferMode::Mirror
    68	    } else {
    69	        TransferMode::Copy
    70	    };
    71	
    72	    // R44-F1: train against scanned features so the predictor's
    73	    // training inputs match its query inputs. The orchestrator
    74	    // queries `predict(...)` with `all_headers.len()` (scanned
    75	    // count) and `total_bytes` (scanned bytes); pre-fix the record
    76	    // was populated with `summary.copied_files`, so the predictor
    77	    // saw a different feature vector at training time than at
    78	    // query time, and predictions drifted on every incremental
    79	    // workload. The `total_bytes` field on the record was already
    80	    // scanned-bytes by accident; this aligns both axes deliberately.
    81	    //
    82	    // `summary.copied_files` and the per-bucket counts
    83	    // (tar_shard_files / raw_bundle_files / large_tasks) still
    84	    // reflect actual writes — they're the load-bearing inputs for
    85	    // `derive_local_plan_tuning`'s bucket-target heuristics, which
    86	    // are computed from observed apply behavior, not scan size.
    87	    let mut record = PerformanceRecord::new(
    88	        mode,
    89	        None,
    90	        None,
    91	        summary.scanned_files,
    92	        summary.scanned_bytes,
    93	        options_snapshot,
    94	        fast_path.map(|s| s.to_string()),
    95	        planner_duration_ms,
    96	        transfer_duration_ms,
    97	        0,
    98	        0,
    99	    );
   100	    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
   101	    record.tar_shard_files = summary.tar_shard_files as u32;
   102	    record.tar_shard_bytes = summary.tar_shard_bytes;
   103	    record.raw_bundle_tasks = summary.raw_bundle_tasks as u32;
   104	    record.raw_bundle_files = summary.raw_bundle_files as u32;
   105	    record.raw_bundle_bytes = summary.raw_bundle_bytes;
   106	    record.large_tasks = summary.large_tasks as u32;
   107	    record.large_bytes = summary.large_bytes;
   108	
   109	    record
   110	}
   111	
   112	pub(super) fn update_predictor(
   113	    predictor: &mut Option<PerformancePredictor>,
   114	    record: &PerformanceRecord,
   115	    verbose: bool,
   116	) {
   117	    if let Some(ref mut predictor) = predictor {
   118	        predictor.observe(record);
   119	        if let Err(err) = predictor.save() {
   120	            if verbose {
   121	                eprintln!("Failed to persist predictor state: {err:?}");
   122	            }
   123	        }
   124	    }
   125	}
   126	
   127	#[cfg(test)]
   128	mod tests {
   129	    use super::*;
   130	    use crate::orchestrator::TransferOutcome;
   131	    use std::time::Duration;
   132	
   133	    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
   134	        LocalMirrorOptions {
   135	            mirror,
   136	            ..LocalMirrorOptions::default()
   137	        }
   138	    }
   139	
   140	    /// R44-F1 contract: the record's `(file_count, total_bytes)`
   141	    /// must mirror the orchestrator's predictor-query features.
   142	    /// Pre-fix this assertion would have failed: the record was
   143	    /// populated from `summary.copied_files` and `summary.total_bytes`
   144	    /// while the query used scanned values, so on this incremental
   145	    /// scenario (1000 scanned, 5 actually written) the predictor
   146	    /// trained on (5, 100KB) but was queried with
   147	    /// (1000, ~10MB).
   148	    #[test]
   149	    fn record_uses_scanned_features_not_copied() {
   150	        let summary = LocalMirrorSummary {
   151	            // Mostly-unchanged incremental run: 1000 files scanned,
   152	            // only 5 actually written.
   153	            scanned_files: 1000,
   154	            scanned_bytes: 10 * 1024 * 1024,
   155	            planned_files: 5,
   156	            copied_files: 5,
   157	            total_bytes: 100 * 1024,
   158	            duration: Duration::from_millis(200),
   159	            outcome: TransferOutcome::Transferred,
   160	            ..LocalMirrorSummary::default()
   161	        };
   162	        let options = options_with_mirror(false);
   163	        let record = build_performance_record(&summary, &options, Some("streaming"), 150, 50);
   164	
   165	        assert_eq!(
   166	            record.file_count, 1000,
   167	            "record.file_count must reflect scanned (planner-side) workload, not copied count"
   168	        );
   169	        assert_eq!(
   170	            record.total_bytes, summary.scanned_bytes,
   171	            "record.total_bytes must reflect scanned bytes, not transferred bytes"
   172	        );
   173	        assert_eq!(record.planner_duration_ms, 150);
   174	        assert_eq!(record.transfer_duration_ms, 50);
   175	    }
   176	
   177	    /// Bucket-shape fields (tar_shard_*, raw_bundle_*, large_*)
   178	    /// must continue to reflect actual write activity — they feed
   179	    /// `derive_local_plan_tuning` which heuristically sizes
   180	    /// destination buckets from past apply behavior.
   181	    #[test]
   182	    fn bucket_counts_still_reflect_actual_writes() {
   183	        let summary = LocalMirrorSummary {
   184	            scanned_files: 100,
   185	            scanned_bytes: 1_000_000,
   186	            copied_files: 10,
   187	            total_bytes: 50_000,
   188	            tar_shard_tasks: 2,
   189	            tar_shard_files: 7,
   190	            tar_shard_bytes: 30_000,
   191	            raw_bundle_tasks: 1,
   192	            raw_bundle_files: 2,
   193	            raw_bundle_bytes: 15_000,
   194	            large_tasks: 1,
   195	            large_bytes: 5_000,
   196	            ..LocalMirrorSummary::default()
   197	        };
   198	        let options = options_with_mirror(true);
   199	        let record = build_performance_record(&summary, &options, Some("streaming"), 30, 70);
   200	
   201	        assert_eq!(record.tar_shard_tasks, 2);
   202	        assert_eq!(record.tar_shard_files, 7);
   203	        assert_eq!(record.tar_shard_bytes, 30_000);
   204	        assert_eq!(record.raw_bundle_tasks, 1);
   205	        assert_eq!(record.raw_bundle_files, 2);
   206	        assert_eq!(record.raw_bundle_bytes, 15_000);
   207	        assert_eq!(record.large_tasks, 1);
   208	        assert_eq!(record.large_bytes, 5_000);
   209	    }
   210	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/tuning.rs | sed -n '1,240p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/options.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::fs_enum::FileFilter;
     2	
     3	/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
     4	/// (FilteredSubset / All) plus a `false`/`true` flag form. R58-F6:
     5	/// pre-fix, local mirror had no plumbing for this — `apply_mirror_deletions`
     6	/// always operated on whatever the transfer filter let through. The
     7	/// remote pull path already supports both modes via
     8	/// `PullSyncOptions.delete_all_scope`; this brings local up to parity.
     9	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    10	pub enum LocalMirrorDeleteScope {
    11	    /// Default: only delete destination entries that the source-side
    12	    /// filter would have allowed. Files matching `--exclude` patterns
    13	    /// at the destination are left alone, because they're not in
    14	    /// scope for this mirror operation.
    15	    #[default]
    16	    FilteredSubset,
    17	    /// Delete every destination entry not present at the source,
    18	    /// regardless of filter scope. Selected via `--delete-scope all`.
    19	    All,
    20	}
    21	
    22	/// Local comparison policy. Mirrors the wire-side `ComparisonMode` enum
    23	/// for the pull / remote-remote-direct paths so local copy/mirror
    24	/// behaves the same as a same-options remote run.
    25	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    26	pub enum LocalCompareMode {
    27	    /// Default size + mtime. Skip if both match.
    28	    #[default]
    29	    SizeMtime,
    30	    /// Compare by Blake3 checksum. Slow but content-accurate.
    31	    Checksum,
    32	    /// Compare by size only. Mtime differences are ignored.
    33	    SizeOnly,
    34	    /// Transfer regardless of target state.
    35	    Force,
    36	    /// Transfer all files unconditionally (--ignore-times). Same
    37	    /// outcome as Force at the planner level; kept as a separate
    38	    /// variant so the user's intent is preserved in summaries.
    39	    IgnoreTimes,
    40	}
    41	
    42	impl LocalCompareMode {
    43	    /// Resolve onto the unified wire-side `ComparisonMode`, honoring
    44	    /// the legacy `checksum: bool` under the default `SizeMtime`
    45	    /// (back-compat: `--checksum` callers that haven't migrated to
    46	    /// `compare_mode` keep their behavior). ue-r2-1c: single home for
    47	    /// a translation that was previously copy-pasted at three sites
    48	    /// (streaming, tuning query, single-file), which had already
    49	    /// diverged once (R58-F7/R58-followup).
    50	    pub fn resolve_comparison_mode(
    51	        self,
    52	        legacy_checksum: bool,
    53	    ) -> crate::generated::ComparisonMode {
    54	        use crate::generated::ComparisonMode;
    55	        match self {
    56	            LocalCompareMode::Checksum => ComparisonMode::Checksum,
    57	            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
    58	            LocalCompareMode::Force => ComparisonMode::Force,
    59	            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
    60	            LocalCompareMode::SizeMtime => {
    61	                if legacy_checksum {
    62	                    ComparisonMode::Checksum
    63	                } else {
    64	                    ComparisonMode::SizeMtime
    65	                }
    66	            }
    67	        }
    68	    }
    69	
    70	    /// Same resolution, onto the perf-history snapshot enum (tuning
    71	    /// buckets key on the full comparison policy -- R59 finding #5).
    72	    pub(crate) fn resolve_compare_snapshot(
    73	        self,
    74	        legacy_checksum: bool,
    75	    ) -> crate::perf_history::CompareModeSnapshot {
    76	        use crate::perf_history::CompareModeSnapshot;
    77	        match self {
    78	            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
    79	            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
    80	            LocalCompareMode::Force => CompareModeSnapshot::Force,
    81	            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
    82	            LocalCompareMode::SizeMtime => {
    83	                if legacy_checksum {
    84	                    CompareModeSnapshot::Checksum
    85	                } else {
    86	                    CompareModeSnapshot::SizeMtime
    87	                }
    88	            }
    89	        }
    90	    }
    91	}
    92	
    93	/// Options for executing a local mirror/copy operation.
    94	#[derive(Clone, Debug)]
    95	pub struct LocalMirrorOptions {
    96	    pub filter: FileFilter,
    97	    pub mirror: bool,
    98	    pub dry_run: bool,
    99	    pub progress: bool,
   100	    pub verbose: bool,
   101	    pub perf_history: bool,
   102	    pub force_tar: bool,
   103	    pub preserve_symlinks: bool,
   104	    pub include_symlinks: bool,
   105	    pub skip_unchanged: bool,
   106	    /// Skip any file the destination already has, regardless of
   107	    /// comparison mode. Orthogonal to `checksum`/`skip_unchanged`;
   108	    /// matches the `ignore_existing` field on `TransferOperationSpec`
   109	    /// for full pipeline parity across local/push/pull paths.
   110	    pub ignore_existing: bool,
   111	    pub checksum: bool,
   112	    /// R58-F7: comparison policy. The orchestrator picks
   113	    /// `compare_mode` based on this rather than just the `checksum`
   114	    /// bool, so `--size-only` / `--ignore-times` / `--force` get
   115	    /// honored on local copy/mirror the same way the pull path
   116	    /// honors them.
   117	    pub compare_mode: LocalCompareMode,
   118	    /// R58-F6: delete-scope policy for mirror. Only consulted when
   119	    /// `mirror == true`. Defaults to FilteredSubset so a
   120	    /// `mirror --exclude '*.log'` doesn't delete the destination's
   121	    /// `*.log` files just because they were out of scope for the
   122	    /// source filter.
   123	    pub delete_scope: LocalMirrorDeleteScope,
   124	    pub workers: usize,
   125	    pub preserve_times: bool,
   126	    pub debug_mode: bool,
   127	    /// Resume interrupted transfers using block-level comparison.
   128	    pub resume: bool,
   129	    /// Discard writes (NullSink). Measures source read + pipeline throughput.
   130	    pub null_sink: bool,
   131	}
   132	
   133	impl Default for LocalMirrorOptions {
   134	    fn default() -> Self {
   135	        Self {
   136	            filter: FileFilter::default(),
   137	            mirror: false,
   138	            dry_run: false,
   139	            progress: false,
   140	            verbose: false,
   141	            perf_history: true,
   142	            force_tar: false,
   143	            preserve_symlinks: true,
   144	            include_symlinks: true,
   145	            skip_unchanged: true,
   146	            ignore_existing: false,
   147	            checksum: false,
   148	            compare_mode: LocalCompareMode::default(),
   149	            delete_scope: LocalMirrorDeleteScope::default(),
   150	            workers: num_cpus::get().max(1),
   151	            preserve_times: true,
   152	            debug_mode: false,
   153	            resume: false,
   154	            null_sink: false,
   155	        }
   156	    }
   157	}

 succeeded in 0ms:
     1	//! History-derived plan tuning for the engine's streaming strategy.
     2	//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
     3	
     4	use eyre::Result;
     5	
     6	use crate::perf_history::TransferMode;
     7	
     8	/// Maximum number of recent eligible records the local tuner looks
     9	/// at. The cap exists so a recent regime change (new disk, fresh
    10	/// install) propagates into tuning within ~20 transfers instead of
    11	/// being diluted by older history.
    12	const TUNING_WINDOW_SIZE: usize = 20;
    13	
    14	/// R56-F2: select the window of recent records that should feed
    15	/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
    16	/// FIRST, then the per-operation discriminants, THEN takes the
    17	/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
    18	/// run_kind filter, so 20 recent dry-run / null-sink records with
    19	/// matching mode could fill the window and force tuning to fall
    20	/// back to defaults even when older real records existed.
    21	///
    22	/// Extracted so the contract is unit-testable without touching
    23	/// the global perf-history JSONL.
    24	pub(super) fn select_tuning_window(
    25	    history: &[crate::perf_history::PerformanceRecord],
    26	    target_mode: TransferMode,
    27	    compare_mode: crate::perf_history::CompareModeSnapshot,
    28	    skip_unchanged: bool,
    29	) -> Vec<crate::perf_history::PerformanceRecord> {
    30	    history
    31	        .iter()
    32	        .rev()
    33	        .filter(|record| record.run_kind.is_real_transfer())
    34	        .filter(|record| record.mode == target_mode)
    35	        // R59 finding #5: key on the full comparison policy
    36	        // (not just `checksum: bool`) so SizeMtime / SizeOnly /
    37	        // Force / IgnoreTimes runs don't mix into the same tuning
    38	        // bucket. Pre-fix a session of `--size-only` runs trained
    39	        // the SizeMtime bucket (and vice versa).
    40	        .filter(|record| record.options.compare_mode == compare_mode)
    41	        .filter(|record| record.options.skip_unchanged == skip_unchanged)
    42	        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
    43	        // R58-followup: require a tuning signal. `derive_local_plan_tuning`
    44	        // only aggregates `tar_shard_*` + `raw_bundle_*`; records with
    45	        // `tar_shard_tasks == 0 && raw_bundle_tasks == 0` (no_work,
    46	        // journal_no_work, single_huge_file, streaming no-ops) are
    47	        // RunKind::Real and pass every other gate but contribute
    48	        // nothing. Pre-fix they could fill the 20-slot window and
    49	        // hide older bucket-bearing records. If the tuner ever
    50	        // starts consuming `large_tasks`, add it here too.
    51	        .filter(|record| record.tar_shard_tasks > 0 || record.raw_bundle_tasks > 0)
    52	        .take(TUNING_WINDOW_SIZE)
    53	        .cloned()
    54	        .collect()
    55	}
    56	
    57	/// R57-F1: wrapper that always reads the FULL history before
    58	/// applying the run_kind filter. The caller used to pass
    59	/// `read_recent_records(50)`, which pre-capped the input slice
    60	/// at 50 records — so 50 recent non-real records could hide
    61	/// older real records before `select_tuning_window` ever saw
    62	/// them. Baking the "ask for all records" invariant into the
    63	/// wrapper means the limit can't drift back to a finite value.
    64	/// The history file is already size-capped at ~1 MiB upstream
    65	/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
    66	/// records is bounded.
    67	///
    68	/// Generic over the reader so unit tests can inject a synthetic
    69	/// history; production passes `read_recent_records` directly.
    70	/// Returns `None` if the reader errored OR no eligible records
    71	/// were found; the caller treats either case as "fall back to
    72	/// defaults."
    73	pub(super) fn select_tuning_window_from_history<F>(
    74	    reader: F,
    75	    target_mode: TransferMode,
    76	    compare_mode: crate::perf_history::CompareModeSnapshot,
    77	    skip_unchanged: bool,
    78	) -> Option<Vec<crate::perf_history::PerformanceRecord>>
    79	where
    80	    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
    81	{
    82	    // `0` means "all records" per read_recent_records' contract
    83	    // (see read_records_from_path in perf_history.rs:298). This
    84	    // is the load-bearing literal — passing anything else
    85	    // reintroduces R57-F1.
    86	    let history = reader(0).ok()?;
    87	    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
    88	    if window.is_empty() {
    89	        None
    90	    } else {
    91	        Some(window)
    92	    }
    93	}
    94	
    95	#[cfg(test)]
    96	mod select_tuning_window_tests {
    97	    //! R56-F2: ensure non-real records are filtered BEFORE the
    98	    //! 20-record window, not after. Pre-fix, recent
    99	    //! dry-run/null-sink records with matching mode could fill the
   100	    //! window and force tuning to fall back to defaults even when
   101	    //! older real records existed.
   102	
   103	    use super::*;
   104	    use crate::auto_tune::derive_local_plan_tuning;
   105	    use crate::perf_history::{
   106	        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
   107	    };
   108	    use eyre::eyre;
   109	
   110	    fn record(
   111	        kind: RunKind,
   112	        mode: TransferMode,
   113	        tar_tasks: u32,
   114	        tar_bytes: u64,
   115	        timestamp_ms: u128,
   116	    ) -> PerformanceRecord {
   117	        let mut r = PerformanceRecord::new(
   118	            mode,
   119	            None,
   120	            None,
   121	            10,
   122	            1024,
   123	            OptionSnapshot {
   124	                dry_run: false,
   125	                preserve_symlinks: true,
   126	                include_symlinks: false,
   127	                skip_unchanged: true,
   128	                checksum: false,
   129	                compare_mode: CompareModeSnapshot::SizeMtime,
   130	                workers: 4,
   131	            },
   132	            None,
   133	            10,
   134	            100,
   135	            0,
   136	            0,
   137	        );
   138	        r.run_kind = kind;
   139	        r.tar_shard_tasks = tar_tasks;
   140	        r.tar_shard_files = tar_tasks * 100;
   141	        r.tar_shard_bytes = tar_bytes;
   142	        r.timestamp_epoch_ms = timestamp_ms;
   143	        r
   144	    }
   145	
   146	    /// 30 recent NullSink records (matching the target operation
   147	    /// shape) followed by 5 older Real records. Pre-fix .take(20)
   148	    /// ran first, grabbed 20 NullSinks, derive_local_plan_tuning
   149	    /// skipped them all internally and returned None — tuning
   150	    /// fell back to defaults despite real history being available.
   151	    /// Post-fix, the filter eats the NullSinks before the take, so
   152	    /// the 5 Real records make it through and tuning succeeds.
   153	    #[test]
   154	    fn null_sink_records_do_not_crowd_out_older_real_records() {
   155	        let mut history = Vec::new();
   156	        // Older real records (timestamps lowest = oldest).
   157	        for i in 0..5 {
   158	            history.push(record(
   159	                RunKind::Real,
   160	                TransferMode::Copy,
   161	                4,
   162	                16 * 1024 * 1024,
   163	                100 + i,
   164	            ));
   165	        }
   166	        // Recent null-sink records (higher timestamps = more recent).
   167	        for i in 0..30 {
   168	            history.push(record(
   169	                RunKind::NullSink,
   170	                TransferMode::Copy,
   171	                4,
   172	                512 * 1024 * 1024,
   173	                10_000 + i,
   174	            ));
   175	        }
   176	
   177	        let window = select_tuning_window(
   178	            &history,
   179	            TransferMode::Copy,
   180	            CompareModeSnapshot::SizeMtime,
   181	            true,
   182	        );
   183	        assert!(
   184	            !window.is_empty(),
   185	            "real records must reach the window; 30 NullSink records crowded them out pre-R56-F2"
   186	        );
   187	        assert!(
   188	            window.iter().all(|r| r.run_kind.is_real_transfer()),
   189	            "only Real records should land in the tuning window"
   190	        );
   191	        // derive_local_plan_tuning succeeds → tuner sees its 5 Real
   192	        // records with 16 MiB tar bytes / 4 tar tasks = 4 MiB avg
   193	        // (clamped to the 4 MiB floor).
   194	        let tuning = derive_local_plan_tuning(&window).expect("tuning must succeed");
   195	        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
   196	        assert!(tuning.small_target_bytes <= 16 * 1024 * 1024);
   197	    }
   198	
   199	    #[test]
   200	    fn dry_run_records_do_not_crowd_out_real_records() {
   201	        let mut history = Vec::new();
   202	        for i in 0..3 {
   203	            history.push(record(
   204	                RunKind::Real,
   205	                TransferMode::Copy,
   206	                2,
   207	                8 * 1024 * 1024,
   208	                100 + i,
   209	            ));
   210	        }
   211	        for i in 0..25 {
   212	            history.push(record(
   213	                RunKind::DryRun,
   214	                TransferMode::Copy,
   215	                10,
   216	                1024 * 1024 * 1024,
   217	                10_000 + i,
   218	            ));
   219	        }
   220	        let window = select_tuning_window(
   221	            &history,
   222	            TransferMode::Copy,
   223	            CompareModeSnapshot::SizeMtime,
   224	            true,
   225	        );
   226	        assert_eq!(
   227	            window.len(),
   228	            3,
   229	            "expected the 3 real records, got {} entries",
   230	            window.len()
   231	        );
   232	        assert!(derive_local_plan_tuning(&window).is_some());
   233	    }
   234	
   235	    #[test]
   236	    fn bench_records_do_not_crowd_out_real_records() {
   237	        let mut history = Vec::new();
   238	        for i in 0..2 {
   239	            history.push(record(
   240	                RunKind::Real,

codex
The single-file accounting implementation is visible now: it records `single_file` or `null_sink` and skips predictor update for null-sink. I’m checking the parent code around the moved single-file and async engine body to catch any unlisted changes.
exec
/usr/bin/zsh -lc "git grep -n \"fn execute_single_file_copy\\|pub async fn execute_local_mirror_async\\|select_tuning_window\\|record_performance_history\\|maybe_select_fast_path\" 63b813a -- crates/blit-core/src/orchestrator/orchestrator.rs crates/blit-core/src/orchestrator/options.rs crates/blit-core/src/orchestrator/history.rs crates/blit-core/src/orchestrator/fast_path.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
63b813a:crates/blit-core/src/orchestrator/fast_path.rs:76:pub(super) fn maybe_select_fast_path(
63b813a:crates/blit-core/src/orchestrator/fast_path.rs:227:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
63b813a:crates/blit-core/src/orchestrator/fast_path.rs:250:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
63b813a:crates/blit-core/src/orchestrator/fast_path.rs:273:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
63b813a:crates/blit-core/src/orchestrator/history.rs:27:pub(super) fn record_performance_history(
63b813a:crates/blit-core/src/orchestrator/history.rs:55:/// touching disk. Split out from `record_performance_history` so
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:24:use super::fast_path::{maybe_select_fast_path, FastPathDecision};
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:25:use super::history::{record_performance_history, update_predictor};
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:45:fn select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:82:/// older real records before `select_tuning_window` ever saw
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:94:fn select_tuning_window_from_history<F>(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:108:    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:151:    pub async fn execute_local_mirror_async(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:277:            if let Some(record) = record_performance_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:294:            maybe_select_fast_path(src_root, dest_root, &options)?
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:334:                    if let Some(record) = record_performance_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:371:                    if let Some(record) = record_performance_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:407:                    if let Some(record) = record_performance_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:450:            // 20-record cap inside `select_tuning_window`, but the
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:488:            if let Some(filtered) = select_tuning_window_from_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:616:        // here; `record_performance_history` populates the matching
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:853:        if let Some(record) = record_performance_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:1138:fn execute_single_file_copy(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:1376:    /// `maybe_select_fast_path`'s mirror short-circuit).
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:1972:mod select_tuning_window_tests {
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2051:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2094:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2139:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2163:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2173:    /// `select_tuning_window_from_history` which bakes the
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2206:        // select_tuning_window iterates .rev() so they come first.
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2207:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2261:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2286:    // The bug class isn't about what `select_tuning_window` does
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2288:    // `select_tuning_window_from_history` wraps the reader call so
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2313:        let _ = select_tuning_window_from_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2332:        let result = select_tuning_window_from_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2351:        let result = select_tuning_window_from_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2370:        let result = select_tuning_window_from_history(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2406:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2442:        let window = select_tuning_window(
63b813a:crates/blit-core/src/orchestrator/orchestrator.rs:2455:        let window = select_tuning_window(

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | nl -ba | sed -n '1120,1260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | nl -ba | sed -n '130,270p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1120	                    snap.ctime_nsec,
  1121	                    snap.root_mtime_epoch_ms
  1122	                );
  1123	            }
  1124	        }
  1125	    }
  1126	}
  1127	
  1128	impl Default for TransferOrchestrator {
  1129	    fn default() -> Self {
  1130	        Self::new()
  1131	    }
  1132	}
  1133	
  1134	/// Copy a single file source directly to `dest_root`, bypassing the
  1135	/// enumerator/planner/pipeline machinery which assumes `src_root` is a
  1136	/// directory. The CLI's destination resolver has already produced the final
  1137	/// target path, so this is a simple `copy_file` call.
  1138	fn execute_single_file_copy(
  1139	    src_root: &Path,
  1140	    dest_root: &Path,
  1141	    options: &LocalMirrorOptions,
  1142	    start_time: Instant,
  1143	) -> Result<LocalMirrorSummary> {
  1144	    use crate::buffer::BufferSizer;
  1145	    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
  1146	    use crate::logger::NoopLogger;
  1147	    use filetime::FileTime;
  1148	
  1149	    let src_meta = std::fs::metadata(src_root)
  1150	        .with_context(|| format!("stat source file {}", src_root.display()))?;
  1151	    let size = src_meta.len();
  1152	
  1153	    // R58-followup: route compare-mode for the single-file path
  1154	    // through the same translation the directory path uses
  1155	    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
  1156	    // at `options.checksum`, so `--size-only` / `--ignore-times` /
  1157	    // `--force` were silently dropped — repro: copy src.txt dst.txt
  1158	    // --size-only re-copied even when sizes matched.
  1159	    let compare_mode = match options.compare_mode {
  1160	        crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
  1161	        crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
  1162	        crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
  1163	        crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
  1164	        crate::orchestrator::LocalCompareMode::SizeMtime => {
  1165	            if options.checksum {
  1166	                ComparisonMode::Checksum
  1167	            } else {
  1168	                ComparisonMode::SizeMtime
  1169	            }
  1170	        }
  1171	    };
  1172	
  1173	    // R58-F5: the single-file short-circuit (orchestrator.rs:125)
  1174	    // bypasses the enumerator + planner, which is where the
  1175	    // streaming-pipeline path checks filter / ignore_existing.
  1176	    // Apply both here so single-file copies honor the same
  1177	    // CLI contract.
  1178	    //
  1179	    // Filter: the source root is itself the only entry. Run
  1180	    // `filter.allows_entry` against the source name. If excluded,
  1181	    // return a "scanned 1 / copied 0" summary so the user sees
  1182	    // "no work performed" rather than the file being copied
  1183	    // anyway.
  1184	    let src_name = src_root.file_name().map(PathBuf::from);
  1185	    let allows = match src_name {
  1186	        Some(name) => {
  1187	            let mtime = src_meta.modified().ok();
  1188	            options
  1189	                .filter
  1190	                .allows_entry(Some(&name), src_root, size, mtime)
  1191	        }
  1192	        None => true,
  1193	    };
  1194	    if !allows {
  1195	        return Ok(LocalMirrorSummary {
  1196	            planned_files: 0,
  1197	            copied_files: 0,
  1198	            total_bytes: 0,
  1199	            scanned_files: 1,
  1200	            scanned_bytes: size,
  1201	            duration: start_time.elapsed(),
  1202	            outcome: TransferOutcome::UpToDate,
  1203	            ..Default::default()
  1204	        });
  1205	    }
  1206	
  1207	    // ignore_existing: if the destination file already exists,
  1208	    // skip the copy entirely. Matches the diff_planner behavior
  1209	    // for the streaming-pipeline path (diff_planner.rs).
  1210	    if options.ignore_existing && dest_root.exists() {
  1211	        return Ok(LocalMirrorSummary {
  1212	            planned_files: 0,
  1213	            copied_files: 0,
  1214	            total_bytes: 0,
  1215	            scanned_files: 1,
  1216	            scanned_bytes: size,
  1217	            duration: start_time.elapsed(),
  1218	            outcome: TransferOutcome::UpToDate,
  1219	            ..Default::default()
  1220	        });
  1221	    }
  1222	
  1223	    if options.dry_run {
  1224	        return Ok(LocalMirrorSummary {
  1225	            planned_files: 1,
  1226	            copied_files: 1,
  1227	            total_bytes: size,
  1228	            scanned_files: 1,
  1229	            scanned_bytes: size,
  1230	            dry_run: true,
  1231	            duration: start_time.elapsed(),
  1232	            ..Default::default()
  1233	        });
  1234	    }
  1235	
  1236	    if options.null_sink {
  1237	        return Ok(LocalMirrorSummary {
  1238	            planned_files: 1,
  1239	            copied_files: 1,
  1240	            total_bytes: size,
  1241	            scanned_files: 1,
  1242	            scanned_bytes: size,
  1243	            duration: start_time.elapsed(),
  1244	            ..Default::default()
  1245	        });
  1246	    }
  1247	
  1248	    let mut did_copy = false;
  1249	    let mut clone_succeeded = false;
  1250	    let mut bytes_copied = 0u64;
  1251	
  1252	    if options.resume {
  1253	        let outcome = resume_copy_file(src_root, dest_root, 0)
  1254	            .with_context(|| format!("resume copy {}", src_root.display()))?;
  1255	        did_copy = outcome.bytes_transferred > 0;
  1256	        bytes_copied = outcome.bytes_transferred;
  1257	    } else {
  1258	        let needs_copy = !options.skip_unchanged
  1259	            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
  1260	        if needs_copy {

 succeeded in 0ms:
   130	    pub fn execute_local_mirror(
   131	        &self,
   132	        src_root: &Path,
   133	        dest_root: &Path,
   134	        options: LocalMirrorOptions,
   135	    ) -> Result<LocalMirrorSummary> {
   136	        let workers = options.workers.max(1);
   137	        let runtime = Builder::new_multi_thread()
   138	            .worker_threads(workers)
   139	            .enable_all()
   140	            .build()
   141	            .context("build tokio runtime")?;
   142	        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
   143	    }
   144	
   145	    /// Async core of the local-mirror orchestrator. Callable from
   146	    /// any async context. Closes F9 of the 2026-05-01 baseline
   147	    /// review: previously `execute_local_mirror` built and owned its
   148	    /// own Tokio runtime, which panicked when called from an async
   149	    /// caller. The sync wrapper above is now a thin convenience for
   150	    /// blocking callers.
   151	    pub async fn execute_local_mirror_async(
   152	        &self,
   153	        src_root: &Path,
   154	        dest_root: &Path,
   155	        options: LocalMirrorOptions,
   156	    ) -> Result<LocalMirrorSummary> {
   157	        if !src_root.exists() {
   158	            return Err(eyre!("source path does not exist: {}", src_root.display()));
   159	        }
   160	
   161	        if !options.dry_run {
   162	            if let Some(parent) = dest_root.parent() {
   163	                std::fs::create_dir_all(parent).with_context(|| {
   164	                    format!("failed to create destination parent {}", parent.display())
   165	                })?;
   166	            }
   167	        }
   168	
   169	        let start_time = Instant::now();
   170	
   171	        // Single-file source: bypass the enumerator/planner/pipeline machinery
   172	        // entirely and copy the file directly. The destination resolver in the
   173	        // CLI has already produced the exact target path (accounting for
   174	        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
   175	        // Without this short-circuit, the enumerator would skip the depth-0
   176	        // root entry and the fast-path would report NoWork — silent data loss.
   177	        if src_root.is_file() {
   178	            return execute_single_file_copy(src_root, dest_root, &options, start_time);
   179	        }
   180	
   181	        let mut journal_tracker = ChangeTracker::load().ok();
   182	        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
   183	        let mut journal_skip = false;
   184	
   185	        let mut predictor = PerformancePredictor::load().ok();
   186	
   187	        let copy_config = CopyConfig {
   188	            workers: options.workers.max(1),
   189	            preserve_times: options.preserve_times,
   190	            dry_run: options.dry_run,
   191	            checksum: if options.checksum {
   192	                Some(crate::checksum::ChecksumType::Blake3)
   193	            } else {
   194	                None
   195	            },
   196	            resume: options.resume,
   197	            null_sink: options.null_sink,
   198	        };
   199	
   200	        // Journal fast-path requires BOTH source and destination to exist and
   201	        // report "no changes". A missing destination obviously needs a full
   202	        // transfer — treating it as unchanged would silently skip the work.
   203	        if options.skip_unchanged
   204	            && !options.checksum
   205	            && !options.force_tar
   206	            && !options.null_sink
   207	            && dest_root.exists()
   208	        {
   209	            if let Some(tracker) = journal_tracker.as_ref() {
   210	                match tracker.probe(src_root) {
   211	                    Ok(src_probe) => {
   212	                        let dest_probe = tracker.probe(dest_root).ok();
   213	
   214	                        if src_probe.snapshot.is_some() {
   215	                            journal_tokens.push(src_probe.clone());
   216	                        }
   217	                        if let Some(ref probe) = dest_probe {
   218	                            if probe.snapshot.is_some() {
   219	                                journal_tokens.push(probe.clone());
   220	                            }
   221	                        }
   222	
   223	                        if options.verbose {
   224	                            log_probe("src", &src_probe);
   225	                            if let Some(probe) = dest_probe.as_ref() {
   226	                                log_probe("dest", probe);
   227	                            } else {
   228	                                eprintln!("Journal probe dest unsupported; cannot take fast-path");
   229	                            }
   230	                        }
   231	
   232	                        let src_no_change = matches!(src_probe.state, ChangeState::NoChanges);
   233	                        // If dest_probe is None (unsupported FS), we cannot
   234	                        // assert "no change" — fall through to full planner.
   235	                        let dest_no_change = dest_probe
   236	                            .as_ref()
   237	                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
   238	                            .unwrap_or(false);
   239	
   240	                        if src_no_change && dest_no_change {
   241	                            journal_skip = true;
   242	                        }
   243	                    }
   244	                    Err(err) => {
   245	                        if options.verbose {
   246	                            eprintln!("Filesystem journal probe failed: {err:?}");
   247	                        }
   248	                    }
   249	                }
   250	            }
   251	        }
   252	
   253	        if journal_skip {
   254	            if options.verbose {
   255	                eprintln!(
   256	                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
   257	                );
   258	            }
   259	            if let Some(tracker) = journal_tracker.as_mut() {
   260	                persist_journal_checkpoints(
   261	                    tracker,
   262	                    journal_tokens.as_mut_slice(),
   263	                    options.verbose,
   264	                );
   265	            }
   266	
   267	            // Journal said both sides match, so we never enumerated.
   268	            // scanned_{files,bytes} stay 0 — predictor sees this as
   269	            // "noop with no scan cost" which is what actually happened.
   270	            let summary = LocalMirrorSummary {

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | nl -ba | sed -n '420,900p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   420	            if let Some(tracker) = journal_tracker.as_mut() {
   421	                persist_journal_checkpoints(
   422	                    tracker,
   423	                    journal_tokens.as_mut_slice(),
   424	                    options.verbose,
   425	                );
   426	            }
   427	
   428	            if options.verbose {
   429	                eprintln!(
   430	                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
   431	                    if options.mirror { "mirror" } else { "copy" },
   432	                    summary.copied_files,
   433	                    summary.total_bytes,
   434	                    summary.duration
   435	                );
   436	            }
   437	
   438	            return Ok(summary);
   439	        }
   440	
   441	        // --- Unified pipeline: same path as remote transfers ---
   442	        let mut plan_options = PlanOptions {
   443	            force_tar: options.force_tar,
   444	            ..PlanOptions::default()
   445	        };
   446	
   447	        if options.perf_history {
   448	            // R57-F1: read ALL history, not a pre-cap window. The
   449	            // R56-F2 fix correctly filtered run_kind before the
   450	            // 20-record cap inside `select_tuning_window`, but the
   451	            // caller was still pre-capping at 50 records from the
   452	            // JSONL — so 50 recent non-real records could still
   453	            // hide older real records one layer up. The file is
   454	            // already size-capped at ~1 MiB upstream
   455	            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
   456	            // all records is bounded; `read_recent_records(0)`
   457	            // means "all" per its limit semantics.
   458	            let target_mode = if options.mirror {
   459	                TransferMode::Mirror
   460	            } else {
   461	                TransferMode::Copy
   462	            };
   463	            // R59 finding #5: tuning window keys on full compare_mode,
   464	            // not just options.checksum. Translate via the same enum
   465	            // the history snapshot uses so the bucket lookup matches
   466	            // what the writer recorded.
   467	            let query_compare_mode = match options.compare_mode {
   468	                crate::orchestrator::LocalCompareMode::Checksum => {
   469	                    crate::perf_history::CompareModeSnapshot::Checksum
   470	                }
   471	                crate::orchestrator::LocalCompareMode::SizeOnly => {
   472	                    crate::perf_history::CompareModeSnapshot::SizeOnly
   473	                }
   474	                crate::orchestrator::LocalCompareMode::Force => {
   475	                    crate::perf_history::CompareModeSnapshot::Force
   476	                }
   477	                crate::orchestrator::LocalCompareMode::IgnoreTimes => {
   478	                    crate::perf_history::CompareModeSnapshot::IgnoreTimes
   479	                }
   480	                crate::orchestrator::LocalCompareMode::SizeMtime => {
   481	                    if options.checksum {
   482	                        crate::perf_history::CompareModeSnapshot::Checksum
   483	                    } else {
   484	                        crate::perf_history::CompareModeSnapshot::SizeMtime
   485	                    }
   486	                }
   487	            };
   488	            if let Some(filtered) = select_tuning_window_from_history(
   489	                read_recent_records,
   490	                target_mode,
   491	                query_compare_mode,
   492	                options.skip_unchanged,
   493	            ) {
   494	                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
   495	                    plan_options.small_target = Some(tuning.small_target_bytes);
   496	                    plan_options.small_count_target = Some(tuning.small_count_target);
   497	                    plan_options.medium_target = Some(tuning.medium_target_bytes);
   498	                }
   499	            }
   500	        }
   501	
   502	        let planning_start = Instant::now();
   503	
   504	        let src_root_buf = src_root.to_path_buf();
   505	        let dest_root_buf = dest_root.to_path_buf();
   506	        let filter = options.filter.clone_without_cache();
   507	        let skip_unchanged = options.skip_unchanged;
   508	        let ignore_existing = options.ignore_existing;
   509	        // R58-F7: translate the orchestrator's `compare_mode` (set by
   510	        // the CLI from --size-only / --ignore-times / --force /
   511	        // --checksum / default) onto the unified ComparisonMode enum.
   512	        // Pre-fix this hardcoded a bool→Checksum-or-SizeMtime mapping
   513	        // and ignored the other flags entirely; remote pull already
   514	        // honored all five variants, so behavior diverged by direction.
   515	        //
   516	        // Backward-compat: the old `options.checksum` bool still
   517	        // wins if it's set without `compare_mode` being explicitly
   518	        // changed — preserves the existing `--checksum` behavior
   519	        // for any caller that hasn't migrated yet.
   520	        let compare_mode = match options.compare_mode {
   521	            crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
   522	            crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
   523	            crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
   524	            crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
   525	            crate::orchestrator::LocalCompareMode::SizeMtime => {
   526	                if options.checksum {
   527	                    ComparisonMode::Checksum
   528	                } else {
   529	                    ComparisonMode::SizeMtime
   530	                }
   531	            }
   532	        };
   533	
   534	        // 1. Scan source via FsTransferSource, wrapped in FilteredSource so
   535	        //    the user filter applies through the universal pipeline chokepoint
   536	        //    (identical to push/pull/remote-remote behavior — full parity).
   537	        let inner: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root_buf.clone()));
   538	        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(inner, filter));
   539	        let unreadable = Arc::new(Mutex::new(Vec::new()));
   540	        let (mut header_rx, scan_handle) = source.scan(None, unreadable.clone());
   541	
   542	        // 2. Collect all headers
   543	        let mut all_headers = Vec::new();
   544	        while let Some(h) = header_rx.recv().await {
   545	            all_headers.push(h);
   546	        }
   547	        let _total_scanned = scan_handle
   548	            .await
   549	            .context("scan task panicked")?
   550	            .context("scan failed")?;
   551	
   552	        // 3. Diff + plan via the shared DiffPlanner stage. Combines
   553	        //    the comparison-filter and payload-planning steps that
   554	        //    were previously inline. Behavior preserved bit-for-bit
   555	        //    (size+mtime or Blake3 hash, then tar/large/raw planning).
   556	        let src = src_root_buf.clone();
   557	        let dst = dest_root_buf.clone();
   558	        let plan_opts = plan_options;
   559	        let headers = all_headers.clone();
   560	        let planned = tokio::task::spawn_blocking(move || {
   561	            plan_local_mirror(
   562	                headers,
   563	                LocalDiffInputs {
   564	                    src_root: &src,
   565	                    dst_root: &dst,
   566	                    compare_mode,
   567	                    ignore_existing,
   568	                    plan_options: plan_opts,
   569	                    skip_unchanged,
   570	                },
   571	            )
   572	        })
   573	        .await
   574	        .context("diff_planner task panicked")??;
   575	
   576	        // 5. Create sink and execute unified pipeline
   577	        let sink: Arc<dyn TransferSink> = if copy_config.null_sink {
   578	            Arc::new(NullSink::new())
   579	        } else {
   580	            Arc::new(FsTransferSink::new(
   581	                src_root_buf.clone(),
   582	                dest_root_buf.clone(),
   583	                FsSinkConfig {
   584	                    preserve_times: copy_config.preserve_times,
   585	                    dry_run: copy_config.dry_run,
   586	                    checksum: copy_config.checksum,
   587	                    resume: copy_config.resume,
   588	                    // R58-followup: thread the orchestrator's
   589	                    // compare_mode into the sink. Pre-fix the sink
   590	                    // hard-coded SizeMtime via
   591	                    // file_needs_copy_with_checksum_type, defeating
   592	                    // --force / --ignore-times: the planner emitted
   593	                    // the file but the sink decided "skip" when
   594	                    // mtime+size matched.
   595	                    compare_mode,
   596	                },
   597	            ))
   598	        };
   599	
   600	        // Boundary between planner and transfer phases. `planning_start`
   601	        // covers scan + diff + plan; everything after this `Instant`
   602	        // is the transfer pipeline. §2.8 phase 2 split: pre-fix the
   603	        // record's `planner_duration_ms` field was set to whole-run
   604	        // time, so the v1 predictor effectively trained on `planner =
   605	        // total` for both targets and couldn't distinguish them.
   606	        let plan_done = Instant::now();
   607	        let planner_duration_ms = plan_done.duration_since(planning_start).as_millis();
   608	
   609	        // §2.8 phase 2: query the predictor BEFORE running the
   610	        // pipeline. Surfaces in summary.predictor_estimate so
   611	        // `--verbose` and `blit profile --json` can compare
   612	        // predicted vs actual.
   613	        //
   614	        // R44-F1: query and observation must use the same feature
   615	        // vector. We query with `(scanned_files, scanned_bytes)`
   616	        // here; `record_performance_history` populates the matching
   617	        // `PerformanceRecord.{file_count,total_bytes}` from
   618	        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
   619	        // record was populated from `summary.copied_files`, so on
   620	        // any incremental run the predictor was queried with one
   621	        // workload size and trained against another.
   622	        //
   623	        // src_fs/dest_fs are left None for 0.1.0 — wiring
   624	        // `fs_capability` per-path probes into the predictor query
   625	        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
   626	        let scanned_files = all_headers.len();
   627	        let scanned_bytes: u64 = all_headers.iter().map(|h| h.size).sum();
   628	        // R45 follow-up to R44-F1: never alias `total_bytes` to
   629	        // `scanned_bytes`. `summary.total_bytes` is the
   630	        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
   631	        // rustdoc); the predictor uses scan features only. Pre-fix
   632	        // this aliased the two so `summary.total_bytes` reported
   633	        // scanned bytes as bytes-written, overcounting throughput
   634	        // on incremental runs.
   635	        let predictor_estimate = predictor.as_ref().and_then(|p| {
   636	            let kind_total = crate::perf_predictor::DurationKind::Total;
   637	            let mode = if options.mirror {
   638	                crate::perf_history::TransferMode::Mirror
   639	            } else {
   640	                crate::perf_history::TransferMode::Copy
   641	            };
   642	            let total_pred = p.predict(
   643	                kind_total,
   644	                mode.clone(),
   645	                None,
   646	                None,
   647	                None,
   648	                options.skip_unchanged,
   649	                options.checksum,
   650	                scanned_files,
   651	                scanned_bytes,
   652	            )?;
   653	            // Pull planner + transfer separately too so the verbose
   654	            // line and the JSON profile can break down the estimate.
   655	            // All three predictor calls share the same
   656	            // (scanned_files, scanned_bytes) feature vector — both
   657	            // for consistency with the recording side, and so a
   658	            // future maintainer can't accidentally reintroduce a
   659	            // train/query mismatch by editing one branch and
   660	            // missing another.
   661	            let planner_pred = p
   662	                .predict(
   663	                    crate::perf_predictor::DurationKind::Planner,
   664	                    mode.clone(),
   665	                    None,
   666	                    None,
   667	                    None,
   668	                    options.skip_unchanged,
   669	                    options.checksum,
   670	                    scanned_files,
   671	                    scanned_bytes,
   672	                )
   673	                .map(|p| p.predicted_ms)
   674	                .unwrap_or(0.0);
   675	            let transfer_pred = p
   676	                .predict(
   677	                    crate::perf_predictor::DurationKind::Transfer,
   678	                    mode,
   679	                    None,
   680	                    None,
   681	                    None,
   682	                    options.skip_unchanged,
   683	                    options.checksum,
   684	                    scanned_files,
   685	                    scanned_bytes,
   686	                )
   687	                .map(|p| p.predicted_ms)
   688	                .unwrap_or(0.0);
   689	            Some(super::summary::PredictorEstimate {
   690	                planner_ms: planner_pred.max(0.0) as u128,
   691	                transfer_ms: transfer_pred.max(0.0) as u128,
   692	                total_ms: total_pred.predicted_ms.max(0.0) as u128,
   693	                observations: total_pred.observations,
   694	                fallback_depth: total_pred.fallback_depth,
   695	            })
   696	        });
   697	        if options.verbose {
   698	            if let Some(est) = predictor_estimate.as_ref() {
   699	                eprintln!(
   700	                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
   701	                     total ~{} ms (n={}, fallback_depth={})",
   702	                    est.planner_ms,
   703	                    est.transfer_ms,
   704	                    est.total_ms,
   705	                    est.observations,
   706	                    est.fallback_depth
   707	                );
   708	            } else {
   709	                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
   710	            }
   711	        }
   712	
   713	        let pipeline_outcome = execute_sink_pipeline(
   714	            source,
   715	            vec![sink],
   716	            planned.payloads,
   717	            DEFAULT_PAYLOAD_PREFETCH,
   718	            None,
   719	        )
   720	        .await
   721	        .context("transfer pipeline failed")?;
   722	        let transfer_duration_ms = plan_done.elapsed().as_millis();
   723	
   724	        // R47-F4: snapshot unreadable paths so the CLI's source-
   725	        // delete step (in `blit move`) can refuse to remove a
   726	        // source it couldn't fully scan. The R46-F2 gate inside
   727	        // the orchestrator only fires on `options.mirror`, but
   728	        // move uses mirror=false — without this surface, an
   729	        // unreadable source file would get skipped during the
   730	        // copy and then silently deleted from the source by the
   731	        // CLI's `remove_dir_all` step.
   732	        let unreadable_snapshot: Vec<String> = unreadable
   733	            .lock()
   734	            .map(|guard| guard.clone())
   735	            .unwrap_or_default();
   736	
   737	        let mut summary = LocalMirrorSummary {
   738	            planned_files: pipeline_outcome.files_written,
   739	            copied_files: pipeline_outcome.files_written,
   740	            // R45: bytes the pipeline actually wrote, not scanned
   741	            // bytes. Distinct on incremental runs.
   742	            total_bytes: pipeline_outcome.bytes_written,
   743	            scanned_files,
   744	            scanned_bytes,
   745	            dry_run: options.dry_run,
   746	            duration: start_time.elapsed(),
   747	            predictor_estimate: predictor_estimate.clone(),
   748	            unreadable_paths: unreadable_snapshot.clone(),
   749	            ..Default::default()
   750	        };
   751	
   752	        if options.mirror {
   753	            // R46-F2: refuse to mirror-delete when the source scan
   754	            // was incomplete. The `unreadable_snapshot` captured
   755	            // above (R47-F4) covers the per-file open path
   756	            // (PermissionDenied / NotFound on individual files) and
   757	            // the walkdir non-root error path (unreadable
   758	            // subdirectories). Either case means the header set
   759	            // we're about to use as the source-of-truth for "what
   760	            // the destination should contain" is missing entries,
   761	            // and a delete pass would silently remove matching
   762	            // destination subtrees.
   763	            if !unreadable_snapshot.is_empty() {
   764	                bail!(
   765	                    "refusing to mirror-delete from {}: source scan was \
   766	                     incomplete ({} unreadable entr{}); the first {} \
   767	                     reported: {}. Resolve the scan errors (typically \
   768	                     permissions) or run as a non-mirror copy.",
   769	                    dest_root.display(),
   770	                    unreadable_snapshot.len(),
   771	                    if unreadable_snapshot.len() == 1 {
   772	                        "y"
   773	                    } else {
   774	                        "ies"
   775	                    },
   776	                    unreadable_snapshot.len().min(5),
   777	                    unreadable_snapshot
   778	                        .iter()
   779	                        .take(5)
   780	                        .cloned()
   781	                        .collect::<Vec<_>>()
   782	                        .join("; "),
   783	                );
   784	            }
   785	
   786	            let source_paths: HashSet<String> = all_headers
   787	                .iter()
   788	                .map(|h| h.relative_path.clone())
   789	                .collect();
   790	            let deletions = apply_mirror_deletions(
   791	                &source_paths,
   792	                dest_root,
   793	                &options.filter,
   794	                options.delete_scope,
   795	                !options.dry_run,
   796	                options.verbose,
   797	            )?;
   798	            summary.deleted_files = deletions.0;
   799	            summary.deleted_dirs = deletions.1;
   800	        }
   801	
   802	        if let Some(tracker) = journal_tracker.as_mut() {
   803	            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
   804	        }
   805	
   806	        if options.verbose {
   807	            eprintln!(
   808	                "Planning enumerated {} file(s), {} bytes",
   809	                scanned_files, scanned_bytes
   810	            );
   811	            eprintln!(
   812	                "Completed local {}: {} file(s), {} bytes in {:.2?} (plan {} ms, xfer {} ms)",
   813	                if options.mirror { "mirror" } else { "copy" },
   814	                summary.copied_files,
   815	                summary.total_bytes,
   816	                summary.duration,
   817	                planner_duration_ms,
   818	                transfer_duration_ms,
   819	            );
   820	            // §2.8: side-by-side predicted-vs-actual so operators
   821	            // can audit the predictor against this run's actual
   822	            // numbers. The bare percentage error per phase is the
   823	            // most useful single number; we keep absolute ms in the
   824	            // line above for context.
   825	            if let Some(est) = summary.predictor_estimate.as_ref() {
   826	                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
   827	                    if actual_ms == 0 {
   828	                        "n/a".to_string()
   829	                    } else {
   830	                        let pred = predicted_ms as f64;
   831	                        let act = actual_ms as f64;
   832	                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
   833	                    }
   834	                };
   835	                eprintln!(
   836	                    "Predictor delta: planner {} ({} vs {} ms), \
   837	                     transfer {} ({} vs {} ms)",
   838	                    pct(est.planner_ms, planner_duration_ms),
   839	                    est.planner_ms,
   840	                    planner_duration_ms,
   841	                    pct(est.transfer_ms, transfer_duration_ms),
   842	                    est.transfer_ms,
   843	                    transfer_duration_ms,
   844	                );
   845	            }
   846	        }
   847	
   848	        let fast_path_label = if options.null_sink {
   849	            Some("null_sink")
   850	        } else {
   851	            None
   852	        };
   853	        if let Some(record) = record_performance_history(
   854	            &summary,
   855	            &options,
   856	            fast_path_label,
   857	            planner_duration_ms,
   858	            transfer_duration_ms,
   859	        ) {
   860	            // Don't update the predictor from null-sink runs — the zero
   861	            // write cost would teach it that transfers are faster than
   862	            // they really are.
   863	            if !options.null_sink {
   864	                update_predictor(&mut predictor, &record, options.verbose);
   865	            }
   866	        }
   867	
   868	        Ok(summary)
   869	    }
   870	}
   871	
   872	/// Delete destination files/dirs not present in the source header set.
   873	///
   874	/// R58-F6: `delete_scope` controls which destination entries are
   875	/// even considered for deletion:
   876	///   - `FilteredSubset` (default): enumerate the destination
   877	///     *through the user's filter*, then delete entries not in
   878	///     the source set. Excluded files (e.g. `*.log` when
   879	///     `--exclude '*.log'`) are out of scope — they're not
   880	///     candidates for deletion, and their parent directories are
   881	///     therefore non-empty from the user's perspective. When
   882	///     `remove_dir` fails with ENOTEMPTY on a parent whose only
   883	///     remaining contents are out-of-scope, we treat it as
   884	///     expected, not as an error.
   885	///   - `All`: enumerate the destination *without* the filter so
   886	///     every entry is in scope. ENOTEMPTY is a genuine error
   887	///     here (we did walk everything, so something other than
   888	///     filter-excluded content must be in the way).
   889	fn apply_mirror_deletions(
   890	    source_paths: &HashSet<String>,
   891	    dest_root: &Path,
   892	    filter: &FileFilter,
   893	    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
   894	    perform: bool,
   895	    verbose: bool,
   896	) -> Result<(usize, usize)> {
   897	    use crate::enumeration::{EntryKind, FileEnumerator};
   898	    use crate::orchestrator::LocalMirrorDeleteScope;
   899	
   900	    // R58-F6: FilteredSubset uses the user's filter for the

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | nl -ba | sed -n '270,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   270	            let summary = LocalMirrorSummary {
   271	                dry_run: options.dry_run,
   272	                duration: start_time.elapsed(),
   273	                outcome: TransferOutcome::JournalSkip,
   274	                ..Default::default()
   275	            };
   276	
   277	            if let Some(record) = record_performance_history(
   278	                &summary,
   279	                &options,
   280	                Some("journal_no_work"),
   281	                0,
   282	                summary.duration.as_millis(),
   283	            ) {
   284	                update_predictor(&mut predictor, &record, options.verbose);
   285	            }
   286	
   287	            return Ok(summary);
   288	        }
   289	
   290	        // Skip fast path when using null sink — it bypasses the sink abstraction.
   291	        let fast_path_outcome = if options.null_sink {
   292	            super::fast_path::FastPathOutcome::streaming()
   293	        } else {
   294	            maybe_select_fast_path(src_root, dest_root, &options)?
   295	        };
   296	        if let Some(decision) = fast_path_outcome.decision {
   297	            // R47-F4: propagate the fast-path scan's suppressed
   298	            // errors into the per-branch summary. Each fast-path
   299	            // outcome below clones this into `unreadable_paths`
   300	            // so the CLI's source-delete step can detect a
   301	            // partial scan even on the Tiny/Huge/NoWork paths.
   302	            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
   303	            let summary = match decision {
   304	                FastPathDecision::NoWork { examined } => {
   305	                    let outcome = if examined == 0 {
   306	                        TransferOutcome::SourceEmpty
   307	                    } else {
   308	                        TransferOutcome::UpToDate
   309	                    };
   310	                    if options.verbose {
   311	                        match outcome {
   312	                            TransferOutcome::SourceEmpty => {
   313	                                eprintln!("Fast-path routing: source yielded no file entries")
   314	                            }
   315	                            _ => eprintln!(
   316	                                "Fast-path routing: {} files examined, all up to date",
   317	                                examined
   318	                            ),
   319	                        }
   320	                    }
   321	                    // NoWork ran a real fast-path scan but copied nothing.
   322	                    // scanned_files = examined captures the planner-side
   323	                    // workload; scanned_bytes is 0 because the fast-path
   324	                    // scanner only resolves names + identity, not sizes.
   325	                    let summary = LocalMirrorSummary {
   326	                        planned_files: examined,
   327	                        scanned_files: examined,
   328	                        dry_run: options.dry_run,
   329	                        duration: start_time.elapsed(),
   330	                        outcome,
   331	                        unreadable_paths: fast_path_unreadable.clone(),
   332	                        ..Default::default()
   333	                    };
   334	                    if let Some(record) = record_performance_history(
   335	                        &summary,
   336	                        &options,
   337	                        Some("no_work"),
   338	                        0,
   339	                        summary.duration.as_millis(),
   340	                    ) {
   341	                        update_predictor(&mut predictor, &record, options.verbose);
   342	                    }
   343	                    summary
   344	                }
   345	                FastPathDecision::Tiny { files } => {
   346	                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
   347	                    if options.verbose {
   348	                        eprintln!(
   349	                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
   350	                            files.len(),
   351	                            total_bytes
   352	                        );
   353	                    }
   354	                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
   355	                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
   356	                    // Tiny copies everything it scanned, so scanned ==
   357	                    // copied here. Setting both lets the predictor
   358	                    // train on the actual workload size for the
   359	                    // tiny_manifest fast-path key.
   360	                    let summary = LocalMirrorSummary {
   361	                        planned_files: files.len(),
   362	                        copied_files: files.len(),
   363	                        total_bytes,
   364	                        scanned_files: files.len(),
   365	                        scanned_bytes: total_bytes,
   366	                        dry_run: options.dry_run,
   367	                        duration: start_time.elapsed(),
   368	                        unreadable_paths: fast_path_unreadable.clone(),
   369	                        ..Default::default()
   370	                    };
   371	                    if let Some(record) = record_performance_history(
   372	                        &summary,
   373	                        &options,
   374	                        Some("tiny_manifest"),
   375	                        0,
   376	                        summary.duration.as_millis(),
   377	                    ) {
   378	                        update_predictor(&mut predictor, &record, options.verbose);
   379	                    }
   380	                    summary
   381	                }
   382	                FastPathDecision::Huge { file, size } => {
   383	                    if options.verbose {
   384	                        eprintln!(
   385	                            "Fast-path routing: huge file {} ({} bytes)",
   386	                            file.display(),
   387	                            size
   388	                        );
   389	                    }
   390	                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
   391	                    // Huge fast-path copies a single file: scan size
   392	                    // and copy size are identical (one file, `size`
   393	                    // bytes).
   394	                    let summary = LocalMirrorSummary {
   395	                        planned_files: 1,
   396	                        copied_files: 1,
   397	                        total_bytes: size,
   398	                        scanned_files: 1,
   399	                        scanned_bytes: size,
   400	                        dry_run: options.dry_run,
   401	                        duration: start_time.elapsed(),
   402	                        large_tasks: 1,
   403	                        large_bytes: size,
   404	                        unreadable_paths: fast_path_unreadable.clone(),
   405	                        ..Default::default()
   406	                    };
   407	                    if let Some(record) = record_performance_history(
   408	                        &summary,
   409	                        &options,
   410	                        Some("single_huge_file"),
   411	                        0,
   412	                        summary.duration.as_millis(),
   413	                    ) {
   414	                        update_predictor(&mut predictor, &record, options.verbose);
   415	                    }
   416	                    summary
   417	                }
   418	            };
   419	
   420	            if let Some(tracker) = journal_tracker.as_mut() {
   421	                persist_journal_checkpoints(
   422	                    tracker,
   423	                    journal_tokens.as_mut_slice(),
   424	                    options.verbose,
   425	                );
   426	            }
   427	
   428	            if options.verbose {
   429	                eprintln!(
   430	                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | nl -ba | sed -n '1260,1335p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1260	        if needs_copy {
  1261	            let sizer = BufferSizer::default();
  1262	            let logger = NoopLogger;
  1263	            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
  1264	                .with_context(|| format!("copy {}", src_root.display()))?;
  1265	            did_copy = true;
  1266	            clone_succeeded = outcome.clone_succeeded;
  1267	            bytes_copied = outcome.bytes_copied;
  1268	        }
  1269	    }
  1270	
  1271	    if options.preserve_times && did_copy && !clone_succeeded {
  1272	        if let Ok(modified) = src_meta.modified() {
  1273	            let ft = FileTime::from_system_time(modified);
  1274	            // R42-F1: warn-don't-silence (was `let _ = ...`).
  1275	            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
  1276	                log::warn!("set mtime on {}: {}", dest_root.display(), e);
  1277	            }
  1278	        }
  1279	    }
  1280	
  1281	    Ok(LocalMirrorSummary {
  1282	        planned_files: 1,
  1283	        copied_files: if did_copy { 1 } else { 0 },
  1284	        total_bytes: bytes_copied,
  1285	        // Single-file path always saw exactly one entry of `size`
  1286	        // bytes; whether we copied it or not is the
  1287	        // copied_files/total_bytes story, but the scan saw it.
  1288	        scanned_files: 1,
  1289	        scanned_bytes: size,
  1290	        duration: start_time.elapsed(),
  1291	        outcome: if did_copy {
  1292	            TransferOutcome::Transferred
  1293	        } else {
  1294	            TransferOutcome::UpToDate
  1295	        },
  1296	        ..Default::default()
  1297	    })
  1298	}
  1299	
  1300	#[cfg(test)]
  1301	mod async_runtime_tests {
  1302	    //! F9 regression: `execute_local_mirror_async` must be callable
  1303	    //! from inside an existing Tokio runtime without panicking. The
  1304	    //! sync `execute_local_mirror` wrapper builds its own runtime
  1305	    //! and would panic with "Cannot start a runtime from within a
  1306	    //! runtime" if called from `#[tokio::test]`.
  1307	    use super::*;
  1308	    use tempfile::tempdir;
  1309	
  1310	    fn write_file(path: &std::path::Path, body: &[u8]) {
  1311	        if let Some(parent) = path.parent() {
  1312	            std::fs::create_dir_all(parent).unwrap();
  1313	        }
  1314	        std::fs::write(path, body).unwrap();
  1315	    }
  1316	
  1317	    fn opts() -> LocalMirrorOptions {
  1318	        LocalMirrorOptions {
  1319	            workers: 2,
  1320	            preserve_times: false,
  1321	            dry_run: false,
  1322	            checksum: false,
  1323	            ..Default::default()
  1324	        }
  1325	    }
  1326	
  1327	    #[tokio::test]
  1328	    async fn async_version_callable_from_async_context() {
  1329	        // The whole point of F9 — calling the async version from
  1330	        // within #[tokio::test]'s runtime must not build a nested
  1331	        // runtime or panic.
  1332	        let tmp = tempdir().unwrap();
  1333	        let src = tmp.path().join("src");
  1334	        let dst = tmp.path().join("dst");
  1335	        write_file(&src.join("a.txt"), b"hello");

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/history.rs | nl -ba | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::perf_history::{
     2	    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
     3	};
     4	use crate::perf_predictor::PerformancePredictor;
     5	
     6	use super::{LocalCompareMode, LocalMirrorOptions, LocalMirrorSummary};
     7	
     8	/// Map the orchestrator's `LocalCompareMode` onto the perf-history
     9	/// snapshot enum so tuning records preserve the user's full intent
    10	/// (not just `checksum: bool`).
    11	fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
    12	    match options.compare_mode {
    13	        LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
    14	        LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
    15	        LocalCompareMode::Force => CompareModeSnapshot::Force,
    16	        LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
    17	        LocalCompareMode::SizeMtime => {
    18	            if options.checksum {
    19	                CompareModeSnapshot::Checksum
    20	            } else {
    21	                CompareModeSnapshot::SizeMtime
    22	            }
    23	        }
    24	    }
    25	}
    26	
    27	pub(super) fn record_performance_history(
    28	    summary: &LocalMirrorSummary,
    29	    options: &LocalMirrorOptions,
    30	    fast_path: Option<&str>,
    31	    planner_duration_ms: u128,
    32	    transfer_duration_ms: u128,
    33	) -> Option<PerformanceRecord> {
    34	    if !options.perf_history {
    35	        return None;
    36	    }
    37	
    38	    let record = build_performance_record(
    39	        summary,
    40	        options,
    41	        fast_path,
    42	        planner_duration_ms,
    43	        transfer_duration_ms,
    44	    );
    45	
    46	    if let Err(err) = append_local_record(&record) {
    47	        if options.verbose {
    48	            eprintln!("Failed to update performance history: {err:?}");
    49	        }
    50	    }
    51	    Some(record)
    52	}
    53	
    54	/// Construct the `PerformanceRecord` from a summary without
    55	/// touching disk. Split out from `record_performance_history` so
    56	/// the record-shape contract — specifically R44-F1's "train and
    57	/// query against the same feature vector" invariant — is
    58	/// unit-testable without writing to the global perf history file.
    59	fn build_performance_record(
    60	    summary: &LocalMirrorSummary,
    61	    options: &LocalMirrorOptions,
    62	    fast_path: Option<&str>,
    63	    planner_duration_ms: u128,
    64	    transfer_duration_ms: u128,
    65	) -> PerformanceRecord {
    66	    let options_snapshot = OptionSnapshot {
    67	        dry_run: options.dry_run,
    68	        preserve_symlinks: options.preserve_symlinks,
    69	        include_symlinks: options.include_symlinks,
    70	        skip_unchanged: options.skip_unchanged,
    71	        checksum: options.checksum,
    72	        compare_mode: snapshot_compare_mode(options),
    73	        workers: options.workers,
    74	    };
    75	
    76	    let mode = if options.mirror {
    77	        TransferMode::Mirror
    78	    } else {
    79	        TransferMode::Copy
    80	    };
    81	
    82	    // R44-F1: train against scanned features so the predictor's
    83	    // training inputs match its query inputs. The orchestrator
    84	    // queries `predict(...)` with `all_headers.len()` (scanned
    85	    // count) and `total_bytes` (scanned bytes); pre-fix the record
    86	    // was populated with `summary.copied_files`, so the predictor
    87	    // saw a different feature vector at training time than at
    88	    // query time, and predictions drifted on every incremental
    89	    // workload. The `total_bytes` field on the record was already
    90	    // scanned-bytes by accident; this aligns both axes deliberately.
    91	    //
    92	    // `summary.copied_files` and the per-bucket counts
    93	    // (tar_shard_files / raw_bundle_files / large_tasks) still
    94	    // reflect actual writes — they're the load-bearing inputs for
    95	    // `derive_local_plan_tuning`'s bucket-target heuristics, which
    96	    // are computed from observed apply behavior, not scan size.
    97	    let mut record = PerformanceRecord::new(
    98	        mode,
    99	        None,
   100	        None,
   101	        summary.scanned_files,
   102	        summary.scanned_bytes,
   103	        options_snapshot,
   104	        fast_path.map(|s| s.to_string()),
   105	        planner_duration_ms,
   106	        transfer_duration_ms,
   107	        0,
   108	        0,
   109	    );
   110	    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
   111	    record.tar_shard_files = summary.tar_shard_files as u32;
   112	    record.tar_shard_bytes = summary.tar_shard_bytes;
   113	    record.raw_bundle_tasks = summary.raw_bundle_tasks as u32;
   114	    record.raw_bundle_files = summary.raw_bundle_files as u32;
   115	    record.raw_bundle_bytes = summary.raw_bundle_bytes;
   116	    record.large_tasks = summary.large_tasks as u32;
   117	    record.large_bytes = summary.large_bytes;
   118	
   119	    record
   120	}
   121	
   122	pub(super) fn update_predictor(
   123	    predictor: &mut Option<PerformancePredictor>,
   124	    record: &PerformanceRecord,
   125	    verbose: bool,
   126	) {
   127	    if let Some(ref mut predictor) = predictor {
   128	        predictor.observe(record);
   129	        if let Err(err) = predictor.save() {
   130	            if verbose {
   131	                eprintln!("Failed to persist predictor state: {err:?}");
   132	            }
   133	        }
   134	    }
   135	}
   136	
   137	#[cfg(test)]
   138	mod tests {
   139	    use super::*;
   140	    use crate::orchestrator::TransferOutcome;
   141	    use std::time::Duration;
   142	
   143	    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
   144	        LocalMirrorOptions {
   145	            mirror,
   146	            ..LocalMirrorOptions::default()
   147	        }
   148	    }
   149	
   150	    /// R44-F1 contract: the record's `(file_count, total_bytes)`
   151	    /// must mirror the orchestrator's predictor-query features.
   152	    /// Pre-fix this assertion would have failed: the record was
   153	    /// populated from `summary.copied_files` and `summary.total_bytes`
   154	    /// while the query used scanned values, so on this incremental
   155	    /// scenario (1000 scanned, 5 actually written) the predictor
   156	    /// trained on (5, 100KB) but was queried with
   157	    /// (1000, ~10MB).
   158	    #[test]
   159	    fn record_uses_scanned_features_not_copied() {
   160	        let summary = LocalMirrorSummary {
   161	            // Mostly-unchanged incremental run: 1000 files scanned,
   162	            // only 5 actually written.
   163	            scanned_files: 1000,
   164	            scanned_bytes: 10 * 1024 * 1024,
   165	            planned_files: 5,
   166	            copied_files: 5,
   167	            total_bytes: 100 * 1024,
   168	            duration: Duration::from_millis(200),
   169	            outcome: TransferOutcome::Transferred,
   170	            ..LocalMirrorSummary::default()
   171	        };
   172	        let options = options_with_mirror(false);
   173	        let record = build_performance_record(&summary, &options, Some("streaming"), 150, 50);
   174	
   175	        assert_eq!(
   176	            record.file_count, 1000,
   177	            "record.file_count must reflect scanned (planner-side) workload, not copied count"
   178	        );
   179	        assert_eq!(
   180	            record.total_bytes, summary.scanned_bytes,
   181	            "record.total_bytes must reflect scanned bytes, not transferred bytes"
   182	        );
   183	        assert_eq!(record.planner_duration_ms, 150);
   184	        assert_eq!(record.transfer_duration_ms, 50);
   185	    }
   186	
   187	    /// Bucket-shape fields (tar_shard_*, raw_bundle_*, large_*)
   188	    /// must continue to reflect actual write activity — they feed
   189	    /// `derive_local_plan_tuning` which heuristically sizes
   190	    /// destination buckets from past apply behavior.
   191	    #[test]
   192	    fn bucket_counts_still_reflect_actual_writes() {
   193	        let summary = LocalMirrorSummary {
   194	            scanned_files: 100,
   195	            scanned_bytes: 1_000_000,
   196	            copied_files: 10,
   197	            total_bytes: 50_000,
   198	            tar_shard_tasks: 2,
   199	            tar_shard_files: 7,
   200	            tar_shard_bytes: 30_000,
   201	            raw_bundle_tasks: 1,
   202	            raw_bundle_files: 2,
   203	            raw_bundle_bytes: 15_000,
   204	            large_tasks: 1,
   205	            large_bytes: 5_000,
   206	            ..LocalMirrorSummary::default()
   207	        };
   208	        let options = options_with_mirror(true);
   209	        let record = build_performance_record(&summary, &options, Some("streaming"), 30, 70);
   210	
   211	        assert_eq!(record.tar_shard_tasks, 2);
   212	        assert_eq!(record.tar_shard_files, 7);
   213	        assert_eq!(record.tar_shard_bytes, 30_000);
   214	        assert_eq!(record.raw_bundle_tasks, 1);
   215	        assert_eq!(record.raw_bundle_files, 2);
   216	        assert_eq!(record.raw_bundle_bytes, 15_000);
   217	        assert_eq!(record.large_tasks, 1);
   218	        assert_eq!(record.large_bytes, 5_000);
   219	    }
   220	}

exec
/usr/bin/zsh -lc "git show 63b813a:crates/blit-core/src/orchestrator/options.rs | nl -ba | sed -n '1,190p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::fs_enum::FileFilter;
     2	
     3	/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
     4	/// (FilteredSubset / All) plus a `false`/`true` flag form. R58-F6:
     5	/// pre-fix, local mirror had no plumbing for this — `apply_mirror_deletions`
     6	/// always operated on whatever the transfer filter let through. The
     7	/// remote pull path already supports both modes via
     8	/// `PullSyncOptions.delete_all_scope`; this brings local up to parity.
     9	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    10	pub enum LocalMirrorDeleteScope {
    11	    /// Default: only delete destination entries that the source-side
    12	    /// filter would have allowed. Files matching `--exclude` patterns
    13	    /// at the destination are left alone, because they're not in
    14	    /// scope for this mirror operation.
    15	    #[default]
    16	    FilteredSubset,
    17	    /// Delete every destination entry not present at the source,
    18	    /// regardless of filter scope. Selected via `--delete-scope all`.
    19	    All,
    20	}
    21	
    22	/// Local comparison policy. Mirrors the wire-side `ComparisonMode` enum
    23	/// for the pull / remote-remote-direct paths so local copy/mirror
    24	/// behaves the same as a same-options remote run.
    25	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    26	pub enum LocalCompareMode {
    27	    /// Default size + mtime. Skip if both match.
    28	    #[default]
    29	    SizeMtime,
    30	    /// Compare by Blake3 checksum. Slow but content-accurate.
    31	    Checksum,
    32	    /// Compare by size only. Mtime differences are ignored.
    33	    SizeOnly,
    34	    /// Transfer regardless of target state.
    35	    Force,
    36	    /// Transfer all files unconditionally (--ignore-times). Same
    37	    /// outcome as Force at the planner level; kept as a separate
    38	    /// variant so the user's intent is preserved in summaries.
    39	    IgnoreTimes,
    40	}
    41	
    42	/// Options for executing a local mirror/copy operation.
    43	#[derive(Clone, Debug)]
    44	pub struct LocalMirrorOptions {
    45	    pub filter: FileFilter,
    46	    pub mirror: bool,
    47	    pub dry_run: bool,
    48	    pub progress: bool,
    49	    pub verbose: bool,
    50	    pub perf_history: bool,
    51	    pub force_tar: bool,
    52	    pub preserve_symlinks: bool,
    53	    pub include_symlinks: bool,
    54	    pub skip_unchanged: bool,
    55	    /// Skip any file the destination already has, regardless of
    56	    /// comparison mode. Orthogonal to `checksum`/`skip_unchanged`;
    57	    /// matches the `ignore_existing` field on `TransferOperationSpec`
    58	    /// for full pipeline parity across local/push/pull paths.
    59	    pub ignore_existing: bool,
    60	    pub checksum: bool,
    61	    /// R58-F7: comparison policy. The orchestrator picks
    62	    /// `compare_mode` based on this rather than just the `checksum`
    63	    /// bool, so `--size-only` / `--ignore-times` / `--force` get
    64	    /// honored on local copy/mirror the same way the pull path
    65	    /// honors them.
    66	    pub compare_mode: LocalCompareMode,
    67	    /// R58-F6: delete-scope policy for mirror. Only consulted when
    68	    /// `mirror == true`. Defaults to FilteredSubset so a
    69	    /// `mirror --exclude '*.log'` doesn't delete the destination's
    70	    /// `*.log` files just because they were out of scope for the
    71	    /// source filter.
    72	    pub delete_scope: LocalMirrorDeleteScope,
    73	    pub workers: usize,
    74	    pub preserve_times: bool,
    75	    pub debug_mode: bool,
    76	    /// Resume interrupted transfers using block-level comparison.
    77	    pub resume: bool,
    78	    /// Discard writes (NullSink). Measures source read + pipeline throughput.
    79	    pub null_sink: bool,
    80	}
    81	
    82	impl Default for LocalMirrorOptions {
    83	    fn default() -> Self {
    84	        Self {
    85	            filter: FileFilter::default(),
    86	            mirror: false,
    87	            dry_run: false,
    88	            progress: false,
    89	            verbose: false,
    90	            perf_history: true,
    91	            force_tar: false,
    92	            preserve_symlinks: true,
    93	            include_symlinks: true,
    94	            skip_unchanged: true,
    95	            ignore_existing: false,
    96	            checksum: false,
    97	            compare_mode: LocalCompareMode::default(),
    98	            delete_scope: LocalMirrorDeleteScope::default(),
    99	            workers: num_cpus::get().max(1),
   100	            preserve_times: true,
   101	            debug_mode: false,
   102	            resume: false,
   103	            null_sink: false,
   104	        }
   105	    }
   106	}

codex
The parent snippets match the new structure in the broad strokes. I’m using targeted diffs for the single-file body and the transferred helper modules now, because silent drift is easiest to miss in a large moved-file diff.
exec
/usr/bin/zsh -lc 'diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/fast_path.rs) <(git show 29e210b:crates/blit-core/src/engine/strategy.rs)' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/summary.rs) <(git show 29e210b:crates/blit-core/src/engine/summary.rs)' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/options.rs) <(git show 29e210b:crates/blit-core/src/engine/options.rs)' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/history.rs) <(git show 29e210b:crates/blit-core/src/engine/history.rs)' in /home/michael/dev/Blit
 exited 1 in 0ms:
--- /proc/self/fd/11	2026-07-03 12:21:25.830064644 -0400
+++ /proc/self/fd/12	2026-07-03 12:21:25.830064644 -0400
@@ -39,6 +39,57 @@
     IgnoreTimes,
 }
 
+impl LocalCompareMode {
+    /// Resolve onto the unified wire-side `ComparisonMode`, honoring
+    /// the legacy `checksum: bool` under the default `SizeMtime`
+    /// (back-compat: `--checksum` callers that haven't migrated to
+    /// `compare_mode` keep their behavior). ue-r2-1c: single home for
+    /// a translation that was previously copy-pasted at three sites
+    /// (streaming, tuning query, single-file), which had already
+    /// diverged once (R58-F7/R58-followup).
+    pub fn resolve_comparison_mode(
+        self,
+        legacy_checksum: bool,
+    ) -> crate::generated::ComparisonMode {
+        use crate::generated::ComparisonMode;
+        match self {
+            LocalCompareMode::Checksum => ComparisonMode::Checksum,
+            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
+            LocalCompareMode::Force => ComparisonMode::Force,
+            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
+            LocalCompareMode::SizeMtime => {
+                if legacy_checksum {
+                    ComparisonMode::Checksum
+                } else {
+                    ComparisonMode::SizeMtime
+                }
+            }
+        }
+    }
+
+    /// Same resolution, onto the perf-history snapshot enum (tuning
+    /// buckets key on the full comparison policy -- R59 finding #5).
+    pub(crate) fn resolve_compare_snapshot(
+        self,
+        legacy_checksum: bool,
+    ) -> crate::perf_history::CompareModeSnapshot {
+        use crate::perf_history::CompareModeSnapshot;
+        match self {
+            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
+            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
+            LocalCompareMode::Force => CompareModeSnapshot::Force,
+            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
+            LocalCompareMode::SizeMtime => {
+                if legacy_checksum {
+                    CompareModeSnapshot::Checksum
+                } else {
+                    CompareModeSnapshot::SizeMtime
+                }
+            }
+        }
+    }
+}
+
 /// Options for executing a local mirror/copy operation.
 #[derive(Clone, Debug)]
 pub struct LocalMirrorOptions {

 exited 1 in 0ms:
--- /proc/self/fd/11	2026-07-03 12:21:25.830064644 -0400
+++ /proc/self/fd/12	2026-07-03 12:21:25.830064644 -0400
@@ -3,25 +3,15 @@
 };
 use crate::perf_predictor::PerformancePredictor;
 
-use super::{LocalCompareMode, LocalMirrorOptions, LocalMirrorSummary};
+use super::{LocalMirrorOptions, LocalMirrorSummary};
 
 /// Map the orchestrator's `LocalCompareMode` onto the perf-history
 /// snapshot enum so tuning records preserve the user's full intent
 /// (not just `checksum: bool`).
 fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
-    match options.compare_mode {
-        LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
-        LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
-        LocalCompareMode::Force => CompareModeSnapshot::Force,
-        LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
-        LocalCompareMode::SizeMtime => {
-            if options.checksum {
-                CompareModeSnapshot::Checksum
-            } else {
-                CompareModeSnapshot::SizeMtime
-            }
-        }
-    }
+    options
+        .compare_mode
+        .resolve_compare_snapshot(options.checksum)
 }
 
 pub(super) fn record_performance_history(

exec
/usr/bin/zsh -lc "diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | sed -n '45,110p') <(git show 29e210b:crates/blit-core/src/engine/tuning.rs | sed -n '8,93p')" in /home/michael/dev/Blit
 exited 1 in 0ms:
--- /proc/self/fd/11	2026-07-03 12:21:35.634077199 -0400
+++ /proc/self/fd/12	2026-07-03 12:21:35.634077199 -0400
@@ -1,4 +1,20 @@
-fn select_tuning_window(
+/// Maximum number of recent eligible records the local tuner looks
+/// at. The cap exists so a recent regime change (new disk, fresh
+/// install) propagates into tuning within ~20 transfers instead of
+/// being diluted by older history.
+const TUNING_WINDOW_SIZE: usize = 20;
+
+/// R56-F2: select the window of recent records that should feed
+/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
+/// FIRST, then the per-operation discriminants, THEN takes the
+/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
+/// run_kind filter, so 20 recent dry-run / null-sink records with
+/// matching mode could fill the window and force tuning to fall
+/// back to defaults even when older real records existed.
+///
+/// Extracted so the contract is unit-testable without touching
+/// the global perf-history JSONL.
+pub(super) fn select_tuning_window(
     history: &[crate::perf_history::PerformanceRecord],
     target_mode: TransferMode,
     compare_mode: crate::perf_history::CompareModeSnapshot,
@@ -47,7 +63,7 @@
 /// Returns `None` if the reader errored OR no eligible records
 /// were found; the caller treats either case as "fall back to
 /// defaults."
-fn select_tuning_window_from_history<F>(
+pub(super) fn select_tuning_window_from_history<F>(
     reader: F,
     target_mode: TransferMode,
     compare_mode: crate::perf_history::CompareModeSnapshot,
@@ -64,3 +80,7 @@
     let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
     if window.is_empty() {
         None
+    } else {
+        Some(window)
+    }
+}

exec
/usr/bin/zsh -lc "diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | sed -n '1138,1298p') <(git show 29e210b:crates/blit-core/src/engine/single_file.rs | sed -n '29,210p')" in /home/michael/dev/Blit
 exited 1 in 0ms:
--- /proc/self/fd/11	2026-07-03 12:21:35.634077199 -0400
+++ /proc/self/fd/12	2026-07-03 12:21:35.634077199 -0400
@@ -1,4 +1,35 @@
-fn execute_single_file_copy(
+pub(super) fn execute_single_file_copy(
+    src_root: &Path,
+    dest_root: &Path,
+    options: &LocalMirrorOptions,
+    start_time: Instant,
+) -> Result<LocalMirrorSummary> {
+    let summary = single_file_copy_inner(src_root, dest_root, options, start_time)?;
+
+    let fast_path_label = if options.null_sink {
+        "null_sink"
+    } else {
+        "single_file"
+    };
+    if let Some(record) = record_performance_history(
+        &summary,
+        options,
+        Some(fast_path_label),
+        0,
+        summary.duration.as_millis(),
+    ) {
+        if !options.null_sink {
+            let mut predictor = PerformancePredictor::load().ok();
+            update_predictor(&mut predictor, &record, options.verbose);
+        }
+    }
+
+    Ok(summary)
+}
+
+/// The copy itself, bypassing the enumerator/planner/pipeline
+/// machinery which assumes `src_root` is a directory.
+fn single_file_copy_inner(
     src_root: &Path,
     dest_root: &Path,
     options: &LocalMirrorOptions,
@@ -19,21 +50,11 @@
     // at `options.checksum`, so `--size-only` / `--ignore-times` /
     // `--force` were silently dropped — repro: copy src.txt dst.txt
     // --size-only re-copied even when sizes matched.
-    let compare_mode = match options.compare_mode {
-        crate::orchestrator::LocalCompareMode::Checksum => ComparisonMode::Checksum,
-        crate::orchestrator::LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
-        crate::orchestrator::LocalCompareMode::Force => ComparisonMode::Force,
-        crate::orchestrator::LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
-        crate::orchestrator::LocalCompareMode::SizeMtime => {
-            if options.checksum {
-                ComparisonMode::Checksum
-            } else {
-                ComparisonMode::SizeMtime
-            }
-        }
-    };
+    let compare_mode: ComparisonMode = options
+        .compare_mode
+        .resolve_comparison_mode(options.checksum);
 
-    // R58-F5: the single-file short-circuit (orchestrator.rs:125)
+    // R58-F5: the single-file strategy (engine dispatch)
     // bypasses the enumerator + planner, which is where the
     // streaming-pipeline path checks filter / ignore_existing.
     // Apply both here so single-file copies honor the same

exec
/usr/bin/zsh -lc "diff -u <(git show 63b813a:crates/blit-core/src/orchestrator/orchestrator.rs | sed -n '889,1126p') <(git show 29e210b:crates/blit-core/src/engine/mirror.rs)" in /home/michael/dev/Blit
 exited 1 in 0ms:
--- /proc/self/fd/11	2026-07-03 12:21:35.684077263 -0400
+++ /proc/self/fd/12	2026-07-03 12:21:35.684077263 -0400
@@ -1,4 +1,31 @@
-fn apply_mirror_deletions(
+//! Mirror-deletion pass for the engine's streaming strategy.
+//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
+
+use std::collections::HashSet;
+use std::path::Path;
+
+use eyre::{bail, Result};
+
+use crate::fs_enum::FileFilter;
+
+/// Delete destination files/dirs not present in the source header set.
+///
+/// R58-F6: `delete_scope` controls which destination entries are
+/// even considered for deletion:
+///   - `FilteredSubset` (default): enumerate the destination
+///     *through the user's filter*, then delete entries not in
+///     the source set. Excluded files (e.g. `*.log` when
+///     `--exclude '*.log'`) are out of scope — they're not
+///     candidates for deletion, and their parent directories are
+///     therefore non-empty from the user's perspective. When
+///     `remove_dir` fails with ENOTEMPTY on a parent whose only
+///     remaining contents are out-of-scope, we treat it as
+///     expected, not as an error.
+///   - `All`: enumerate the destination *without* the filter so
+///     every entry is in scope. ENOTEMPTY is a genuine error
+///     here (we did walk everything, so something other than
+///     filter-excluded content must be in the way).
+pub(super) fn apply_mirror_deletions(
     source_paths: &HashSet<String>,
     dest_root: &Path,
     filter: &FileFilter,
@@ -169,70 +196,3 @@
 
     Ok((deleted_files, deleted_dirs))
 }
-
-fn persist_journal_checkpoints(
-    tracker: &mut ChangeTracker,
-    tokens: &mut [ProbeToken],
-    verbose: bool,
-) {
-    if tokens.is_empty() {
-        return;
-    }
-
-    for token in tokens.iter_mut() {
-        match tracker.reprobe_canonical(&token.canonical_path) {
-            Ok(snapshot) => token.snapshot = snapshot,
-            Err(err) => {
-                token.snapshot = None;
-                if verbose {
-                    eprintln!(
-                        "Failed to refresh journal snapshot for {}: {err:?}",
-                        token.canonical_path.display()
-                    );
-                }
-            }
-        }
-    }
-
-    if let Err(err) = tracker.refresh_and_persist(tokens) {
-        if verbose {
-            eprintln!("Failed to update journal checkpoint: {err:?}");
-        }
-    }
-}
-
-fn log_probe(label: &str, probe: &ProbeToken) {
-    eprintln!(
-        "Journal probe {label} state={:?} snapshot={} path={}",
-        probe.state,
-        probe.snapshot.is_some(),
-        probe.canonical_path.display()
-    );
-
-    if let Some(snapshot) = &probe.snapshot {
-        match snapshot {
-            StoredSnapshot::Windows(snap) => {
-                eprintln!(
-                    "  {label} windows: volume={} journal_id={} next_usn={} mtime={:?}",
-                    snap.volume, snap.journal_id, snap.next_usn, snap.root_mtime_epoch_ms
-                );
-            }
-            StoredSnapshot::MacOs(snap) => {
-                eprintln!(
-                    "  {label} macOS: fsid={} event_id={}",
-                    snap.fsid, snap.event_id
-                );
-            }
-            StoredSnapshot::Linux(snap) => {
-                eprintln!(
-                    "  {label} linux: device={} inode={} ctime={}s+{}ns mtime={:?}",
-                    snap.device,
-                    snap.inode,
-                    snap.ctime_sec,
-                    snap.ctime_nsec,
-                    snap.root_mtime_epoch_ms
-                );
-            }
-        }
-    }
-}

exec
/usr/bin/zsh -lc 'git diff --color=never 63b813a 29e210b -- crates/blit-core/src/engine/journal.rs crates/blit-core/src/engine/mirror.rs crates/blit-core/src/engine/tuning.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/engine/journal.rs b/crates/blit-core/src/engine/journal.rs
new file mode 100644
index 0000000..2f745b3
--- /dev/null
+++ b/crates/blit-core/src/engine/journal.rs
@@ -0,0 +1,72 @@
+//! Change-journal checkpoint persistence + probe logging for the
+//! engine's `journal_no_work` strategy. Moved verbatim from
+//! `orchestrator/orchestrator.rs` at ue-r2-1c.
+
+use crate::change_journal::{ChangeTracker, ProbeToken, StoredSnapshot};
+
+pub(super) fn persist_journal_checkpoints(
+    tracker: &mut ChangeTracker,
+    tokens: &mut [ProbeToken],
+    verbose: bool,
+) {
+    if tokens.is_empty() {
+        return;
+    }
+
+    for token in tokens.iter_mut() {
+        match tracker.reprobe_canonical(&token.canonical_path) {
+            Ok(snapshot) => token.snapshot = snapshot,
+            Err(err) => {
+                token.snapshot = None;
+                if verbose {
+                    eprintln!(
+                        "Failed to refresh journal snapshot for {}: {err:?}",
+                        token.canonical_path.display()
+                    );
+                }
+            }
+        }
+    }
+
+    if let Err(err) = tracker.refresh_and_persist(tokens) {
+        if verbose {
+            eprintln!("Failed to update journal checkpoint: {err:?}");
+        }
+    }
+}
+
+pub(super) fn log_probe(label: &str, probe: &ProbeToken) {
+    eprintln!(
+        "Journal probe {label} state={:?} snapshot={} path={}",
+        probe.state,
+        probe.snapshot.is_some(),
+        probe.canonical_path.display()
+    );
+
+    if let Some(snapshot) = &probe.snapshot {
+        match snapshot {
+            StoredSnapshot::Windows(snap) => {
+                eprintln!(
+                    "  {label} windows: volume={} journal_id={} next_usn={} mtime={:?}",
+                    snap.volume, snap.journal_id, snap.next_usn, snap.root_mtime_epoch_ms
+                );
+            }
+            StoredSnapshot::MacOs(snap) => {
+                eprintln!(
+                    "  {label} macOS: fsid={} event_id={}",
+                    snap.fsid, snap.event_id
+                );
+            }
+            StoredSnapshot::Linux(snap) => {
+                eprintln!(
+                    "  {label} linux: device={} inode={} ctime={}s+{}ns mtime={:?}",
+                    snap.device,
+                    snap.inode,
+                    snap.ctime_sec,
+                    snap.ctime_nsec,
+                    snap.root_mtime_epoch_ms
+                );
+            }
+        }
+    }
+}
diff --git a/crates/blit-core/src/engine/mirror.rs b/crates/blit-core/src/engine/mirror.rs
new file mode 100644
index 0000000..e316b08
--- /dev/null
+++ b/crates/blit-core/src/engine/mirror.rs
@@ -0,0 +1,198 @@
+//! Mirror-deletion pass for the engine's streaming strategy.
+//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
+
+use std::collections::HashSet;
+use std::path::Path;
+
+use eyre::{bail, Result};
+
+use crate::fs_enum::FileFilter;
+
+/// Delete destination files/dirs not present in the source header set.
+///
+/// R58-F6: `delete_scope` controls which destination entries are
+/// even considered for deletion:
+///   - `FilteredSubset` (default): enumerate the destination
+///     *through the user's filter*, then delete entries not in
+///     the source set. Excluded files (e.g. `*.log` when
+///     `--exclude '*.log'`) are out of scope — they're not
+///     candidates for deletion, and their parent directories are
+///     therefore non-empty from the user's perspective. When
+///     `remove_dir` fails with ENOTEMPTY on a parent whose only
+///     remaining contents are out-of-scope, we treat it as
+///     expected, not as an error.
+///   - `All`: enumerate the destination *without* the filter so
+///     every entry is in scope. ENOTEMPTY is a genuine error
+///     here (we did walk everything, so something other than
+///     filter-excluded content must be in the way).
+pub(super) fn apply_mirror_deletions(
+    source_paths: &HashSet<String>,
+    dest_root: &Path,
+    filter: &FileFilter,
+    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
+    perform: bool,
+    verbose: bool,
+) -> Result<(usize, usize)> {
+    use crate::enumeration::{EntryKind, FileEnumerator};
+    use crate::orchestrator::LocalMirrorDeleteScope;
+
+    // R58-F6: FilteredSubset uses the user's filter for the
+    // enumeration (only in-scope entries become deletion
+    // candidates). All bypasses the filter so every destination
+    // entry is considered.
+    let enum_filter = match delete_scope {
+        LocalMirrorDeleteScope::FilteredSubset => filter.clone_without_cache(),
+        LocalMirrorDeleteScope::All => FileFilter::default(),
+    };
+    let enumerator = FileEnumerator::new(enum_filter);
+    let dest_entries = enumerator.enumerate_local(dest_root)?;
+
+    // R48-F1: source.scan() only emits file headers, so
+    // `source_paths` is a set of *files*. Pre-fix this meant every
+    // destination directory was "not in source_paths" and got
+    // queued for deletion. Combined with R46-F5's hard-error
+    // policy on remove_* failures, a normal mirror containing
+    // `sub/file.txt` would keep `sub/file.txt`, then try
+    // `remove_dir("sub")` and fail the whole operation with
+    // ENOTEMPTY. Derive `source_dirs` from each file's parent
+    // chain so dest dirs that exist implicitly on the source
+    // side (because they contain a source file) get preserved.
+    let mut source_dirs: HashSet<String> = HashSet::new();
+    for path in source_paths {
+        let p = std::path::Path::new(path);
+        let mut cur = p.parent();
+        while let Some(parent) = cur {
+            if parent.as_os_str().is_empty() {
+                break;
+            }
+            let parent_str = crate::path_posix::relative_path_to_posix(parent);
+            // Insert and keep walking up; if already present every
+            // shallower ancestor is too, so we could break — but
+            // the walk is cheap and the eager form is simpler to
+            // reason about.
+            source_dirs.insert(parent_str);
+            cur = parent.parent();
+        }
+    }
+
+    let mut files_to_delete = Vec::new();
+    let mut dirs_to_delete = Vec::new();
+
+    for entry in &dest_entries {
+        let rel = crate::path_posix::relative_path_to_posix(&entry.relative_path);
+        let absent_at_source = match entry.kind {
+            EntryKind::Directory => !source_dirs.contains(&rel),
+            _ => !source_paths.contains(&rel),
+        };
+        if absent_at_source {
+            let abs = dest_root.join(&entry.relative_path);
+            match entry.kind {
+                EntryKind::Directory => dirs_to_delete.push(abs),
+                _ => files_to_delete.push(abs),
+            }
+        }
+    }
+
+    // Sort dirs deepest-first so children are deleted before parents.
+    dirs_to_delete.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
+
+    let mut deleted_files = 0usize;
+    let mut deleted_dirs = 0usize;
+    // R46-F5: collect deletion failures and bail at the end. Pre-fix
+    // each `remove_file` / `remove_dir` error was printed as a
+    // warning and the function returned Ok, so a mirror could
+    // succeed-on-paper while leaving stale destination content
+    // behind. Now we still attempt every deletion (better partial
+    // progress than abort-on-first-failure), but we bail with an
+    // aggregated error if any failed — the caller's mirror operation
+    // returns Err, the user sees the failed entries, and the summary
+    // line doesn't claim "complete".
+    let mut failures: Vec<String> = Vec::new();
+
+    for path in files_to_delete {
+        #[cfg(windows)]
+        crate::win_fs::clear_readonly_recursive(&path);
+
+        if perform {
+            match std::fs::remove_file(&path) {
+                Ok(_) => {
+                    deleted_files += 1;
+                    if verbose {
+                        eprintln!("Deleted file: {}", path.display());
+                    }
+                }
+                Err(err) => {
+                    eprintln!("Failed to delete file {}: {}", path.display(), err);
+                    failures.push(format!("{}: {}", path.display(), err));
+                }
+            }
+        } else {
+            deleted_files += 1;
+        }
+    }
+
+    for path in dirs_to_delete {
+        #[cfg(windows)]
+        crate::win_fs::clear_readonly_recursive(&path);
+
+        if perform {
+            match std::fs::remove_dir(&path) {
+                Ok(_) => {
+                    deleted_dirs += 1;
+                    if verbose {
+                        eprintln!("Deleted directory: {}", path.display());
+                    }
+                }
+                Err(err) => {
+                    // R58-F6: in FilteredSubset mode, ENOTEMPTY on
+                    // a destination dir means the dir contains
+                    // out-of-scope content (files matching the
+                    // user's exclude rules). Those files
+                    // intentionally aren't candidates for
+                    // deletion, so the dir genuinely can't be
+                    // empty — that's not a failure, it's the
+                    // expected behavior of the scope contract.
+                    // Skip silently in that case; surface the
+                    // error in `All` mode where the dir really
+                    // should have been empty.
+                    let is_not_empty = err.kind() == std::io::ErrorKind::DirectoryNotEmpty
+                        || err.raw_os_error() == Some(66); // ENOTEMPTY on macOS/BSD
+                    if matches!(delete_scope, LocalMirrorDeleteScope::FilteredSubset)
+                        && is_not_empty
+                    {
+                        if verbose {
+                            eprintln!(
+                                "Kept directory {} (contains out-of-scope contents)",
+                                path.display()
+                            );
+                        }
+                    } else {
+                        eprintln!("Failed to delete directory {}: {}", path.display(), err);
+                        failures.push(format!("{}: {}", path.display(), err));
+                    }
+                }
+            }
+        } else {
+            deleted_dirs += 1;
+        }
+    }
+
+    if !failures.is_empty() {
+        let preview = failures
+            .iter()
+            .take(5)
+            .cloned()
+            .collect::<Vec<_>>()
+            .join("; ");
+        bail!(
+            "mirror-delete left {} entr{} in place at {} ({} succeeded): {}",
+            failures.len(),
+            if failures.len() == 1 { "y" } else { "ies" },
+            dest_root.display(),
+            deleted_files + deleted_dirs,
+            preview
+        );
+    }
+
+    Ok((deleted_files, deleted_dirs))
+}
diff --git a/crates/blit-core/src/engine/tuning.rs b/crates/blit-core/src/engine/tuning.rs
new file mode 100644
index 0000000..92c2cc8
--- /dev/null
+++ b/crates/blit-core/src/engine/tuning.rs
@@ -0,0 +1,592 @@
+//! History-derived plan tuning for the engine's streaming strategy.
+//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
+
+use eyre::Result;
+
+use crate::perf_history::TransferMode;
+
+/// Maximum number of recent eligible records the local tuner looks
+/// at. The cap exists so a recent regime change (new disk, fresh
+/// install) propagates into tuning within ~20 transfers instead of
+/// being diluted by older history.
+const TUNING_WINDOW_SIZE: usize = 20;
+
+/// R56-F2: select the window of recent records that should feed
+/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
+/// FIRST, then the per-operation discriminants, THEN takes the
+/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
+/// run_kind filter, so 20 recent dry-run / null-sink records with
+/// matching mode could fill the window and force tuning to fall
+/// back to defaults even when older real records existed.
+///
+/// Extracted so the contract is unit-testable without touching
+/// the global perf-history JSONL.
+pub(super) fn select_tuning_window(
+    history: &[crate::perf_history::PerformanceRecord],
+    target_mode: TransferMode,
+    compare_mode: crate::perf_history::CompareModeSnapshot,
+    skip_unchanged: bool,
+) -> Vec<crate::perf_history::PerformanceRecord> {
+    history
+        .iter()
+        .rev()
+        .filter(|record| record.run_kind.is_real_transfer())
+        .filter(|record| record.mode == target_mode)
+        // R59 finding #5: key on the full comparison policy
+        // (not just `checksum: bool`) so SizeMtime / SizeOnly /
+        // Force / IgnoreTimes runs don't mix into the same tuning
+        // bucket. Pre-fix a session of `--size-only` runs trained
+        // the SizeMtime bucket (and vice versa).
+        .filter(|record| record.options.compare_mode == compare_mode)
+        .filter(|record| record.options.skip_unchanged == skip_unchanged)
+        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
+        // R58-followup: require a tuning signal. `derive_local_plan_tuning`
+        // only aggregates `tar_shard_*` + `raw_bundle_*`; records with
+        // `tar_shard_tasks == 0 && raw_bundle_tasks == 0` (no_work,
+        // journal_no_work, single_huge_file, streaming no-ops) are
+        // RunKind::Real and pass every other gate but contribute
+        // nothing. Pre-fix they could fill the 20-slot window and
+        // hide older bucket-bearing records. If the tuner ever
+        // starts consuming `large_tasks`, add it here too.
+        .filter(|record| record.tar_shard_tasks > 0 || record.raw_bundle_tasks > 0)
+        .take(TUNING_WINDOW_SIZE)
+        .cloned()
+        .collect()
+}
+
+/// R57-F1: wrapper that always reads the FULL history before
+/// applying the run_kind filter. The caller used to pass
+/// `read_recent_records(50)`, which pre-capped the input slice
+/// at 50 records — so 50 recent non-real records could hide
+/// older real records before `select_tuning_window` ever saw
+/// them. Baking the "ask for all records" invariant into the
+/// wrapper means the limit can't drift back to a finite value.
+/// The history file is already size-capped at ~1 MiB upstream
+/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
+/// records is bounded.
+///
+/// Generic over the reader so unit tests can inject a synthetic
+/// history; production passes `read_recent_records` directly.
+/// Returns `None` if the reader errored OR no eligible records
+/// were found; the caller treats either case as "fall back to
+/// defaults."
+pub(super) fn select_tuning_window_from_history<F>(
+    reader: F,
+    target_mode: TransferMode,
+    compare_mode: crate::perf_history::CompareModeSnapshot,
+    skip_unchanged: bool,
+) -> Option<Vec<crate::perf_history::PerformanceRecord>>
+where
+    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
+{
+    // `0` means "all records" per read_recent_records' contract
+    // (see read_records_from_path in perf_history.rs:298). This
+    // is the load-bearing literal — passing anything else
+    // reintroduces R57-F1.
+    let history = reader(0).ok()?;
+    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
+    if window.is_empty() {
+        None
+    } else {
+        Some(window)
+    }
+}
+
+#[cfg(test)]
+mod select_tuning_window_tests {
+    //! R56-F2: ensure non-real records are filtered BEFORE the
+    //! 20-record window, not after. Pre-fix, recent
+    //! dry-run/null-sink records with matching mode could fill the
+    //! window and force tuning to fall back to defaults even when
+    //! older real records existed.
+
+    use super::*;
+    use crate::auto_tune::derive_local_plan_tuning;
+    use crate::perf_history::{
+        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
+    };
+    use eyre::eyre;
+
+    fn record(
+        kind: RunKind,
+        mode: TransferMode,
+        tar_tasks: u32,
+        tar_bytes: u64,
+        timestamp_ms: u128,
+    ) -> PerformanceRecord {
+        let mut r = PerformanceRecord::new(
+            mode,
+            None,
+            None,
+            10,
+            1024,
+            OptionSnapshot {
+                dry_run: false,
+                preserve_symlinks: true,
+                include_symlinks: false,
+                skip_unchanged: true,
+                checksum: false,
+                compare_mode: CompareModeSnapshot::SizeMtime,
+                workers: 4,
+            },
+            None,
+            10,
+            100,
+            0,
+            0,
+        );
+        r.run_kind = kind;
+        r.tar_shard_tasks = tar_tasks;
+        r.tar_shard_files = tar_tasks * 100;
+        r.tar_shard_bytes = tar_bytes;
+        r.timestamp_epoch_ms = timestamp_ms;
+        r
+    }
+
+    /// 30 recent NullSink records (matching the target operation
+    /// shape) followed by 5 older Real records. Pre-fix .take(20)
+    /// ran first, grabbed 20 NullSinks, derive_local_plan_tuning
+    /// skipped them all internally and returned None — tuning
+    /// fell back to defaults despite real history being available.
+    /// Post-fix, the filter eats the NullSinks before the take, so
+    /// the 5 Real records make it through and tuning succeeds.
+    #[test]
+    fn null_sink_records_do_not_crowd_out_older_real_records() {
+        let mut history = Vec::new();
+        // Older real records (timestamps lowest = oldest).
+        for i in 0..5 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Copy,
+                4,
+                16 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        // Recent null-sink records (higher timestamps = more recent).
+        for i in 0..30 {
+            history.push(record(
+                RunKind::NullSink,
+                TransferMode::Copy,
+                4,
+                512 * 1024 * 1024,
+                10_000 + i,
+            ));
+        }
+
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert!(
+            !window.is_empty(),
+            "real records must reach the window; 30 NullSink records crowded them out pre-R56-F2"
+        );
+        assert!(
+            window.iter().all(|r| r.run_kind.is_real_transfer()),
+            "only Real records should land in the tuning window"
+        );
+        // derive_local_plan_tuning succeeds → tuner sees its 5 Real
+        // records with 16 MiB tar bytes / 4 tar tasks = 4 MiB avg
+        // (clamped to the 4 MiB floor).
+        let tuning = derive_local_plan_tuning(&window).expect("tuning must succeed");
+        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
+        assert!(tuning.small_target_bytes <= 16 * 1024 * 1024);
+    }
+
+    #[test]
+    fn dry_run_records_do_not_crowd_out_real_records() {
+        let mut history = Vec::new();
+        for i in 0..3 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Copy,
+                2,
+                8 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        for i in 0..25 {
+            history.push(record(
+                RunKind::DryRun,
+                TransferMode::Copy,
+                10,
+                1024 * 1024 * 1024,
+                10_000 + i,
+            ));
+        }
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(
+            window.len(),
+            3,
+            "expected the 3 real records, got {} entries",
+            window.len()
+        );
+        assert!(derive_local_plan_tuning(&window).is_some());
+    }
+
+    #[test]
+    fn bench_records_do_not_crowd_out_real_records() {
+        let mut history = Vec::new();
+        for i in 0..2 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Copy,
+                1,
+                4 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        for i in 0..50 {
+            history.push(record(
+                RunKind::BenchTransfer,
+                TransferMode::Copy,
+                100,
+                512 * 1024 * 1024,
+                10_000 + i,
+            ));
+        }
+        for i in 0..50 {
+            history.push(record(
+                RunKind::BenchWire,
+                TransferMode::Copy,
+                100,
+                512 * 1024 * 1024,
+                20_000 + i,
+            ));
+        }
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(window.len(), 2);
+        assert!(window.iter().all(|r| r.run_kind == RunKind::Real));
+    }
+
+    /// Sanity: with abundant real records, the window caps at 20.
+    #[test]
+    fn window_caps_at_20_real_records() {
+        let history: Vec<_> = (0..50)
+            .map(|i| {
+                record(
+                    RunKind::Real,
+                    TransferMode::Copy,
+                    2,
+                    8 * 1024 * 1024,
+                    100 + i,
+                )
+            })
+            .collect();
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(window.len(), 20, "expected the 20 most recent real records");
+    }
+
+    /// R57-F1 regression: the call site is now
+    /// `select_tuning_window_from_history` which bakes the
+    /// "ask for all records" invariant into the wrapper — see
+    /// the dedicated tests below for the synthetic-reader
+    /// regression that catches a future drift back to a finite
+    /// limit. The pure-helper test below verifies that the
+    /// in-function logic copes with arbitrarily large histories
+    /// even if the wrapper were bypassed.
+    #[test]
+    fn handles_large_history_with_non_real_records_at_the_front() {
+        let mut history = Vec::new();
+        // 200 recent NullSink records (would have fit inside the
+        // old 50-record pre-cap with room to spare).
+        for i in 0..200 {
+            history.push(record(
+                RunKind::NullSink,
+                TransferMode::Copy,
+                4,
+                512 * 1024 * 1024,
+                10_000 + i,
+            ));
+        }
+        // 5 older Real records (would never have been seen with
+        // pre-cap=50, since the 200 NullSinks alone exceed it).
+        for i in 0..5 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Copy,
+                4,
+                16 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        // Real records were appended last (highest timestamps);
+        // select_tuning_window iterates .rev() so they come first.
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(
+            window.len(),
+            5,
+            "expected the 5 real records to survive a flood of non-real history"
+        );
+        assert!(window.iter().all(|r| r.run_kind.is_real_transfer()));
+        assert!(derive_local_plan_tuning(&window).is_some());
+    }
+
+    /// R58-followup: Real records with no tuning signal
+    /// (`tar_shard_tasks == 0 && raw_bundle_tasks == 0`) must not
+    /// crowd out older bucket-bearing records. These exist when a
+    /// run took the no_work / journal_no_work / single_huge_file
+    /// fast-path or was a streaming run that copied nothing — they
+    /// pass `is_real_transfer`, pass the per-operation discriminants,
+    /// pass the !=tiny_manifest gate, but contribute zero to
+    /// `derive_local_plan_tuning`. Pre-fix the 20-record window
+    /// could fill with them and the tuner fell back to defaults.
+    #[test]
+    fn no_signal_real_records_do_not_crowd_out_bucket_bearing_records() {
+        let mut history = Vec::new();
+        // 5 older Real records WITH bucket signal (timestamps lowest).
+        for i in 0..5 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Copy,
+                4,
+                16 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        // 30 recent Real records WITHOUT bucket signal: tar_tasks=0,
+        // bytes=0 — same shape `single_huge_file` / `no_work` /
+        // `journal_no_work` / streaming-no-op records produce.
+        for i in 0..30 {
+            let mut r = record(RunKind::Real, TransferMode::Copy, 0, 0, 10_000 + i);
+            // Vary fast_path across the no-signal categories to
+            // mirror real history. None of these exclude the record
+            // from the existing gates.
+            r.fast_path = match i % 4 {
+                0 => Some("no_work".to_string()),
+                1 => Some("journal_no_work".to_string()),
+                2 => Some("single_huge_file".to_string()),
+                _ => None,
+            };
+            history.push(r);
+        }
+
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert!(
+            !window.is_empty(),
+            "older bucket-bearing records must reach the window; \
+             30 no-signal Real records crowded them out pre-fix"
+        );
+        assert!(
+            window
+                .iter()
+                .all(|r| r.tar_shard_tasks > 0 || r.raw_bundle_tasks > 0),
+            "every record in the window must carry a tuning signal"
+        );
+        assert!(
+            derive_local_plan_tuning(&window).is_some(),
+            "tuner must return a value, not fall back to defaults"
+        );
+    }
+
+    // ── R57-F1: wrapper's "ask for all records" invariant ────────────
+    //
+    // The bug class isn't about what `select_tuning_window` does
+    // with a slice; it's about which slice the caller passes in.
+    // `select_tuning_window_from_history` wraps the reader call so
+    // a future maintainer can't drift the limit back to a finite
+    // value. These tests catch that drift by asserting on the
+    // limit value the wrapper passes to its reader.
+
+    use std::cell::Cell;
+    use std::rc::Rc;
+
+    /// Captures the `limit` argument every call to the reader.
+    /// The reader returns a fixed slice; we just want to see what
+    /// the wrapper asks for.
+    fn recording_reader(
+        captured_limit: Rc<Cell<Option<usize>>>,
+        records: Vec<PerformanceRecord>,
+    ) -> impl FnOnce(usize) -> Result<Vec<PerformanceRecord>> {
+        move |limit| {
+            captured_limit.set(Some(limit));
+            Ok(records)
+        }
+    }
+
+    #[test]
+    fn wrapper_passes_zero_to_reader() {
+        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
+        let reader = recording_reader(captured.clone(), vec![]);
+        let _ = select_tuning_window_from_history(
+            reader,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(
+            captured.get(),
+            Some(0),
+            "R57-F1: the wrapper must ask for all records (limit=0); any \
+             finite limit reintroduces the JSONL-layer crowd-out bug"
+        );
+    }
+
+    #[test]
+    fn wrapper_returns_none_when_reader_errors() {
+        let reader = |_limit: usize| -> Result<Vec<PerformanceRecord>> {
+            Err(eyre!("simulated read failure"))
+        };
+        let result = select_tuning_window_from_history(
+            reader,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert!(result.is_none());
+    }
+
+    #[test]
+    fn wrapper_returns_none_when_no_eligible_records() {
+        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
+        let reader = recording_reader(
+            captured,
+            vec![
+                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 100),
+                record(RunKind::NullSink, TransferMode::Copy, 4, 1024 * 1024, 200),
+            ],
+        );
+        let result = select_tuning_window_from_history(
+            reader,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert!(result.is_none());
+    }
+
+    #[test]
+    fn wrapper_returns_some_window_when_real_records_present() {
+        let captured: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
+        let reader = recording_reader(
+            captured.clone(),
+            vec![
+                record(RunKind::Real, TransferMode::Copy, 4, 16 * 1024 * 1024, 100),
+                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 200),
+            ],
+        );
+        let result = select_tuning_window_from_history(
+            reader,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        )
+        .unwrap();
+        assert_eq!(result.len(), 1);
+        assert_eq!(result[0].run_kind, RunKind::Real);
+        assert_eq!(captured.get(), Some(0));
+    }
+
+    /// Sanity: mode and option filters still apply post-R56-F2.
+    /// A Real record with the wrong mode/checksum/skip_unchanged
+    /// must NOT land in the window.
+    #[test]
+    fn mode_and_option_filters_still_apply() {
+        let mut history = Vec::new();
+        // Real Mirror records (wrong mode).
+        for i in 0..10 {
+            history.push(record(
+                RunKind::Real,
+                TransferMode::Mirror,
+                4,
+                16 * 1024 * 1024,
+                100 + i,
+            ));
+        }
+        // Real Copy record.
+        history.push(record(
+            RunKind::Real,
+            TransferMode::Copy,
+            2,
+            8 * 1024 * 1024,
+            500,
+        ));
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(window.len(), 1);
+        assert_eq!(window[0].mode, TransferMode::Copy);
+    }
+
+    /// R59 finding #5: SizeOnly / Force / IgnoreTimes runs must
+    /// not contaminate the SizeMtime tuning bucket. Pre-fix the
+    /// window filtered on `options.checksum == checksum_bool`, so a
+    /// `--size-only` run (checksum=false) landed in the same bucket
+    /// as a default `SizeMtime` run.
+    #[test]
+    fn compare_mode_buckets_are_separate() {
+        let mut history = Vec::new();
+        // 10 SizeOnly Real records (signal-bearing).
+        for i in 0..10 {
+            let mut r = record(
+                RunKind::Real,
+                TransferMode::Copy,
+                4,
+                16 * 1024 * 1024,
+                100 + i,
+            );
+            r.options.compare_mode = CompareModeSnapshot::SizeOnly;
+            history.push(r);
+        }
+        // One SizeMtime Real record.
+        let mut sm = record(RunKind::Real, TransferMode::Copy, 2, 8 * 1024 * 1024, 500);
+        sm.options.compare_mode = CompareModeSnapshot::SizeMtime;
+        history.push(sm);
+
+        // Querying SizeMtime must NOT pick up the 10 SizeOnly records.
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeMtime,
+            true,
+        );
+        assert_eq!(window.len(), 1);
+        assert_eq!(
+            window[0].options.compare_mode,
+            CompareModeSnapshot::SizeMtime
+        );
+
+        // Querying SizeOnly returns the SizeOnly records.
+        let window = select_tuning_window(
+            &history,
+            TransferMode::Copy,
+            CompareModeSnapshot::SizeOnly,
+            true,
+        );
+        assert_eq!(window.len(), 10);
+        assert!(window
+            .iter()
+            .all(|r| r.options.compare_mode == CompareModeSnapshot::SizeOnly));
+    }
+}

exec
/usr/bin/zsh -lc 'git diff --color=never 63b813a 29e210b -- crates/blit-core/src/local_worker.rs crates/blit-daemon/src/service/pull.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/local_worker.rs b/crates/blit-core/src/local_worker.rs
index 5f631d8..bb91e83 100644
--- a/crates/blit-core/src/local_worker.rs
+++ b/crates/blit-core/src/local_worker.rs
@@ -1,7 +1,7 @@
 //! Local filesystem copy helpers used by the orchestrator's fast-path.
 //!
 //! The main transfer pipeline now uses `FsTransferSink` (via `execute_sink_pipeline`).
-//! These blocking helpers remain for the fast-path routing in `orchestrator::fast_path`
+//! These blocking helpers remain for the fast-path strategies in `engine::strategy`
 //! which handles tiny manifests and single huge files without the full pipeline overhead.
 
 use std::path::{Path, PathBuf};
diff --git a/crates/blit-daemon/src/service/pull.rs b/crates/blit-daemon/src/service/pull.rs
index 01877c8..a41a205 100644
--- a/crates/blit-daemon/src/service/pull.rs
+++ b/crates/blit-daemon/src/service/pull.rs
@@ -421,7 +421,7 @@ pub(crate) async fn collect_pull_entries_with_checksums(
         // too. Pre-fix the daemon returned the entry unconditionally,
         // so `blit pull host:module/file.txt --exclude '*.txt'` still
         // pulled the file even though the local single-file path
-        // (orchestrator.rs:1132) correctly skipped on the same flag.
+        // (engine/single_file.rs) correctly skipped on the same flag.
         // Filter against the basename (matches what allows_entry
         // does for directory enumeration of leaf files).
         let name = physical.file_name().map(PathBuf::from);
@@ -977,7 +977,7 @@ fn payload_bytes(payload: &TransferPayload) -> u64 {
 mod single_file_filter_tests {
     //! R59 finding #4: the daemon pull single-file fast path
     //! returned the entry unconditionally, ignoring the user-supplied
-    //! filter. Local single-file copy (orchestrator.rs:1132) already
+    //! filter. Local single-file copy (engine/single_file.rs) already
     //! honored the filter, so the two paths drifted apart.
 
     use super::*;

exec
/usr/bin/zsh -lc "rg -n \"blit_core::orchestrator|crate::orchestrator|orchestrator::\\{|LocalMirrorOptions|LocalCompareMode|LocalMirrorSummary|TransferOutcome|TransferOrchestrator\" crates/blit-app crates/blit-cli crates/blit-tui crates/blit-daemon crates/blit-core/tests crates/blit-core/src --glob '"'!target'"'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/local_transfers.rs:2:use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
crates/blit-core/tests/local_transfers.rs:55:    let options = LocalMirrorOptions {
crates/blit-core/tests/local_transfers.rs:61:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/local_transfers.rs:74:/// `TransferOutcome::UpToDate`, and records the `no_work` perf-history
crates/blit-core/tests/local_transfers.rs:78:    use blit_core::orchestrator::TransferOutcome;
crates/blit-core/tests/local_transfers.rs:92:    let options = || LocalMirrorOptions {
crates/blit-core/tests/local_transfers.rs:101:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/local_transfers.rs:107:    assert_eq!(second.outcome, TransferOutcome::UpToDate);
crates/blit-core/tests/local_transfers.rs:121:/// `TransferOutcome::SourceEmpty`. Previously untested.
crates/blit-core/tests/local_transfers.rs:124:    use blit_core::orchestrator::TransferOutcome;
crates/blit-core/tests/local_transfers.rs:136:    let options = LocalMirrorOptions {
crates/blit-core/tests/local_transfers.rs:142:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/local_transfers.rs:145:    assert_eq!(summary.outcome, TransferOutcome::SourceEmpty);
crates/blit-core/tests/local_transfers.rs:169:    let options = LocalMirrorOptions {
crates/blit-core/tests/local_transfers.rs:175:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/local_transfers.rs:207:    let options = LocalMirrorOptions {
crates/blit-core/tests/local_transfers.rs:213:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/predictor_streaming.rs:7:use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
crates/blit-core/tests/predictor_streaming.rs:87:    let options = LocalMirrorOptions {
crates/blit-core/tests/predictor_streaming.rs:92:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/tests/predictor_streaming.rs:114:    let options = LocalMirrorOptions {
crates/blit-core/tests/predictor_streaming.rs:119:    let orchestrator = TransferOrchestrator::new();
crates/blit-core/src/engine/single_file.rs:15:use super::options::LocalMirrorOptions;
crates/blit-core/src/engine/single_file.rs:16:use super::summary::{LocalMirrorSummary, TransferOutcome};
crates/blit-core/src/engine/single_file.rs:32:    options: &LocalMirrorOptions,
crates/blit-core/src/engine/single_file.rs:34:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:63:    options: &LocalMirrorOptions,
crates/blit-core/src/engine/single_file.rs:65:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:107:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:114:            outcome: TransferOutcome::UpToDate,
crates/blit-core/src/engine/single_file.rs:123:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:130:            outcome: TransferOutcome::UpToDate,
crates/blit-core/src/engine/single_file.rs:136:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:149:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:193:    Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:204:            TransferOutcome::Transferred
crates/blit-core/src/engine/single_file.rs:206:            TransferOutcome::UpToDate
crates/blit-core/src/engine/mod.rs:9://! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
crates/blit-core/src/engine/mod.rs:27:pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
crates/blit-core/src/engine/mod.rs:28:pub use summary::{LocalMirrorSummary, TransferOutcome};
crates/blit-core/src/engine/mod.rs:70:    pub options: LocalMirrorOptions,
crates/blit-core/src/engine/mod.rs:86:    /// `TransferOrchestrator::execute_local_mirror_async` at
crates/blit-core/src/engine/mod.rs:88:    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/mod.rs:200:            let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:203:                outcome: TransferOutcome::JournalSkip,
crates/blit-core/src/engine/mod.rs:236:                        TransferOutcome::SourceEmpty
crates/blit-core/src/engine/mod.rs:238:                        TransferOutcome::UpToDate
crates/blit-core/src/engine/mod.rs:242:                            TransferOutcome::SourceEmpty => {
crates/blit-core/src/engine/mod.rs:255:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:290:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:324:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:479:        // NullSink -- see TransferOrchestrator).
crates/blit-core/src/engine/mod.rs:511:        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
crates/blit-core/src/engine/mod.rs:618:        let mut summary = LocalMirrorSummary {
crates/blit-core/src/engine/mirror.rs:32:    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
crates/blit-core/src/engine/mirror.rs:37:    use crate::orchestrator::LocalMirrorDeleteScope;
crates/blit-app/src/transfers/local.rs:6://! /local.rs`. Everything else (clap-arg → `LocalMirrorOptions`
crates/blit-app/src/transfers/local.rs:14://! `LocalMirrorOptions` struct from blit-core. The TUI's future
crates/blit-app/src/transfers/local.rs:15://! local-transfer trigger will build its own `LocalMirrorOptions`
crates/blit-app/src/transfers/local.rs:19:use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
crates/blit-app/src/transfers/local.rs:23:pub use blit_core::orchestrator::TransferOutcome;
crates/blit-app/src/transfers/local.rs:39:    options: LocalMirrorOptions,
crates/blit-app/src/transfers/local.rs:40:) -> Result<LocalMirrorSummary> {
crates/blit-app/src/transfers/local.rs:45:        TransferOrchestrator::new()
crates/blit-core/src/engine/strategy.rs:9:use super::LocalMirrorOptions;
crates/blit-core/src/engine/strategy.rs:37:    /// fast-path scan. Propagated into `LocalMirrorSummary.
crates/blit-core/src/engine/strategy.rs:79:    options: &LocalMirrorOptions,
crates/blit-core/src/engine/strategy.rs:92:    if !matches!(options.compare_mode, super::LocalCompareMode::SizeMtime) {
crates/blit-core/src/engine/strategy.rs:223:        let options = LocalMirrorOptions {
crates/blit-core/src/engine/strategy.rs:246:        let options = LocalMirrorOptions {
crates/blit-core/src/engine/strategy.rs:269:        let options = LocalMirrorOptions {
crates/blit-core/src/engine/history.rs:6:use super::{LocalMirrorOptions, LocalMirrorSummary};
crates/blit-core/src/engine/history.rs:8:/// Map the orchestrator's `LocalCompareMode` onto the perf-history
crates/blit-core/src/engine/history.rs:11:fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
crates/blit-core/src/engine/history.rs:18:    summary: &LocalMirrorSummary,
crates/blit-core/src/engine/history.rs:19:    options: &LocalMirrorOptions,
crates/blit-core/src/engine/history.rs:50:    summary: &LocalMirrorSummary,
crates/blit-core/src/engine/history.rs:51:    options: &LocalMirrorOptions,
crates/blit-core/src/engine/history.rs:130:    use crate::orchestrator::TransferOutcome;
crates/blit-core/src/engine/history.rs:133:    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
crates/blit-core/src/engine/history.rs:134:        LocalMirrorOptions {
crates/blit-core/src/engine/history.rs:136:            ..LocalMirrorOptions::default()
crates/blit-core/src/engine/history.rs:150:        let summary = LocalMirrorSummary {
crates/blit-core/src/engine/history.rs:159:            outcome: TransferOutcome::Transferred,
crates/blit-core/src/engine/history.rs:160:            ..LocalMirrorSummary::default()
crates/blit-core/src/engine/history.rs:183:        let summary = LocalMirrorSummary {
crates/blit-core/src/engine/history.rs:196:            ..LocalMirrorSummary::default()
crates/blit-core/src/engine/summary.rs:11:pub enum TransferOutcome {
crates/blit-core/src/engine/summary.rs:75:pub struct LocalMirrorSummary {
crates/blit-core/src/engine/summary.rs:103:    pub outcome: TransferOutcome,
crates/blit-cli/src/transfers/local.rs:4:use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
crates/blit-cli/src/transfers/local.rs:21:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:36:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:50:    summary: &LocalMirrorSummary,
crates/blit-cli/src/transfers/local.rs:80:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:142:) -> Result<LocalMirrorOptions> {
crates/blit-cli/src/transfers/local.rs:143:    use blit_core::orchestrator::{LocalCompareMode, LocalMirrorDeleteScope};
crates/blit-cli/src/transfers/local.rs:146:    // LocalCompareMode enum. The orchestrator then routes to the
crates/blit-cli/src/transfers/local.rs:156:        LocalCompareMode::IgnoreTimes
crates/blit-cli/src/transfers/local.rs:158:        LocalCompareMode::Force
crates/blit-cli/src/transfers/local.rs:160:        LocalCompareMode::SizeOnly
crates/blit-cli/src/transfers/local.rs:162:        LocalCompareMode::Checksum
crates/blit-cli/src/transfers/local.rs:164:        LocalCompareMode::SizeMtime
crates/blit-cli/src/transfers/local.rs:169:    // and `all`. Pre-fix LocalMirrorOptions had no field for
crates/blit-cli/src/transfers/local.rs:179:    let mut options = LocalMirrorOptions {
crates/blit-cli/src/transfers/local.rs:192:        ..LocalMirrorOptions::default()
crates/blit-cli/src/transfers/local.rs:214:    summary: &LocalMirrorSummary,
crates/blit-cli/src/transfers/local.rs:235:        TransferOutcome::JournalSkip => {
crates/blit-cli/src/transfers/local.rs:242:        TransferOutcome::UpToDate => {
crates/blit-cli/src/transfers/local.rs:249:        TransferOutcome::SourceEmpty => {
crates/blit-cli/src/transfers/local.rs:256:        TransferOutcome::Transferred => {}
crates/blit-cli/src/transfers/local.rs:320:    summary: &LocalMirrorSummary,
crates/blit-cli/src/transfers/local.rs:332:        TransferOutcome::Transferred => "transferred",
crates/blit-cli/src/transfers/local.rs:333:        TransferOutcome::JournalSkip => "journal_skip",
crates/blit-cli/src/transfers/local.rs:334:        TransferOutcome::UpToDate => "up_to_date",
crates/blit-cli/src/transfers/local.rs:335:        TransferOutcome::SourceEmpty => "source_empty",
crates/blit-core/src/engine/options.rs:26:pub enum LocalCompareMode {
crates/blit-core/src/engine/options.rs:42:impl LocalCompareMode {
crates/blit-core/src/engine/options.rs:56:            LocalCompareMode::Checksum => ComparisonMode::Checksum,
crates/blit-core/src/engine/options.rs:57:            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
crates/blit-core/src/engine/options.rs:58:            LocalCompareMode::Force => ComparisonMode::Force,
crates/blit-core/src/engine/options.rs:59:            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
crates/blit-core/src/engine/options.rs:60:            LocalCompareMode::SizeMtime => {
crates/blit-core/src/engine/options.rs:78:            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
crates/blit-core/src/engine/options.rs:79:            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
crates/blit-core/src/engine/options.rs:80:            LocalCompareMode::Force => CompareModeSnapshot::Force,
crates/blit-core/src/engine/options.rs:81:            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
crates/blit-core/src/engine/options.rs:82:            LocalCompareMode::SizeMtime => {
crates/blit-core/src/engine/options.rs:95:pub struct LocalMirrorOptions {
crates/blit-core/src/engine/options.rs:117:    pub compare_mode: LocalCompareMode,
crates/blit-core/src/engine/options.rs:133:impl Default for LocalMirrorOptions {
crates/blit-core/src/engine/options.rs:148:            compare_mode: LocalCompareMode::default(),
crates/blit-tui/src/main.rs:3080:    result: Result<blit_core::orchestrator::LocalMirrorSummary, String>,
crates/blit-tui/src/main.rs:4101:        let options = blit_core::orchestrator::LocalMirrorOptions {
crates/blit-tui/src/main.rs:4132:/// Surfaces the `LocalMirrorSummary` on success (so the
crates/blit-tui/src/main.rs:4161:) -> Result<blit_core::orchestrator::LocalMirrorSummary, String> {
crates/blit-tui/src/main.rs:4163:    let options = blit_core::orchestrator::LocalMirrorOptions {
crates/blit-cli/src/transfers/mod.rs:379:    // currently plumbed through `LocalMirrorOptions` /
crates/blit-tui/src/transfer.rs:23:use blit_core::orchestrator::LocalMirrorSummary;
crates/blit-tui/src/transfer.rs:73:        summary: Box<LocalMirrorSummary>,
crates/blit-tui/src/transfer.rs:219:        summary: LocalMirrorSummary,
crates/blit-tui/src/transfer.rs:257:    fn empty_summary() -> LocalMirrorSummary {
crates/blit-tui/src/transfer.rs:258:        LocalMirrorSummary::default()
crates/blit-core/src/orchestrator/mod.rs:4:    LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions, LocalMirrorSummary,
crates/blit-core/src/orchestrator/mod.rs:5:    TransferOutcome,
crates/blit-core/src/orchestrator/mod.rs:7:pub use orchestrator::TransferOrchestrator;
crates/blit-core/src/orchestrator/orchestrator.rs:11:use super::{LocalMirrorOptions, LocalMirrorSummary};
crates/blit-core/src/orchestrator/orchestrator.rs:21:pub struct TransferOrchestrator;
crates/blit-core/src/orchestrator/orchestrator.rs:23:impl TransferOrchestrator {
crates/blit-core/src/orchestrator/orchestrator.rs:41:        options: LocalMirrorOptions,
crates/blit-core/src/orchestrator/orchestrator.rs:42:    ) -> Result<LocalMirrorSummary> {
crates/blit-core/src/orchestrator/orchestrator.rs:59:        options: LocalMirrorOptions,
crates/blit-core/src/orchestrator/orchestrator.rs:60:    ) -> Result<LocalMirrorSummary> {
crates/blit-core/src/orchestrator/orchestrator.rs:127:impl Default for TransferOrchestrator {
crates/blit-core/src/orchestrator/orchestrator.rs:150:    fn opts() -> LocalMirrorOptions {
crates/blit-core/src/orchestrator/orchestrator.rs:151:        LocalMirrorOptions {
crates/blit-core/src/orchestrator/orchestrator.rs:169:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:190:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:229:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:321:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:389:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:425:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:463:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:494:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:522:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:557:        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::FilteredSubset;
crates/blit-core/src/orchestrator/orchestrator.rs:560:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:596:        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::All;
crates/blit-core/src/orchestrator/orchestrator.rs:599:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:643:        opts.compare_mode = crate::orchestrator::LocalCompareMode::SizeOnly;
crates/blit-core/src/orchestrator/orchestrator.rs:645:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:676:        opts.compare_mode = crate::orchestrator::LocalCompareMode::Force;
crates/blit-core/src/orchestrator/orchestrator.rs:678:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:705:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:744:        opts.compare_mode = crate::orchestrator::LocalCompareMode::SizeOnly;
crates/blit-core/src/orchestrator/orchestrator.rs:746:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:786:        opts.compare_mode = crate::orchestrator::LocalCompareMode::Force;
crates/blit-core/src/orchestrator/orchestrator.rs:788:        let orch = TransferOrchestrator::new();

exec
/usr/bin/zsh -lc "rg -n \"pub mod engine|mod engine|pub use crate::engine|pub use options|pub use summary|pub struct EngineRequest|pub struct TransferEngine|pub\\(super\\)|pub\\(crate\\)\" crates/blit-core/src/engine crates/blit-core/src/lib.rs crates/blit-core/src/orchestrator/mod.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/orchestrator/mod.rs:3:pub use crate::engine::{
crates/blit-core/src/lib.rs:8:pub mod engine;
crates/blit-core/src/engine/single_file.rs:29:pub(super) fn execute_single_file_copy(
crates/blit-core/src/engine/mod.rs:27:pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
crates/blit-core/src/engine/mod.rs:28:pub use summary::{LocalMirrorSummary, TransferOutcome};
crates/blit-core/src/engine/mod.rs:61:pub struct EngineRequest {
crates/blit-core/src/engine/mod.rs:76:pub struct TransferEngine;
crates/blit-core/src/engine/journal.rs:7:pub(super) fn persist_journal_checkpoints(
crates/blit-core/src/engine/journal.rs:38:pub(super) fn log_probe(label: &str, probe: &ProbeToken) {
crates/blit-core/src/engine/mirror.rs:28:pub(super) fn apply_mirror_deletions(
crates/blit-core/src/engine/tuning.rs:24:pub(super) fn select_tuning_window(
crates/blit-core/src/engine/tuning.rs:73:pub(super) fn select_tuning_window_from_history<F>(
crates/blit-core/src/engine/strategy.rs:11:pub(super) const TINY_FILE_LIMIT: usize = 256;
crates/blit-core/src/engine/strategy.rs:12:pub(super) const TINY_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
crates/blit-core/src/engine/strategy.rs:13:pub(super) const HUGE_SINGLE_BYTES: u64 = 1024 * 1024 * 1024;
crates/blit-core/src/engine/strategy.rs:16:pub(super) enum FastPathDecision {
crates/blit-core/src/engine/strategy.rs:34:pub(super) struct FastPathOutcome {
crates/blit-core/src/engine/strategy.rs:35:    pub(super) decision: Option<FastPathDecision>,
crates/blit-core/src/engine/strategy.rs:41:    pub(super) unreadable_paths: Vec<String>,
crates/blit-core/src/engine/strategy.rs:45:    pub(super) fn fast_path(decision: FastPathDecision) -> Self {
crates/blit-core/src/engine/strategy.rs:52:    pub(super) fn streaming() -> Self {
crates/blit-core/src/engine/strategy.rs:59:    pub(super) fn with_unreadable(mut self, paths: Vec<String>) -> Self {
crates/blit-core/src/engine/strategy.rs:76:pub(super) fn maybe_select_fast_path(
crates/blit-core/src/engine/history.rs:17:pub(super) fn record_performance_history(
crates/blit-core/src/engine/history.rs:112:pub(super) fn update_predictor(
crates/blit-core/src/engine/options.rs:72:    pub(crate) fn resolve_compare_snapshot(

exec
/usr/bin/zsh -lc 'rg -n "RunKind|fast_path|NullSink|is_real_transfer|append_local_record|read_recent_records|PerformanceRecord::new" crates/blit-core/src/perf_history.rs crates/blit-core/src/perf_predictor.rs crates/blit-core/src/engine crates/blit-core/tests/local_transfers.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/local_transfers.rs:40:fn tiny_manifest_records_fast_path() -> Result<()> {
crates/blit-core/tests/local_transfers.rs:65:    let records = perf_history::read_recent_records(0)?;
crates/blit-core/tests/local_transfers.rs:67:    assert_eq!(last.fast_path.as_deref(), Some("tiny_manifest"));
crates/blit-core/tests/local_transfers.rs:113:    let records = perf_history::read_recent_records(0)?;
crates/blit-core/tests/local_transfers.rs:115:    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
crates/blit-core/tests/local_transfers.rs:147:    let records = perf_history::read_recent_records(0)?;
crates/blit-core/tests/local_transfers.rs:149:    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
crates/blit-core/tests/local_transfers.rs:179:    let records = perf_history::read_recent_records(0)?;
crates/blit-core/tests/local_transfers.rs:181:    assert_eq!(last.fast_path.as_deref(), Some("single_file"));
crates/blit-core/tests/local_transfers.rs:217:    let records = perf_history::read_recent_records(0)?;
crates/blit-core/tests/local_transfers.rs:220:        last.fast_path.is_none(),
crates/blit-core/src/perf_predictor.rs:25:/// new training to `RunKind::Real` doesn't undo coefficients
crates/blit-core/src/perf_predictor.rs:46:/// callers walk the fallback chain (drop fast_path → drop dest_fs →
crates/blit-core/src/perf_predictor.rs:131:/// (1 = drop fast_path, 2 = also drop dest_fs, 3 = also drop
crates/blit-core/src/perf_predictor.rs:162:    fast_path: Option<String>,
crates/blit-core/src/perf_predictor.rs:173:            fast_path: record.fast_path.clone(),
crates/blit-core/src/perf_predictor.rs:183:        fast_path: Option<&str>,
crates/blit-core/src/perf_predictor.rs:191:            fast_path: fast_path.map(|s| s.to_string()),
crates/blit-core/src/perf_predictor.rs:232:    ///   0: exact `(src_fs, dest_fs, fast_path, skip_unchanged, checksum)`
crates/blit-core/src/perf_predictor.rs:233:    ///   1: drop `fast_path`
crates/blit-core/src/perf_predictor.rs:245:        fast_path: Option<&str>,
crates/blit-core/src/perf_predictor.rs:260:                    fast_path,
crates/blit-core/src/perf_predictor.rs:336:            record.fast_path.as_deref(),
crates/blit-core/src/perf_predictor.rs:352:            record.fast_path.as_deref(),
crates/blit-core/src/perf_predictor.rs:369:            record.fast_path.as_deref(),
crates/blit-core/src/perf_predictor.rs:391:        if !record.run_kind.is_real_transfer() {
crates/blit-core/src/perf_predictor.rs:423:        fast_path: Option<&str>,
crates/blit-core/src/perf_predictor.rs:434:                    fast_path,
crates/blit-core/src/perf_predictor.rs:636:        fast_path: Option<&str>,
crates/blit-core/src/perf_predictor.rs:642:            run_kind: crate::perf_history::RunKind::Real,
crates/blit-core/src/perf_predictor.rs:656:            fast_path: fast_path.map(str::to_string),
crates/blit-core/src/perf_predictor.rs:1015:    fn fallback_chain_drops_fast_path_then_dest_then_src() {
crates/blit-core/src/perf_predictor.rs:1016:        // Train one profile with fast_path="x", source_fs="ext4",
crates/blit-core/src/perf_predictor.rs:1018:        // mode/skip/checksum but a fast_path that has no profile.
crates/blit-core/src/perf_predictor.rs:1019:        // The query should fall through to depth 1 (drop fast_path)
crates/blit-core/src/perf_predictor.rs:1025:            // Trained profile: fast_path is None at depth 1, so we
crates/blit-core/src/perf_predictor.rs:1026:            // train with fast_path None directly.
crates/blit-core/src/perf_predictor.rs:1039:        // Query has fast_path "tiny_manifest" — no exact match;
crates/blit-core/src/perf_predictor.rs:1040:        // depth 1 drops fast_path and finds the trained profile.
crates/blit-core/src/perf_predictor.rs:1052:        assert_eq!(pred.fallback_depth, 1, "should drop fast_path");
crates/blit-core/src/perf_predictor.rs:1113:        kind: crate::perf_history::RunKind,
crates/blit-core/src/perf_predictor.rs:1118:            dry_run: matches!(kind, crate::perf_history::RunKind::DryRun),
crates/blit-core/src/perf_predictor.rs:1126:        let fast_path = match kind {
crates/blit-core/src/perf_predictor.rs:1127:            crate::perf_history::RunKind::NullSink => Some("null_sink".to_string()),
crates/blit-core/src/perf_predictor.rs:1130:        let mut record = PerformanceRecord::new(
crates/blit-core/src/perf_predictor.rs:1137:            fast_path,
crates/blit-core/src/perf_predictor.rs:1156:                crate::perf_history::RunKind::DryRun,
crates/blit-core/src/perf_predictor.rs:1188:                crate::perf_history::RunKind::NullSink,
crates/blit-core/src/perf_predictor.rs:1217:                crate::perf_history::RunKind::BenchTransfer,
crates/blit-core/src/perf_predictor.rs:1224:                crate::perf_history::RunKind::BenchWire,
crates/blit-core/src/perf_predictor.rs:1255:                crate::perf_history::RunKind::Real,
crates/blit-core/src/perf_predictor.rs:1310:            fast_path: None,
crates/blit-core/src/perf_predictor.rs:1351:            fast_path: None,
crates/blit-core/src/perf_history.rs:31:///       `fast_path == Some("null_sink")`; migration derives `run_kind`
crates/blit-core/src/perf_history.rs:38:/// Orthogonal to `RunKind`, which answers "what kind of measurement is
crates/blit-core/src/perf_history.rs:60:pub enum RunKind {
crates/blit-core/src/perf_history.rs:71:    NullSink,
crates/blit-core/src/perf_history.rs:80:impl RunKind {
crates/blit-core/src/perf_history.rs:86:    pub fn is_real_transfer(&self) -> bool {
crates/blit-core/src/perf_history.rs:87:        matches!(self, RunKind::Real)
crates/blit-core/src/perf_history.rs:142:    /// `fast_path == Some("null_sink")`. Filtering on
crates/blit-core/src/perf_history.rs:143:    /// `run_kind.is_real_transfer()` is the single chokepoint
crates/blit-core/src/perf_history.rs:147:    pub run_kind: RunKind,
crates/blit-core/src/perf_history.rs:153:    pub fast_path: Option<String>,
crates/blit-core/src/perf_history.rs:185:        fast_path: Option<String>,
crates/blit-core/src/perf_history.rs:198:            RunKind::DryRun
crates/blit-core/src/perf_history.rs:199:        } else if fast_path.as_deref() == Some("null_sink") {
crates/blit-core/src/perf_history.rs:200:            RunKind::NullSink
crates/blit-core/src/perf_history.rs:202:            RunKind::Real
crates/blit-core/src/perf_history.rs:217:            fast_path,
crates/blit-core/src/perf_history.rs:239:pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
crates/blit-core/src/perf_history.rs:278:    // `fast_path == Some("null_sink")`. R56-F1: derive the kind
crates/blit-core/src/perf_history.rs:284:    // on the field gives us RunKind::Real for a missing-field
crates/blit-core/src/perf_history.rs:291:            RunKind::DryRun
crates/blit-core/src/perf_history.rs:292:        } else if record.fast_path.as_deref() == Some("null_sink") {
crates/blit-core/src/perf_history.rs:293:            RunKind::NullSink
crates/blit-core/src/perf_history.rs:295:            RunKind::Real
crates/blit-core/src/perf_history.rs:302:pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/perf_history.rs:500:        r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":10,"total_bytes":1024,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"workers":4},"fast_path":null,"planner_duration_ms":50,"transfer_duration_ms":200,"stall_events":0,"error_count":0}"#
crates/blit-core/src/perf_history.rs:504:        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
crates/blit-core/src/perf_history.rs:586:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:620:    /// `fast_path == Some("null_sink")`. Migration must derive the
crates/blit-core/src/perf_history.rs:631:            RunKind::Real,
crates/blit-core/src/perf_history.rs:650:            RunKind::Real,
crates/blit-core/src/perf_history.rs:659:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:664:            RunKind::DryRun,
crates/blit-core/src/perf_history.rs:672:        // Old v1 record with fast_path = "null_sink".
crates/blit-core/src/perf_history.rs:673:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:679:            RunKind::NullSink,
crates/blit-core/src/perf_history.rs:680:            "fast_path=null_sink must migrate to NullSink lane"
crates/blit-core/src/perf_history.rs:685:    /// `options.dry_run` and `fast_path` so callers don't have to
crates/blit-core/src/perf_history.rs:698:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:711:        assert_eq!(record.run_kind, RunKind::DryRun);
crates/blit-core/src/perf_history.rs:716:    fn new_record_with_null_sink_fast_path_picks_nullsink_lane() {
crates/blit-core/src/perf_history.rs:726:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:739:        assert_eq!(record.run_kind, RunKind::NullSink);
crates/blit-core/src/perf_history.rs:753:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:766:        assert_eq!(record.run_kind, RunKind::Real);
crates/blit-core/src/perf_history.rs:767:        assert!(record.run_kind.is_real_transfer());
crates/blit-core/src/perf_history.rs:771:    /// gate on; pin it explicitly so changes to RunKind variants
crates/blit-core/src/perf_history.rs:774:    fn is_real_transfer_only_true_for_real() {
crates/blit-core/src/perf_history.rs:775:        assert!(RunKind::Real.is_real_transfer());
crates/blit-core/src/perf_history.rs:776:        assert!(!RunKind::DryRun.is_real_transfer());
crates/blit-core/src/perf_history.rs:777:        assert!(!RunKind::NullSink.is_real_transfer());
crates/blit-core/src/perf_history.rs:778:        assert!(!RunKind::BenchTransfer.is_real_transfer());
crates/blit-core/src/perf_history.rs:779:        assert!(!RunKind::BenchWire.is_real_transfer());
crates/blit-core/src/engine/single_file.rs:24:/// lane convention so RunKind::NullSink derivation keeps working), no
crates/blit-core/src/engine/single_file.rs:37:    let fast_path_label = if options.null_sink {
crates/blit-core/src/engine/single_file.rs:45:        Some(fast_path_label),
crates/blit-core/src/engine/mod.rs:40:use crate::perf_history::{read_recent_records, TransferMode};
crates/blit-core/src/engine/mod.rs:54:use self::strategy::{maybe_select_fast_path, FastPathDecision};
crates/blit-core/src/engine/mod.rs:67:    /// or `NullSink` locally). Fast-path strategies use their own
crates/blit-core/src/engine/mod.rs:221:        let fast_path_outcome = if options.null_sink {
crates/blit-core/src/engine/mod.rs:224:            maybe_select_fast_path(src_root, dest_root, &options)?
crates/blit-core/src/engine/mod.rs:226:        if let Some(decision) = fast_path_outcome.decision {
crates/blit-core/src/engine/mod.rs:232:            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
crates/blit-core/src/engine/mod.rs:261:                        unreadable_paths: fast_path_unreadable.clone(),
crates/blit-core/src/engine/mod.rs:298:                        unreadable_paths: fast_path_unreadable.clone(),
crates/blit-core/src/engine/mod.rs:334:                        unreadable_paths: fast_path_unreadable.clone(),
crates/blit-core/src/engine/mod.rs:386:            // all records is bounded; `read_recent_records(0)`
crates/blit-core/src/engine/mod.rs:401:                read_recent_records,
crates/blit-core/src/engine/mod.rs:479:        // NullSink -- see TransferOrchestrator).
crates/blit-core/src/engine/mod.rs:729:        let fast_path_label = if options.null_sink {
crates/blit-core/src/engine/mod.rs:737:            fast_path_label,
crates/blit-core/src/engine/tuning.rs:15:/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
crates/blit-core/src/engine/tuning.rs:33:        .filter(|record| record.run_kind.is_real_transfer())
crates/blit-core/src/engine/tuning.rs:42:        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
crates/blit-core/src/engine/tuning.rs:47:        // RunKind::Real and pass every other gate but contribute
crates/blit-core/src/engine/tuning.rs:59:/// `read_recent_records(50)`, which pre-capped the input slice
crates/blit-core/src/engine/tuning.rs:69:/// history; production passes `read_recent_records` directly.
crates/blit-core/src/engine/tuning.rs:82:    // `0` means "all records" per read_recent_records' contract
crates/blit-core/src/engine/tuning.rs:106:        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
crates/blit-core/src/engine/tuning.rs:111:        kind: RunKind,
crates/blit-core/src/engine/tuning.rs:117:        let mut r = PerformanceRecord::new(
crates/blit-core/src/engine/tuning.rs:146:    /// 30 recent NullSink records (matching the target operation
crates/blit-core/src/engine/tuning.rs:148:    /// ran first, grabbed 20 NullSinks, derive_local_plan_tuning
crates/blit-core/src/engine/tuning.rs:151:    /// Post-fix, the filter eats the NullSinks before the take, so
crates/blit-core/src/engine/tuning.rs:159:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:169:                RunKind::NullSink,
crates/blit-core/src/engine/tuning.rs:185:            "real records must reach the window; 30 NullSink records crowded them out pre-R56-F2"
crates/blit-core/src/engine/tuning.rs:188:            window.iter().all(|r| r.run_kind.is_real_transfer()),
crates/blit-core/src/engine/tuning.rs:204:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:213:                RunKind::DryRun,
crates/blit-core/src/engine/tuning.rs:240:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:249:                RunKind::BenchTransfer,
crates/blit-core/src/engine/tuning.rs:258:                RunKind::BenchWire,
crates/blit-core/src/engine/tuning.rs:272:        assert!(window.iter().all(|r| r.run_kind == RunKind::Real));
crates/blit-core/src/engine/tuning.rs:281:                    RunKind::Real,
crates/blit-core/src/engine/tuning.rs:309:        // 200 recent NullSink records (would have fit inside the
crates/blit-core/src/engine/tuning.rs:313:                RunKind::NullSink,
crates/blit-core/src/engine/tuning.rs:321:        // pre-cap=50, since the 200 NullSinks alone exceed it).
crates/blit-core/src/engine/tuning.rs:324:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:344:        assert!(window.iter().all(|r| r.run_kind.is_real_transfer()));
crates/blit-core/src/engine/tuning.rs:353:    /// pass `is_real_transfer`, pass the per-operation discriminants,
crates/blit-core/src/engine/tuning.rs:363:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:374:            let mut r = record(RunKind::Real, TransferMode::Copy, 0, 0, 10_000 + i);
crates/blit-core/src/engine/tuning.rs:375:            // Vary fast_path across the no-signal categories to
crates/blit-core/src/engine/tuning.rs:378:            r.fast_path = match i % 4 {
crates/blit-core/src/engine/tuning.rs:473:                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 100),
crates/blit-core/src/engine/tuning.rs:474:                record(RunKind::NullSink, TransferMode::Copy, 4, 1024 * 1024, 200),
crates/blit-core/src/engine/tuning.rs:492:                record(RunKind::Real, TransferMode::Copy, 4, 16 * 1024 * 1024, 100),
crates/blit-core/src/engine/tuning.rs:493:                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 200),
crates/blit-core/src/engine/tuning.rs:504:        assert_eq!(result[0].run_kind, RunKind::Real);
crates/blit-core/src/engine/tuning.rs:517:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:526:            RunKind::Real,
crates/blit-core/src/engine/tuning.rs:553:                RunKind::Real,
crates/blit-core/src/engine/tuning.rs:563:        let mut sm = record(RunKind::Real, TransferMode::Copy, 2, 8 * 1024 * 1024, 500);
crates/blit-core/src/engine/strategy.rs:45:    pub(super) fn fast_path(decision: FastPathDecision) -> Self {
crates/blit-core/src/engine/strategy.rs:76:pub(super) fn maybe_select_fast_path(
crates/blit-core/src/engine/strategy.rs:186:            FastPathOutcome::fast_path(FastPathDecision::NoWork { examined })
crates/blit-core/src/engine/strategy.rs:192:        return Ok(FastPathOutcome::fast_path(FastPathDecision::Tiny { files })
crates/blit-core/src/engine/strategy.rs:199:                FastPathOutcome::fast_path(FastPathDecision::Huge { file, size })
crates/blit-core/src/engine/strategy.rs:215:    fn tiny_fast_path_single_file() -> Result<()> {
crates/blit-core/src/engine/strategy.rs:227:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
crates/blit-core/src/engine/strategy.rs:236:    fn tiny_fast_path_many_small_files() -> Result<()> {
crates/blit-core/src/engine/strategy.rs:250:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
crates/blit-core/src/engine/strategy.rs:273:        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
crates/blit-core/src/engine/history.rs:2:    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
crates/blit-core/src/engine/history.rs:20:    fast_path: Option<&str>,
crates/blit-core/src/engine/history.rs:31:        fast_path,
crates/blit-core/src/engine/history.rs:36:    if let Err(err) = append_local_record(&record) {
crates/blit-core/src/engine/history.rs:52:    fast_path: Option<&str>,
crates/blit-core/src/engine/history.rs:87:    let mut record = PerformanceRecord::new(
crates/blit-core/src/engine/history.rs:94:        fast_path.map(|s| s.to_string()),
crates/blit-core/src/engine/summary.rs:33:/// workload's `(mode, src_fs, dst_fs, fast_path, skip_unchanged,
crates/blit-core/src/engine/summary.rs:43:    /// (drop fast_path, drop dest_fs, drop src_fs). Higher depths
crates/blit-core/src/engine/options.rs:129:    /// Discard writes (NullSink). Measures source read + pipeline throughput.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Pluggable write backends for the transfer pipeline.
     2	//!
     3	//! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
     4	//! Implementations handle the actual write: local filesystem, TCP data plane, etc.
     5	
     6	use std::path::{Path, PathBuf};
     7	use std::sync::Arc;
     8	
     9	use async_trait::async_trait;
    10	use eyre::{Context, Result};
    11	use filetime::FileTime;
    12	
    13	use crate::buffer::BufferSizer;
    14	use crate::checksum::ChecksumType;
    15	use crate::copy::{copy_file, resume_copy_file};
    16	use crate::generated::{ComparisonMode, FileHeader};
    17	use crate::logger::NoopLogger;
    18	use crate::remote::transfer::payload::PreparedPayload;
    19	use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
    20	use crate::remote::transfer::source::TransferSource;
    21	
    22	// Re-export for consumers.
    23	pub use super::data_plane::DataPlaneSession;
    24	
    25	/// Outcome of writing payload(s) to a sink.
    26	#[derive(Debug, Default, Clone)]
    27	pub struct SinkOutcome {
    28	    pub files_written: usize,
    29	    pub bytes_written: u64,
    30	}
    31	
    32	impl SinkOutcome {
    33	    pub fn merge(&mut self, other: &SinkOutcome) {
    34	        self.files_written += other.files_written;
    35	        self.bytes_written += other.bytes_written;
    36	    }
    37	}
    38	
    39	/// A pluggable write backend for the transfer pipeline.
    40	///
    41	/// Implementations receive [`PreparedPayload`] items produced by a [`TransferSource`]
    42	/// and write them to a destination (local filesystem, TCP stream, etc.).
    43	#[async_trait]
    44	pub trait TransferSink: Send + Sync {
    45	    /// Write a single prepared payload to the destination.
    46	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
    47	
    48	    /// Stream a file payload from a borrowed async reader.
    49	    ///
    50	    /// Used by the receive pipeline so file bytes that arrive on a TCP
    51	    /// wire can be written through the same sink as local copies — no
    52	    /// double-buffering into a `'static` reader. Sinks that don't
    53	    /// support inbound streaming (e.g. `GrpcFallbackSink`) inherit the
    54	    /// default error implementation.
    55	    async fn write_file_stream(
    56	        &self,
    57	        header: &FileHeader,
    58	        _reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    59	    ) -> Result<SinkOutcome> {
    60	        eyre::bail!(
    61	            "{} does not support write_file_stream (called for {})",
    62	            std::any::type_name::<Self>(),
    63	            header.relative_path
    64	        )
    65	    }
    66	
    67	    /// Signal that all payloads have been sent. Flushes buffers, sends terminators, etc.
    68	    /// Default implementation is a no-op.
    69	    async fn finish(&self) -> Result<()> {
    70	        Ok(())
    71	    }
    72	
    73	    /// Destination root path (if applicable).
    74	    fn root(&self) -> &Path;
    75	}
    76	
    77	// ---------------------------------------------------------------------------
    78	// FsTransferSink — local filesystem writer
    79	// ---------------------------------------------------------------------------
    80	
    81	/// Configuration for filesystem sink writes.
    82	#[derive(Debug, Clone)]
    83	pub struct FsSinkConfig {
    84	    pub preserve_times: bool,
    85	    pub dry_run: bool,
    86	    pub checksum: Option<ChecksumType>,
    87	    pub resume: bool,
    88	    /// R58-followup: comparison policy the sink uses when deciding
    89	    /// whether to copy a `PreparedPayload::File`. The diff_planner
    90	    /// upstream already filters by `compare_mode`, but
    91	    /// `write_file_payload` re-checks before copying as a defense
    92	    /// layer; pre-fix it called `file_needs_copy_with_checksum_type`
    93	    /// which only knows SizeMtime + Checksum, so `Force` and
    94	    /// `IgnoreTimes` were silently downgraded to SizeMtime and
    95	    /// dropped at the sink layer. The default `SizeMtime` keeps
    96	    /// pre-fix behavior for callers that haven't migrated.
    97	    pub compare_mode: ComparisonMode,
    98	}
    99	
   100	impl Default for FsSinkConfig {
   101	    fn default() -> Self {
   102	        Self {
   103	            preserve_times: true,
   104	            dry_run: false,
   105	            checksum: None,
   106	            resume: false,
   107	            compare_mode: ComparisonMode::SizeMtime,
   108	        }
   109	    }
   110	}
   111	
   112	/// Writes files directly to a local filesystem using zero-copy primitives
   113	/// (copy_file_range, sendfile, clonefile, block clone) where available.
   114	pub struct FsTransferSink {
   115	    src_root: PathBuf,
   116	    dst_root: PathBuf,
   117	    /// Canonical form of `dst_root` (or its deepest existing
   118	    /// ancestor) captured once at sink construction time. Every
   119	    /// per-entry write resolves the lexical path under `dst_root`
   120	    /// and then verifies it stays inside `canonical_dst_root`
   121	    /// post-symlink. R46-F3: pre-fix the sink only ran lexical
   122	    /// `safe_join`, so a peer-controlled relative path joined under
   123	    /// a `dst_root/link → /outside` symlink would write outside
   124	    /// the destination root.
   125	    canonical_dst_root: Option<PathBuf>,
   126	    config: FsSinkConfig,
   127	    /// Optional collector for relative paths of successfully-written
   128	    /// files. Used by remote pull's mirror flow to know which files to
   129	    /// keep when purging extraneous local entries. Each successful
   130	    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
   131	    path_tracker: Option<Arc<std::sync::Mutex<Vec<PathBuf>>>>,
   132	    /// Optional byte-level progress sink. When set,
   133	    /// `write_file_stream` passes it into
   134	    /// `receive_stream_double_buffered` so chunk-granularity
   135	    /// writes report cumulative byte progress against the
   136	    /// daemon's per-transfer counter (c-1a). Unset on the CLI
   137	    /// side; the daemon side sets it via
   138	    /// [`FsTransferSink::with_byte_progress`] from
   139	    /// `ActiveJobGuard::bytes_counter()`.
   140	    byte_progress: Option<ByteProgressSink>,
   141	}
   142	
   143	impl FsTransferSink {
   144	    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
   145	        // Best-effort canonical root capture. We don't fail
   146	        // construction if canonicalize fails (e.g. dst_root is a
   147	        // not-yet-created path under a deeply unusual filesystem) —
   148	        // instead we leave canonical_dst_root as None and the
   149	        // per-write check degrades to lexical-only with a warn.
   150	        // R46-F3: in the common case (dst_root or its ancestor
   151	        // exists) this captures the canonical form needed for
   152	        // symlink-escape rejection.
   153	        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
   154	        Self {
   155	            src_root,
   156	            dst_root,
   157	            canonical_dst_root,
   158	            config,
   159	            path_tracker: None,
   160	            byte_progress: None,
   161	        }
   162	    }
   163	
   164	    /// Enable path tracking. After each successful write, the relative
   165	    /// path of the written file is pushed onto the supplied collector.
   166	    /// Lets receive callers (e.g. mirror) discover which files survived
   167	    /// without re-implementing the record dispatch loop.
   168	    pub fn with_path_tracker(mut self, tracker: Arc<std::sync::Mutex<Vec<PathBuf>>>) -> Self {
   169	        self.path_tracker = Some(tracker);
   170	        self
   171	    }
   172	
   173	    /// Attach a byte-level progress sink. When set,
   174	    /// `write_file_stream` reports every chunk the data plane
   175	    /// writes against this sink. Used by the daemon side of
   176	    /// remote→remote transfers so `GetState.active[].bytes_completed`
   177	    /// tracks live progress; CLI-side callers omit it.
   178	    pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
   179	        self.byte_progress = Some(sink);
   180	        self
   181	    }
   182	
   183	    /// R46-F3: lexical resolve + canonical containment check in one
   184	    /// call. Used by every per-entry write site on this sink so a
   185	    /// peer-controlled relative path can't escape the destination
   186	    /// root via a pre-existing symlink. Falls back to lexical-only
   187	    /// (with a warn) if `canonical_dst_root` was None at
   188	    /// construction time — that path remains exposed but is
   189	    /// extremely unusual in practice.
   190	    fn resolve_destination(&self, wire_path: &str) -> Result<PathBuf> {
   191	        match self.canonical_dst_root.as_ref() {
   192	            Some(canonical) => {
   193	                crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
   194	            }
   195	            None => {
   196	                log::warn!(
   197	                    "FsTransferSink at '{}' has no canonical root; \
   198	                     receive falls back to lexical-only path check \
   199	                     (R46-F3 escape protection unavailable)",
   200	                    self.dst_root.display()
   201	                );
   202	                crate::path_safety::safe_join(&self.dst_root, wire_path)
   203	            }
   204	        }
   205	    }
   206	
   207	    fn track(&self, rel: &str) {
   208	        if let Some(tracker) = &self.path_tracker {
   209	            if let Ok(mut guard) = tracker.lock() {
   210	                guard.push(PathBuf::from(rel));
   211	            }
   212	        }
   213	    }
   214	}
   215	
   216	#[async_trait]
   217	impl TransferSink for FsTransferSink {
   218	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   219	        // Resume payloads need async I/O (file open + seek + write
   220	        // through tokio). Local-source payloads (File / TarShard) stay
   221	        // on a blocking thread so the zero-copy cascade and tar
   222	        // extraction can use std::fs.
   223	        let outcome = match payload {
   224	            PreparedPayload::FileBlock {
   225	                relative_path,
   226	                offset,
   227	                bytes,
   228	            } => {
   229	                write_file_block_payload(
   230	                    &self.dst_root,
   231	                    self.canonical_dst_root.as_deref(),
   232	                    &relative_path,
   233	                    offset,
   234	                    bytes,
   235	                )
   236	                .await?
   237	            }
   238	            PreparedPayload::FileBlockComplete {
   239	                relative_path,
   240	                total_size,

exec
/usr/bin/zsh -lc "rg -n \"pub struct FsTransferSink|impl FsTransferSink|fn new\\(\" crates/blit-core/src/remote/transfer/sink.rs crates/blit-core/src/remote/transfer/source.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/source.rs:54:    pub fn new(root: PathBuf) -> Self {
crates/blit-core/src/remote/transfer/source.rs:225:    pub fn new(client: RemotePullClient, root: PathBuf) -> Self {
crates/blit-core/src/remote/transfer/source.rs:344:    pub fn new(inner: Arc<dyn TransferSource>, filter: FileFilter) -> Self {
crates/blit-core/src/remote/transfer/source.rs:458:        fn new(headers: Vec<FileHeader>) -> Self {
crates/blit-core/src/remote/transfer/sink.rs:114:pub struct FsTransferSink {
crates/blit-core/src/remote/transfer/sink.rs:143:impl FsTransferSink {
crates/blit-core/src/remote/transfer/sink.rs:144:    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
crates/blit-core/src/remote/transfer/sink.rs:756:    pub fn new(
crates/blit-core/src/remote/transfer/sink.rs:856:    pub fn new() -> Self {
crates/blit-core/src/remote/transfer/sink.rs:934:    pub fn new(
crates/blit-core/src/remote/transfer/sink.rs:1112:    pub fn new(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/source.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::path::{Path, PathBuf};
     2	use std::sync::{Arc, Mutex};
     3	
     4	use async_trait::async_trait;
     5	use eyre::{bail, Result};
     6	use tokio::fs;
     7	use tokio::sync::mpsc;
     8	
     9	use crate::fs_enum::FileFilter;
    10	use crate::generated::FileHeader;
    11	use crate::remote::pull::RemotePullClient;
    12	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    13	use tokio::io::AsyncReadExt;
    14	
    15	#[async_trait]
    16	pub trait TransferSource: Send + Sync {
    17	    /// Scans the source and streams discovered file headers.
    18	    /// Returns a receiver for the headers and a join handle for the scan task.
    19	    fn scan(
    20	        &self,
    21	        filter: Option<FileFilter>,
    22	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    23	    ) -> (
    24	        mpsc::Receiver<FileHeader>,
    25	        tokio::task::JoinHandle<Result<u64>>,
    26	    );
    27	
    28	    /// Prepares a payload for transfer (e.g. opens a file or builds a tar shard).
    29	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
    30	
    31	    /// Checks if the files in the headers are available for transfer.
    32	    /// Returns a list of available headers.
    33	    async fn check_availability(
    34	        &self,
    35	        headers: Vec<FileHeader>,
    36	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    37	    ) -> Result<Vec<FileHeader>>;
    38	
    39	    /// Opens a file for reading.
    40	    async fn open_file(
    41	        &self,
    42	        header: &FileHeader,
    43	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;
    44	
    45	    /// Returns the root path of the source (if applicable).
    46	    fn root(&self) -> &Path;
    47	}
    48	
    49	pub struct FsTransferSource {
    50	    root: PathBuf,
    51	}
    52	
    53	impl FsTransferSource {
    54	    pub fn new(root: PathBuf) -> Self {
    55	        Self { root }
    56	    }
    57	}
    58	
    59	#[async_trait]
    60	impl TransferSource for FsTransferSource {
    61	    fn scan(
    62	        &self,
    63	        filter: Option<FileFilter>,
    64	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    65	    ) -> (
    66	        mpsc::Receiver<FileHeader>,
    67	        tokio::task::JoinHandle<Result<u64>>,
    68	    ) {
    69	        use crate::remote::push::client::helpers::spawn_manifest_task;
    70	        spawn_manifest_task(
    71	            self.root.clone(),
    72	            filter.unwrap_or_default(),
    73	            unreadable_paths,
    74	        )
    75	    }
    76	
    77	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
    78	        use crate::remote::transfer::payload::prepare_payload;
    79	        prepare_payload(payload, self.root.clone()).await
    80	    }
    81	
    82	    async fn check_availability(
    83	        &self,
    84	        headers: Vec<FileHeader>,
    85	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    86	    ) -> Result<Vec<FileHeader>> {
    87	        use crate::remote::push::client::helpers::filter_readable_headers;
    88	        filter_readable_headers(&self.root, headers, &unreadable_paths).await
    89	    }
    90	
    91	    async fn open_file(
    92	        &self,
    93	        header: &FileHeader,
    94	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
    95	        // An empty relative_path means "the root itself" — used when the
    96	        // enumeration root is a single file. Don't join, because
    97	        // PathBuf::join with some Path representations can produce a
    98	        // trailing-slash form that OS interprets as "descend into" and
    99	        // fails with ENOTDIR when the root is a regular file.
   100	        let path = if header.relative_path.is_empty() {
   101	            self.root.clone()
   102	        } else {
   103	            self.root.join(&header.relative_path)
   104	        };
   105	        let file = fs::File::open(&path).await?;
   106	        Ok(Box::new(file))
   107	    }
   108	
   109	    fn root(&self) -> &Path {
   110	        &self.root
   111	    }
   112	}
   113	
   114	/// Validate `FileHeader` sizes for a remote-source tar shard against
   115	/// the shared cap. Both the per-entry size and the cumulative shard
   116	/// size must stay within `tar_safety::MAX_TAR_SHARD_BYTES` so a
   117	/// hostile or buggy remote source can't force the relay into an
   118	/// unbounded allocation while building the tar.
   119	///
   120	/// Extracted from `RemoteTransferSource::prepare_payload` so the
   121	/// bounds are unit-testable without a `RemotePullClient` mock.
   122	/// Closes F7 of `docs/reviews/codebase_review_2026-05-01.md`.
   123	fn validate_remote_tar_shard_sizes(headers: &[FileHeader]) -> Result<()> {
   124	    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
   125	
   126	    for header in headers {
   127	        if header.size > MAX_TAR_SHARD_BYTES {
   128	            bail!(
   129	                "remote-source tar entry '{}' size {} exceeds local cap {} bytes",
   130	                header.relative_path,
   131	                header.size,
   132	                MAX_TAR_SHARD_BYTES
   133	            );
   134	        }
   135	    }
   136	    let total_bytes: u64 = headers
   137	        .iter()
   138	        .try_fold(0u64, |acc, h| acc.checked_add(h.size))
   139	        .ok_or_else(|| eyre::eyre!("remote-source tar shard size sum overflows u64"))?;
   140	    if total_bytes > MAX_TAR_SHARD_BYTES {
   141	        bail!(
   142	            "remote-source tar shard total size {} exceeds local cap {} bytes",
   143	            total_bytes,
   144	            MAX_TAR_SHARD_BYTES
   145	        );
   146	    }
   147	    Ok(())
   148	}
   149	
   150	/// Read exactly `expected_size` bytes from a remote-source stream
   151	/// into a bounded `Vec<u8>`. Closes R11-F1 of
   152	/// `docs/reviews/followup_review_2026-05-02.md`: previously the
   153	/// caller did `try_reserve_exact(size)` then `read_to_end(...)`,
   154	/// which only bounded the *reservation* — `read_to_end` would still
   155	/// grow the Vec past the bound if the remote source streamed extra
   156	/// bytes. Now the read itself is wrapped with `take(size + 1)` so
   157	/// over-reads are bounded at one byte past the declared size, and
   158	/// the post-read length check rejects both lie-large and lie-small.
   159	///
   160	/// Extracted as a free function so it's unit-testable against any
   161	/// `AsyncRead` (a real `RemotePullClient` stream isn't required).
   162	async fn read_remote_entry_bounded<R>(reader: R, expected_size: u64, label: &str) -> Result<Vec<u8>>
   163	where
   164	    R: tokio::io::AsyncRead + Unpin,
   165	{
   166	    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
   167	
   168	    // Defense-in-depth: this helper is private and current callers
   169	    // pre-validate, but reject explicitly so any future caller that
   170	    // bypasses validation can't allocate past the cap.
   171	    if expected_size > MAX_TAR_SHARD_BYTES {
   172	        bail!(
   173	            "remote-source entry '{}' size {} exceeds local cap {} bytes",
   174	            label,
   175	            expected_size,
   176	            MAX_TAR_SHARD_BYTES
   177	        );
   178	    }
   179	
   180	    let mut data: Vec<u8> = Vec::new();
   181	    data.try_reserve_exact(expected_size as usize)
   182	        .map_err(|err| {
   183	            eyre::eyre!(
   184	                "allocating buffer for remote-source entry '{}' (size {}): {}",
   185	                label,
   186	                expected_size,
   187	                err
   188	            )
   189	        })?;
   190	
   191	    // Read at most `expected_size + 1` bytes. The +1 is the over-read
   192	    // canary: if the post-read length is `expected_size + 1` we know
   193	    // the source sent more than declared. `expected_size` is bounded
   194	    // at `MAX_TAR_SHARD_BYTES` (above), so the addition can't overflow.
   195	    let read_limit = expected_size + 1;
   196	    let mut limited = reader.take(read_limit);
   197	    limited.read_to_end(&mut data).await?;
   198	
   199	    if data.len() as u64 != expected_size {
   200	        bail!(
   201	            "remote-source entry '{}' returned {} bytes; manifest declared {}",
   202	            label,
   203	            data.len(),
   204	            expected_size
   205	        );
   206	    }
   207	    Ok(data)
   208	}
   209	
   210	/// Transfer source backed by a remote daemon.
   211	///
   212	/// This is intentionally the legacy remote→remote relay primitive: the CLI
   213	/// pulls bytes from a source daemon, then pushes them to a destination daemon.
   214	/// Default remote→remote transfers now use destination-side `DelegatedPull`, so
   215	/// payload bytes flow source→destination without crossing the CLI host.
   216	/// `RemoteTransferSource` remains for the explicit `--relay-via-cli` escape
   217	/// hatch and for any future topology where the CLI really must be in the byte
   218	/// path.
   219	pub struct RemoteTransferSource {
   220	    client: RemotePullClient,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/strategy.rs | sed -n '1,330p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::path::{Path, PathBuf};
     2	
     3	use eyre::Result;
     4	
     5	use crate::enumeration::{EntryKind, FileEnumerator};
     6	use crate::fs_enum::{CopyJob, FileEntry};
     7	use crate::mirror_planner::MirrorPlanner;
     8	
     9	use super::LocalMirrorOptions;
    10	
    11	pub(super) const TINY_FILE_LIMIT: usize = 256;
    12	pub(super) const TINY_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
    13	pub(super) const HUGE_SINGLE_BYTES: u64 = 1024 * 1024 * 1024;
    14	
    15	#[derive(Clone, Debug)]
    16	pub(super) enum FastPathDecision {
    17	    /// Enumeration produced zero file entries to consider for copying.
    18	    /// `examined` distinguishes "source was empty / had no enumerable
    19	    /// files" (examined=0) from "source had N files but all already
    20	    /// matched the destination under skip_unchanged" (examined>0).
    21	    NoWork {
    22	        examined: usize,
    23	    },
    24	    Tiny {
    25	        files: Vec<(PathBuf, u64)>,
    26	    },
    27	    Huge {
    28	        file: PathBuf,
    29	        size: u64,
    30	    },
    31	}
    32	
    33	#[derive(Clone, Debug, Default)]
    34	pub(super) struct FastPathOutcome {
    35	    pub(super) decision: Option<FastPathDecision>,
    36	    /// R47-F4: suppressed walkdir errors observed during the
    37	    /// fast-path scan. Propagated into `LocalMirrorSummary.
    38	    /// unreadable_paths` so the CLI's source-delete step (move)
    39	    /// can refuse to remove a source it couldn't fully scan.
    40	    /// Empty on a clean walk.
    41	    pub(super) unreadable_paths: Vec<String>,
    42	}
    43	
    44	impl FastPathOutcome {
    45	    pub(super) fn fast_path(decision: FastPathDecision) -> Self {
    46	        Self {
    47	            decision: Some(decision),
    48	            unreadable_paths: Vec::new(),
    49	        }
    50	    }
    51	
    52	    pub(super) fn streaming() -> Self {
    53	        Self {
    54	            decision: None,
    55	            unreadable_paths: Vec::new(),
    56	        }
    57	    }
    58	
    59	    pub(super) fn with_unreadable(mut self, paths: Vec<String>) -> Self {
    60	        self.unreadable_paths = paths;
    61	        self
    62	    }
    63	}
    64	
    65	#[derive(Debug)]
    66	struct FastPathAbort;
    67	
    68	impl std::fmt::Display for FastPathAbort {
    69	    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    70	        write!(f, "fast-path aborted")
    71	    }
    72	}
    73	
    74	impl std::error::Error for FastPathAbort {}
    75	
    76	pub(super) fn maybe_select_fast_path(
    77	    src_root: &Path,
    78	    dest_root: &Path,
    79	    options: &LocalMirrorOptions,
    80	) -> Result<FastPathOutcome> {
    81	    if options.mirror || options.checksum || options.force_tar {
    82	        return Ok(FastPathOutcome::streaming());
    83	    }
    84	    // R58-F7: the fast-path's tiny/huge planners route through
    85	    // MirrorPlanner::should_copy_entry, which only understands
    86	    // SizeMtime (and Checksum via the checksum bool). SizeOnly /
    87	    // Force / IgnoreTimes silently became SizeMtime here, so a
    88	    // tiny-manifest copy with --size-only would still re-copy when
    89	    // mtimes differed but sizes matched. Route through the
    90	    // streaming planner, which honors all five ComparisonMode
    91	    // variants via plan_local_mirror.
    92	    if !matches!(options.compare_mode, super::LocalCompareMode::SizeMtime) {
    93	        return Ok(FastPathOutcome::streaming());
    94	    }
    95	
    96	    let mut enumerator = FileEnumerator::new(options.filter.clone_without_cache());
    97	    if !options.preserve_symlinks {
    98	        enumerator = enumerator.follow_symlinks(true);
    99	    }
   100	    if options.include_symlinks {
   101	        enumerator = enumerator.include_symlinks(true);
   102	    }
   103	
   104	    let planner = MirrorPlanner::new(options.checksum);
   105	    let mut files: Vec<(PathBuf, u64)> = Vec::new();
   106	    let mut total_bytes: u64 = 0;
   107	    let mut aborted = false;
   108	    let mut huge_candidate: Option<(PathBuf, u64)> = None;
   109	    let mut examined: usize = 0;
   110	
   111	    // R47-F4: capture suppressed walk errors so the fast-path
   112	    // summary can carry them into `summary.unreadable_paths` —
   113	    // otherwise a move with an unreadable source subdir would
   114	    // route through Tiny (or NoWork on an incremental run),
   115	    // produce a summary with empty unreadable_paths, and the
   116	    // CLI's source-delete step would proceed without seeing the
   117	    // partial-scan signal.
   118	    let scan_result = enumerator.enumerate_local_streaming_capturing(src_root, |entry| {
   119	        if let EntryKind::File { size } = entry.kind {
   120	            examined += 1;
   121	            let should_copy = if options.skip_unchanged {
   122	                let job = CopyJob {
   123	                    entry: FileEntry {
   124	                        path: entry.absolute_path.clone(),
   125	                        size,
   126	                        is_directory: false,
   127	                    },
   128	                };
   129	                planner.should_copy_entry(&job, src_root, dest_root)
   130	            } else {
   131	                true
   132	            };
   133	
   134	            if should_copy {
   135	                if files.is_empty() {
   136	                    huge_candidate = Some((entry.relative_path.clone(), size));
   137	                } else {
   138	                    huge_candidate = None;
   139	                }
   140	
   141	                files.push((entry.relative_path.clone(), size));
   142	                total_bytes += size;
   143	
   144	                if files.len() > TINY_FILE_LIMIT {
   145	                    aborted = true;
   146	                    return Err(FastPathAbort.into());
   147	                }
   148	
   149	                if total_bytes > TINY_TOTAL_BYTES && files.len() > 1 {
   150	                    aborted = true;
   151	                    return Err(FastPathAbort.into());
   152	                }
   153	            }
   154	        }
   155	
   156	        Ok(())
   157	    });
   158	
   159	    let suppressed = match scan_result {
   160	        Ok(outcome) => outcome
   161	            .suppressed_errors
   162	            .into_iter()
   163	            .map(|e| format!("{} ({})", e.path, e.message))
   164	            .collect::<Vec<_>>(),
   165	        Err(err) => {
   166	            if err.downcast_ref::<FastPathAbort>().is_none() {
   167	                return Err(err);
   168	            }
   169	            // FastPathAbort means we threw mid-walk to escape the
   170	            // tiny-budget tripwire. The capturing-enumerator's
   171	            // outcome isn't returned in that case, but the abort
   172	            // path always switches to streaming-planner, which
   173	            // does its own (capturing) source.scan() and gets a
   174	            // proper unreadable list — so leaving it empty here
   175	            // is correct.
   176	            Vec::new()
   177	        }
   178	    };
   179	
   180	    if aborted {
   181	        return Ok(FastPathOutcome::streaming().with_unreadable(suppressed));
   182	    }
   183	
   184	    if files.is_empty() {
   185	        return Ok(
   186	            FastPathOutcome::fast_path(FastPathDecision::NoWork { examined })
   187	                .with_unreadable(suppressed),
   188	        );
   189	    }
   190	
   191	    if files.len() <= TINY_FILE_LIMIT && total_bytes <= TINY_TOTAL_BYTES {
   192	        return Ok(FastPathOutcome::fast_path(FastPathDecision::Tiny { files })
   193	            .with_unreadable(suppressed));
   194	    }
   195	
   196	    if let Some((file, size)) = huge_candidate {
   197	        if size >= HUGE_SINGLE_BYTES {
   198	            return Ok(
   199	                FastPathOutcome::fast_path(FastPathDecision::Huge { file, size })
   200	                    .with_unreadable(suppressed),
   201	            );
   202	        }
   203	    }
   204	
   205	    Ok(FastPathOutcome::streaming().with_unreadable(suppressed))
   206	}
   207	
   208	#[cfg(test)]
   209	mod tests {
   210	    use super::*;
   211	    use eyre::Result;
   212	    use tempfile::tempdir;
   213	
   214	    #[test]
   215	    fn tiny_fast_path_single_file() -> Result<()> {
   216	        let temp = tempdir()?;
   217	        let src = temp.path().join("src");
   218	        let dest = temp.path().join("dest");
   219	        std::fs::create_dir_all(&src)?;
   220	        std::fs::create_dir_all(&dest)?;
   221	        std::fs::write(src.join("file.txt"), b"hello")?;
   222	
   223	        let options = LocalMirrorOptions {
   224	            perf_history: false,
   225	            ..Default::default()
   226	        };
   227	        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
   228	        assert!(matches!(
   229	            outcome.decision,
   230	            Some(FastPathDecision::Tiny { .. })
   231	        ));
   232	        Ok(())
   233	    }
   234	
   235	    #[test]
   236	    fn tiny_fast_path_many_small_files() -> Result<()> {
   237	        let temp = tempdir()?;
   238	        let src = temp.path().join("src");
   239	        let dest = temp.path().join("dest");
   240	        std::fs::create_dir_all(&src)?;
   241	        std::fs::create_dir_all(&dest)?;
   242	        for i in 0..100 {
   243	            std::fs::write(src.join(format!("file_{i}.txt")), b"data")?;
   244	        }
   245	
   246	        let options = LocalMirrorOptions {
   247	            perf_history: false,
   248	            ..Default::default()
   249	        };
   250	        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
   251	        assert!(
   252	            matches!(outcome.decision, Some(FastPathDecision::Tiny { .. })),
   253	            "100 small files should use fast path"
   254	        );
   255	        Ok(())
   256	    }
   257	
   258	    #[test]
   259	    fn streaming_path_when_over_file_limit() -> Result<()> {
   260	        let temp = tempdir()?;
   261	        let src = temp.path().join("src");
   262	        let dest = temp.path().join("dest");
   263	        std::fs::create_dir_all(&src)?;
   264	        std::fs::create_dir_all(&dest)?;
   265	        for i in 0..300 {
   266	            std::fs::write(src.join(format!("file_{i}.txt")), b"data")?;
   267	        }
   268	
   269	        let options = LocalMirrorOptions {
   270	            perf_history: false,
   271	            ..Default::default()
   272	        };
   273	        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
   274	        assert!(
   275	            outcome.decision.is_none(),
   276	            "300 files should fall through to streaming path"
   277	        );
   278	        Ok(())
   279	    }
   280	}

codex
The re-exported caller surface still points through `blit_core::orchestrator::{...}`, so app/CLI/TUI imports are structurally preserved. I did spot one dependency-direction issue in the new engine helpers and I’m checking whether it has runtime impact or is just hygiene.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/tests/local_transfers.rs | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use blit_core::config;
     2	use blit_core::orchestrator::{LocalMirrorOptions, TransferOrchestrator};
     3	use blit_core::perf_history;
     4	use eyre::Result;
     5	use std::fs;
     6	use std::path::PathBuf;
     7	use std::sync::Mutex;
     8	use tempfile::tempdir;
     9	
    10	/// Both tests mutate process-global state (the config-dir override
    11	/// and the perf-history file inside it); they must not interleave.
    12	static SERIAL: Mutex<()> = Mutex::new(());
    13	
    14	struct ConfigDirGuard {
    15	    // RAII holder: the tempdir must outlive the override.
    16	    _temp: tempfile::TempDir,
    17	    prev: Option<PathBuf>,
    18	}
    19	
    20	impl ConfigDirGuard {
    21	    fn new() -> Result<Self> {
    22	        let temp = tempdir()?;
    23	        let prev = config::config_dir_override();
    24	        config::set_config_dir(temp.path());
    25	        Ok(Self { _temp: temp, prev })
    26	    }
    27	}
    28	
    29	impl Drop for ConfigDirGuard {
    30	    fn drop(&mut self) {
    31	        if let Some(prev) = &self.prev {
    32	            config::set_config_dir(prev);
    33	        } else {
    34	            config::clear_config_dir_override();
    35	        }
    36	    }
    37	}
    38	
    39	#[test]
    40	fn tiny_manifest_records_fast_path() -> Result<()> {
    41	    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    42	    let _guard = ConfigDirGuard::new()?;
    43	    perf_history::set_perf_history_enabled(true)?;
    44	    let _ = perf_history::clear_history()?;
    45	
    46	    let tmp = tempdir()?;
    47	    let src = tmp.path().join("src");
    48	    let dest = tmp.path().join("dest");
    49	    fs::create_dir_all(&src)?;
    50	    fs::create_dir_all(&dest)?;
    51	    fs::write(src.join("a.txt"), b"one")?;
    52	    fs::write(src.join("b.txt"), b"two")?;
    53	    fs::write(src.join("c.txt"), b"three")?;
    54	
    55	    let options = LocalMirrorOptions {
    56	        progress: false,
    57	        perf_history: true,
    58	        ..Default::default()
    59	    };
    60	
    61	    let orchestrator = TransferOrchestrator::new();
    62	    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
    63	    assert_eq!(summary.copied_files, 3);
    64	
    65	    let records = perf_history::read_recent_records(0)?;
    66	    let last = records.last().expect("expected perf history record");
    67	    assert_eq!(last.fast_path.as_deref(), Some("tiny_manifest"));
    68	    Ok(())
    69	}
    70	
    71	/// ue-r2-1c behavior pin (added before the engine move): a second run
    72	/// over an already-synced directory routes through
    73	/// `FastPathDecision::NoWork{examined > 0}`, reports
    74	/// `TransferOutcome::UpToDate`, and records the `no_work` perf-history
    75	/// tag. Previously this strategy had no test at all.
    76	#[test]
    77	fn up_to_date_second_run_records_no_work() -> Result<()> {
    78	    use blit_core::orchestrator::TransferOutcome;
    79	
    80	    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
    81	    let _guard = ConfigDirGuard::new()?;
    82	    perf_history::set_perf_history_enabled(true)?;
    83	    let _ = perf_history::clear_history()?;
    84	
    85	    let tmp = tempdir()?;
    86	    let src = tmp.path().join("src");
    87	    let dest = tmp.path().join("dest");
    88	    fs::create_dir_all(&src)?;
    89	    fs::write(src.join("a.txt"), b"one")?;
    90	    fs::write(src.join("b.txt"), b"two")?;
    91	
    92	    let options = || LocalMirrorOptions {
    93	        progress: false,
    94	        perf_history: true,
    95	        // preserve_times keeps mtimes matching so the second run's
    96	        // size+mtime comparison sees both files as unchanged.
    97	        preserve_times: true,
    98	        ..Default::default()
    99	    };
   100	
   101	    let orchestrator = TransferOrchestrator::new();
   102	    let first = orchestrator.execute_local_mirror(&src, &dest, options())?;
   103	    assert_eq!(first.copied_files, 2);
   104	
   105	    let second = orchestrator.execute_local_mirror(&src, &dest, options())?;
   106	    assert_eq!(second.copied_files, 0);
   107	    assert_eq!(second.outcome, TransferOutcome::UpToDate);
   108	    assert!(
   109	        second.scanned_files >= 2,
   110	        "NoWork must report examined files"
   111	    );
   112	
   113	    let records = perf_history::read_recent_records(0)?;
   114	    let last = records.last().expect("expected perf history record");
   115	    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
   116	    Ok(())
   117	}
   118	
   119	/// ue-r2-1c behavior pin (added before the engine move): an empty
   120	/// source directory routes through `NoWork{examined: 0}` and reports
   121	/// `TransferOutcome::SourceEmpty`. Previously untested.
   122	#[test]
   123	fn empty_source_dir_reports_source_empty() -> Result<()> {
   124	    use blit_core::orchestrator::TransferOutcome;
   125	
   126	    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
   127	    let _guard = ConfigDirGuard::new()?;
   128	    perf_history::set_perf_history_enabled(true)?;
   129	    let _ = perf_history::clear_history()?;
   130	
   131	    let tmp = tempdir()?;
   132	    let src = tmp.path().join("src");
   133	    let dest = tmp.path().join("dest");
   134	    fs::create_dir_all(&src)?;
   135	
   136	    let options = LocalMirrorOptions {
   137	        progress: false,
   138	        perf_history: true,
   139	        ..Default::default()
   140	    };
   141	
   142	    let orchestrator = TransferOrchestrator::new();
   143	    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
   144	    assert_eq!(summary.copied_files, 0);
   145	    assert_eq!(summary.outcome, TransferOutcome::SourceEmpty);
   146	
   147	    let records = perf_history::read_recent_records(0)?;
   148	    let last = records.last().expect("expected perf history record");
   149	    assert_eq!(last.fast_path.as_deref(), Some("no_work"));
   150	    Ok(())
   151	}
   152	
   153	/// ue-r2-1c: the single-file shortcut historically bypassed
   154	/// perf-history recording entirely — the only strategy that did. It
   155	/// now records with the `single_file` tag and scanned-feature
   156	/// accounting (REV4 Design §2: strategies share common accounting).
   157	#[test]
   158	fn single_file_copy_records_history() -> Result<()> {
   159	    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
   160	    let _guard = ConfigDirGuard::new()?;
   161	    perf_history::set_perf_history_enabled(true)?;
   162	    let _ = perf_history::clear_history()?;
   163	
   164	    let tmp = tempdir()?;
   165	    let src = tmp.path().join("one.bin");
   166	    let dest = tmp.path().join("dest.bin");
   167	    fs::write(&src, b"payload-bytes")?;
   168	
   169	    let options = LocalMirrorOptions {
   170	        progress: false,
   171	        perf_history: true,
   172	        ..Default::default()
   173	    };
   174	
   175	    let orchestrator = TransferOrchestrator::new();
   176	    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
   177	    assert_eq!(summary.copied_files, 1);
   178	
   179	    let records = perf_history::read_recent_records(0)?;
   180	    let last = records.last().expect("expected perf history record");
   181	    assert_eq!(last.fast_path.as_deref(), Some("single_file"));
   182	    assert_eq!(last.file_count, 1);
   183	    assert_eq!(last.total_bytes, b"payload-bytes".len() as u64);
   184	    Ok(())
   185	}
   186	
   187	#[test]
   188	fn larger_manifest_records_streaming_path() -> Result<()> {
   189	    let _serial = SERIAL.lock().unwrap_or_else(|poison| poison.into_inner());
   190	    let _guard = ConfigDirGuard::new()?;
   191	    perf_history::set_perf_history_enabled(true)?;
   192	    let _ = perf_history::clear_history()?;
   193	
   194	    let tmp = tempdir()?;
   195	    let src = tmp.path().join("src");
   196	    let dest = tmp.path().join("dest");
   197	    fs::create_dir_all(&src)?;
   198	    fs::create_dir_all(&dest)?;
   199	    // Must exceed the fast-path tiny budget (TINY_FILE_LIMIT = 256
   200	    // in engine/strategy.rs) so the streaming planner runs.
   201	    // The original 32-file version predates that threshold.
   202	    for idx in 0..300 {
   203	        let file = src.join(format!("file-{idx}.txt"));
   204	        fs::write(file, format!("payload-{idx}"))?;
   205	    }
   206	
   207	    let options = LocalMirrorOptions {
   208	        progress: false,
   209	        perf_history: true,
   210	        ..Default::default()
   211	    };
   212	
   213	    let orchestrator = TransferOrchestrator::new();
   214	    let summary = orchestrator.execute_local_mirror(&src, &dest, options)?;
   215	    assert_eq!(summary.copied_files, 300);
   216	
   217	    let records = perf_history::read_recent_records(0)?;
   218	    let last = records.last().expect("expected perf history record");
   219	    assert!(
   220	        last.fast_path.is_none(),
   221	        "streaming path should not record a fast-path tag"
   222	    );
   223	    Ok(())
   224	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mirror.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Mirror-deletion pass for the engine's streaming strategy.
     2	//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
     3	
     4	use std::collections::HashSet;
     5	use std::path::Path;
     6	
     7	use eyre::{bail, Result};
     8	
     9	use crate::fs_enum::FileFilter;
    10	
    11	/// Delete destination files/dirs not present in the source header set.
    12	///
    13	/// R58-F6: `delete_scope` controls which destination entries are
    14	/// even considered for deletion:
    15	///   - `FilteredSubset` (default): enumerate the destination
    16	///     *through the user's filter*, then delete entries not in
    17	///     the source set. Excluded files (e.g. `*.log` when
    18	///     `--exclude '*.log'`) are out of scope — they're not
    19	///     candidates for deletion, and their parent directories are
    20	///     therefore non-empty from the user's perspective. When
    21	///     `remove_dir` fails with ENOTEMPTY on a parent whose only
    22	///     remaining contents are out-of-scope, we treat it as
    23	///     expected, not as an error.
    24	///   - `All`: enumerate the destination *without* the filter so
    25	///     every entry is in scope. ENOTEMPTY is a genuine error
    26	///     here (we did walk everything, so something other than
    27	///     filter-excluded content must be in the way).
    28	pub(super) fn apply_mirror_deletions(
    29	    source_paths: &HashSet<String>,
    30	    dest_root: &Path,
    31	    filter: &FileFilter,
    32	    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
    33	    perform: bool,
    34	    verbose: bool,
    35	) -> Result<(usize, usize)> {
    36	    use crate::enumeration::{EntryKind, FileEnumerator};
    37	    use crate::orchestrator::LocalMirrorDeleteScope;
    38	
    39	    // R58-F6: FilteredSubset uses the user's filter for the
    40	    // enumeration (only in-scope entries become deletion
    41	    // candidates). All bypasses the filter so every destination
    42	    // entry is considered.
    43	    let enum_filter = match delete_scope {
    44	        LocalMirrorDeleteScope::FilteredSubset => filter.clone_without_cache(),
    45	        LocalMirrorDeleteScope::All => FileFilter::default(),
    46	    };
    47	    let enumerator = FileEnumerator::new(enum_filter);
    48	    let dest_entries = enumerator.enumerate_local(dest_root)?;
    49	
    50	    // R48-F1: source.scan() only emits file headers, so
    51	    // `source_paths` is a set of *files*. Pre-fix this meant every
    52	    // destination directory was "not in source_paths" and got
    53	    // queued for deletion. Combined with R46-F5's hard-error
    54	    // policy on remove_* failures, a normal mirror containing
    55	    // `sub/file.txt` would keep `sub/file.txt`, then try
    56	    // `remove_dir("sub")` and fail the whole operation with
    57	    // ENOTEMPTY. Derive `source_dirs` from each file's parent
    58	    // chain so dest dirs that exist implicitly on the source
    59	    // side (because they contain a source file) get preserved.
    60	    let mut source_dirs: HashSet<String> = HashSet::new();
    61	    for path in source_paths {
    62	        let p = std::path::Path::new(path);
    63	        let mut cur = p.parent();
    64	        while let Some(parent) = cur {
    65	            if parent.as_os_str().is_empty() {
    66	                break;
    67	            }
    68	            let parent_str = crate::path_posix::relative_path_to_posix(parent);
    69	            // Insert and keep walking up; if already present every
    70	            // shallower ancestor is too, so we could break — but
    71	            // the walk is cheap and the eager form is simpler to
    72	            // reason about.
    73	            source_dirs.insert(parent_str);
    74	            cur = parent.parent();
    75	        }
    76	    }
    77	
    78	    let mut files_to_delete = Vec::new();
    79	    let mut dirs_to_delete = Vec::new();
    80	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/history.rs | sed -n '120,145p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_history.rs | sed -n '180,210p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   120	            if verbose {
   121	                eprintln!("Failed to persist predictor state: {err:?}");
   122	            }
   123	        }
   124	    }
   125	}
   126	
   127	#[cfg(test)]
   128	mod tests {
   129	    use super::*;
   130	    use crate::orchestrator::TransferOutcome;
   131	    use std::time::Duration;
   132	
   133	    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
   134	        LocalMirrorOptions {
   135	            mirror,
   136	            ..LocalMirrorOptions::default()
   137	        }
   138	    }
   139	
   140	    /// R44-F1 contract: the record's `(file_count, total_bytes)`
   141	    /// must mirror the orchestrator's predictor-query features.
   142	    /// Pre-fix this assertion would have failed: the record was
   143	    /// populated from `summary.copied_files` and `summary.total_bytes`
   144	    /// while the query used scanned values, so on this incremental
   145	    /// scenario (1000 scanned, 5 actually written) the predictor

 succeeded in 0ms:
   180	        source_fs: Option<String>,
   181	        dest_fs: Option<String>,
   182	        file_count: usize,
   183	        total_bytes: u64,
   184	        options: OptionSnapshot,
   185	        fast_path: Option<String>,
   186	        planner_duration_ms: u128,
   187	        transfer_duration_ms: u128,
   188	        stall_events: u32,
   189	        error_count: u32,
   190	    ) -> Self {
   191	        // R56-F1: derive `run_kind` from the call-site inputs. The
   192	        // callers that need a specific kind (bench verbs, future
   193	        // synthetic source) should mutate `record.run_kind` after
   194	        // construction; this default infers from existing fields so
   195	        // we don't have to thread a new parameter through every
   196	        // caller right now.
   197	        let run_kind = if options.dry_run {
   198	            RunKind::DryRun
   199	        } else if fast_path.as_deref() == Some("null_sink") {
   200	            RunKind::NullSink
   201	        } else {
   202	            RunKind::Real
   203	        };
   204	        let now = SystemTime::now()
   205	            .duration_since(UNIX_EPOCH)
   206	            .unwrap_or_default();
   207	        Self {
   208	            schema_version: CURRENT_SCHEMA_VERSION,
   209	            timestamp_epoch_ms: now.as_millis(),
   210	            mode,

codex
The moved fast-path and summary modules are byte-identical; history/options only differ by the resolver extraction, and single-file only differs by the wrapper plus resolver call. I’m doing one last pass over compile-facing exports and the accounting lane derivation before forming the verdict.
exec
/usr/bin/zsh -lc 'git diff --check 63b813a 29e210b' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'git show --stat --oneline 7730eb1 dc9b0ed 29e210b' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git diff --shortstat 63b813a 7730eb1 -- crates/blit-core/tests/local_transfers.rs && git diff --shortstat 7730eb1 dc9b0ed -- crates/blit-core/tests/local_transfers.rs && git diff --shortstat dc9b0ed 29e210b -- crates/blit-core/tests/local_transfers.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
7730eb1 ue-r2-1c: pin NoWork strategy behavior before the engine move
 .review/findings/ue-r2-1c.md              | 100 ++++++++++++++++++++++++++++++
 crates/blit-core/tests/local_transfers.rs |  79 +++++++++++++++++++++++
 2 files changed, 179 insertions(+)
dc9b0ed ue-r2-1c: add TransferEngine; TransferOrchestrator becomes the local adapter
 .../src/{orchestrator => engine}/history.rs        |   18 +-
 crates/blit-core/src/engine/journal.rs             |   72 +
 crates/blit-core/src/engine/mirror.rs              |  198 +++
 crates/blit-core/src/engine/mod.rs                 |  757 +++++++++
 .../src/{orchestrator => engine}/options.rs        |   51 +
 crates/blit-core/src/engine/single_file.rs         |  180 ++
 .../fast_path.rs => engine/strategy.rs}            |    0
 .../src/{orchestrator => engine}/summary.rs        |    0
 crates/blit-core/src/engine/tuning.rs              |  592 +++++++
 crates/blit-core/src/lib.rs                        |    1 +
 crates/blit-core/src/local_worker.rs               |    2 +-
 crates/blit-core/src/orchestrator/mod.rs           |   10 +-
 crates/blit-core/src/orchestrator/orchestrator.rs  | 1774 +-------------------
 crates/blit-core/tests/local_transfers.rs          |    7 +-
 crates/blit-daemon/src/service/pull.rs             |    4 +-
 15 files changed, 1922 insertions(+), 1744 deletions(-)
29e210b ue-r2-1c: single-file strategy gains perf-history/predictor accounting
 crates/blit-core/src/engine/single_file.rs | 70 +++++++++++++++++++++---------
 crates/blit-core/tests/local_transfers.rs  | 34 +++++++++++++++
 2 files changed, 84 insertions(+), 20 deletions(-)

 succeeded in 0ms:
 1 file changed, 79 insertions(+)
 1 file changed, 5 insertions(+), 2 deletions(-)
 1 file changed, 34 insertions(+)

exec
/usr/bin/zsh -lc "git grep -n \"orchestrator::fast_path\\|orchestrator::history\\|orchestrator::options\\|orchestrator::summary\" 29e210b -- crates" in /home/michael/dev/Blit
 exited 1 in 0ms:
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '360,455p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   360	    /// Convenience wrapper to predict end-to-end (planner +
   361	    /// transfer) duration. The decision the orchestrator makes uses
   362	    /// this as the high-level cost estimate.
   363	    pub fn predict_total(&self, record: &PerformanceRecord) -> Option<Prediction> {
   364	        self.predict(
   365	            DurationKind::Total,
   366	            record.mode.clone(),
   367	            record.source_fs.as_deref(),
   368	            record.dest_fs.as_deref(),
   369	            record.fast_path.as_deref(),
   370	            record.options.skip_unchanged,
   371	            record.options.checksum,
   372	            record.file_count,
   373	            record.total_bytes,
   374	        )
   375	    }
   376	
   377	    /// Apply a completed run's observed durations to its profile.
   378	    /// Updates BOTH the planner and transfer coefficient sets so
   379	    /// future `predict_planner` / `predict_transfer` calls see the
   380	    /// gradient-descent update. The single observation counter is
   381	    /// shared because the two targets always update together.
   382	    ///
   383	    /// R56-F1: silently skips non-real-transfer records (dry-run,
   384	    /// null-sink, bench). The predictor's job is to model
   385	    /// production transfer cost; a dry-run with zero transfer
   386	    /// duration or a null-sink run with cost-free writes would
   387	    /// pull the coefficients toward wrong values. Bench records
   388	    /// belong on a future separate predictor lane (see
   389	    /// `BENCH_VERB_PLAN.md` §6); for now they're just dropped.
   390	    pub fn observe(&mut self, record: &PerformanceRecord) {
   391	        if !record.run_kind.is_real_transfer() {
   392	            return;
   393	        }
   394	        let key = ProfileKey::new(record);
   395	        let profile = self
   396	            .state
   397	            .profiles
   398	            .entry(key)
   399	            .or_insert_with(PredictorProfile::new);
   400	        profile.coefficients.planner.apply_observation(
   401	            record.file_count,
   402	            record.total_bytes,
   403	            record.planner_duration_ms as f64,
   404	        );
   405	        profile.coefficients.transfer.apply_observation(
   406	            record.file_count,
   407	            record.total_bytes,
   408	            record.transfer_duration_ms as f64,
   409	        );
   410	        profile.observations = profile.observations.saturating_add(1);
   411	    }
   412	
   413	    /// Coefficient inspection helper for `blit profile --json`. Walks
   414	    /// the same fallback chain as `predict()` but returns the raw
   415	    /// coefficient sets so operators can audit what the predictor
   416	    /// actually believes about a workload class. Returns `None` if
   417	    /// no profile in the chain meets the confidence threshold.
   418	    pub fn coefficients_for(
   419	        &self,
   420	        mode: TransferMode,
   421	        source_fs: Option<&str>,
   422	        dest_fs: Option<&str>,
   423	        fast_path: Option<&str>,
   424	        skip_unchanged: bool,
   425	        checksum: bool,
   426	    ) -> Option<(DurationCoefficients, u64, usize)> {
   427	        let candidates: [(usize, ProfileKey); 4] = [
   428	            (
   429	                0,
   430	                ProfileKey::from_components(
   431	                    source_fs.map(str::to_string),
   432	                    dest_fs.map(str::to_string),
   433	                    mode.clone(),
   434	                    fast_path,
   435	                    skip_unchanged,
   436	                    checksum,
   437	                ),
   438	            ),
   439	            (
   440	                1,
   441	                ProfileKey::from_components(
   442	                    source_fs.map(str::to_string),
   443	                    dest_fs.map(str::to_string),
   444	                    mode.clone(),
   445	                    None,
   446	                    skip_unchanged,
   447	                    checksum,
   448	                ),
   449	            ),
   450	            (
   451	                2,
   452	                ProfileKey::from_components(
   453	                    source_fs.map(str::to_string),
   454	                    None,
   455	                    mode.clone(),

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '150,210p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   150	        Self {
   151	            coefficients: DurationCoefficients::default(),
   152	            observations: 0,
   153	        }
   154	    }
   155	}
   156	
   157	#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
   158	struct ProfileKey {
   159	    source_fs: Option<String>,
   160	    dest_fs: Option<String>,
   161	    mode: TransferMode,
   162	    fast_path: Option<String>,
   163	    skip_unchanged: bool,
   164	    checksum: bool,
   165	}
   166	
   167	impl ProfileKey {
   168	    fn new(record: &PerformanceRecord) -> Self {
   169	        Self {
   170	            source_fs: record.source_fs.clone(),
   171	            dest_fs: record.dest_fs.clone(),
   172	            mode: record.mode.clone(),
   173	            fast_path: record.fast_path.clone(),
   174	            skip_unchanged: record.options.skip_unchanged,
   175	            checksum: record.options.checksum,
   176	        }
   177	    }
   178	
   179	    fn from_components(
   180	        source_fs: Option<String>,
   181	        dest_fs: Option<String>,
   182	        mode: TransferMode,
   183	        fast_path: Option<&str>,
   184	        skip_unchanged: bool,
   185	        checksum: bool,
   186	    ) -> Self {
   187	        Self {
   188	            source_fs,
   189	            dest_fs,
   190	            mode,
   191	            fast_path: fast_path.map(|s| s.to_string()),
   192	            skip_unchanged,
   193	            checksum,
   194	        }
   195	    }
   196	}
   197	
   198	#[derive(Debug, Clone, Serialize, Deserialize)]
   199	struct PredictorState {
   200	    version: u32,
   201	    #[serde(with = "profile_map")]
   202	    profiles: HashMap<ProfileKey, PredictorProfile>,
   203	}
   204	
   205	impl PredictorState {
   206	    fn new() -> Self {
   207	        Self {
   208	            version: STATE_VERSION,
   209	            profiles: HashMap::new(),
   210	        }

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '320,360p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   320	                        fallback_depth: *depth,
   321	                    });
   322	                }
   323	            }
   324	        }
   325	        None
   326	    }
   327	
   328	    /// Convenience wrapper to predict planner duration for a record
   329	    /// shape. Equivalent to `predict(DurationKind::Planner, …)`.
   330	    pub fn predict_planner(&self, record: &PerformanceRecord) -> Option<Prediction> {
   331	        self.predict(
   332	            DurationKind::Planner,
   333	            record.mode.clone(),
   334	            record.source_fs.as_deref(),
   335	            record.dest_fs.as_deref(),
   336	            record.fast_path.as_deref(),
   337	            record.options.skip_unchanged,
   338	            record.options.checksum,
   339	            record.file_count,
   340	            record.total_bytes,
   341	        )
   342	    }
   343	
   344	    /// Convenience wrapper to predict transfer duration for a record
   345	    /// shape. Equivalent to `predict(DurationKind::Transfer, …)`.
   346	    pub fn predict_transfer(&self, record: &PerformanceRecord) -> Option<Prediction> {
   347	        self.predict(
   348	            DurationKind::Transfer,
   349	            record.mode.clone(),
   350	            record.source_fs.as_deref(),
   351	            record.dest_fs.as_deref(),
   352	            record.fast_path.as_deref(),
   353	            record.options.skip_unchanged,
   354	            record.options.checksum,
   355	            record.file_count,
   356	            record.total_bytes,
   357	        )
   358	    }
   359	
   360	    /// Convenience wrapper to predict end-to-end (planner +

exec
/usr/bin/zsh -lc "rg -n \"predict\\(|observe\\(|fast_path\" crates/blit-core/src/perf_predictor.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
21:/// Bumped to v3 in R56-F1: previously `observe()` trained
46:/// callers walk the fallback chain (drop fast_path → drop dest_fs →
131:/// (1 = drop fast_path, 2 = also drop dest_fs, 3 = also drop
162:    fast_path: Option<String>,
173:            fast_path: record.fast_path.clone(),
183:        fast_path: Option<&str>,
191:            fast_path: fast_path.map(|s| s.to_string()),
232:    ///   0: exact `(src_fs, dest_fs, fast_path, skip_unchanged, checksum)`
233:    ///   1: drop `fast_path`
239:    pub fn predict(
245:        fast_path: Option<&str>,
260:                    fast_path,
329:    /// shape. Equivalent to `predict(DurationKind::Planner, …)`.
331:        self.predict(
336:            record.fast_path.as_deref(),
345:    /// shape. Equivalent to `predict(DurationKind::Transfer, …)`.
347:        self.predict(
352:            record.fast_path.as_deref(),
364:        self.predict(
369:            record.fast_path.as_deref(),
390:    pub fn observe(&mut self, record: &PerformanceRecord) {
414:    /// the same fallback chain as `predict()` but returns the raw
423:        fast_path: Option<&str>,
434:                    fast_path,
636:        fast_path: Option<&str>,
656:            fast_path: fast_path.map(str::to_string),
712:            predictor.observe(&make_record(
750:            predictor.observe(&make_record(
764:            predictor.observe(&make_record(
794:            predictor.observe(&make_record(
802:            predictor.observe(&make_record(
833:            predictor.observe(&make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 250));
883:        predictor.observe(&make_record(TransferMode::Copy, 100, 1024, 50));
898:            predictor.observe(&make_record(TransferMode::Copy, 100, 1024, 50));
917:            predictor.observe(&make_record(TransferMode::Copy, 100, 0, 100));
918:            predictor.observe(&make_record(TransferMode::Copy, 1000, 0, 1000));
948:            predictor.observe(&make_record_full(
988:            predictor.observe(&make_record_full(
1015:    fn fallback_chain_drops_fast_path_then_dest_then_src() {
1016:        // Train one profile with fast_path="x", source_fs="ext4",
1018:        // mode/skip/checksum but a fast_path that has no profile.
1019:        // The query should fall through to depth 1 (drop fast_path)
1025:            // Trained profile: fast_path is None at depth 1, so we
1026:            // train with fast_path None directly.
1027:            predictor.observe(&make_record_full(
1039:        // Query has fast_path "tiny_manifest" — no exact match;
1040:        // depth 1 drops fast_path and finds the trained profile.
1052:        assert_eq!(pred.fallback_depth, 1, "should drop fast_path");
1107:    // ── R56-F1: observe() filters non-real records ────────────────────
1126:        let fast_path = match kind {
1137:            fast_path,
1155:            predictor.observe(&record_with_lane(
1161:        // No real records yet → predict() returns None (below the
1166:                .predict(
1187:            predictor.observe(&record_with_lane(
1195:                .predict(
1216:            predictor.observe(&record_with_lane(
1223:            predictor.observe(&record_with_lane(
1231:                .predict(
1254:            predictor.observe(&record_with_lane(
1261:            .predict(
1310:            fast_path: None,
1351:            fast_path: None,

codex
- [crates/blit-core/src/engine/mirror.rs:32](/home/michael/dev/Blit/crates/blit-core/src/engine/mirror.rs:32) - Low - `engine` reaches back through `crate::orchestrator::LocalMirrorDeleteScope` for a type now owned/re-exported by `engine`. Runtime behavior is preserved, but this inverts the intended engine→adapter layering and makes future adapter refactors brittle; use the engine-local type path instead.

VERDICT: PASS with one non-blocking Low hygiene finding; review-only, tests not run.
tokens used
160,013
- [crates/blit-core/src/engine/mirror.rs:32](/home/michael/dev/Blit/crates/blit-core/src/engine/mirror.rs:32) - Low - `engine` reaches back through `crate::orchestrator::LocalMirrorDeleteScope` for a type now owned/re-exported by `engine`. Runtime behavior is preserved, but this inverts the intended engine→adapter layering and makes future adapter refactors brittle; use the engine-local type path instead.

VERDICT: PASS with one non-blocking Low hygiene finding; review-only, tests not run.
