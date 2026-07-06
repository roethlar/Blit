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
session id: 019f38ad-5003-78b0-8bef-bbe5934991cd
--------
user
Review the diff of commit e6a0b3b (run: git show e6a0b3b). It implements otp-5b-1, the single-stream SOURCE-responder TCP data plane for the ONE_TRANSFER_PATH unified transfer session (docs/plan/ONE_TRANSFER_PATH.md slice otp-5; contract docs/TRANSFER_SESSION.md; finding .review/findings/otp-5b-source-responder-data-plane.md). It decouples data-plane connection role (RESPONDER binds+accepts, INITIATOR dials) from byte role (SOURCE sends, DESTINATION receives), adding the pull pair: a SOURCE responder that accepts+sends (accept_source_data_plane) and a DESTINATION initiator that dials+receives (dial_destination_data_plane), single-stream (no resize; that is otp-5b-2). Check: correctness regressions in the push path (the existing DESTINATION-responder/SOURCE-initiator data plane must be unchanged in behavior); that a SOURCE responder never proposes a DataPlaneResize (resizable=false) and a DESTINATION initiator rejects one; deadlock/ordering of the up-front accept (SOURCE responder) vs up-front dial (DESTINATION initiator) relative to manifest streaming; byte-identical results and byte-accounting (the DESTINATION is the scorer); StallGuard coverage on both new socket paths; AbortOnDrop leak-safety; that the same-build handshake and in-stream fallback still hold; and that the test count did not drop (1519 -> 1521). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review the target commit and the repo guidance first, then trace the changed paths against the transfer-session contract and tests. No skills invoked.
exec
/usr/bin/zsh -lc "sed -n '1,240p' .agents/repo-guidance.md" in /home/michael/dev/Blit
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
`.agents/skills/`. This repo drafts `.agents/playbooks/reviewloop.md` as a template, but the codex review loop and `docs/agent/PROTOCOL.md` already cover that role for review-loop work.

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
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames e6a0b3b' in /home/michael/dev/Blit
 succeeded in 0ms:
e6a0b3b (HEAD -> master) otp-5b-1: single-stream SOURCE-responder data plane (transport/role decoupling)
 .../findings/otp-5b-source-responder-data-plane.md | 153 ++++++++++++++
 .../src/remote/transfer/session_client.rs          |  34 +++-
 .../blit-core/src/transfer_session/data_plane.rs   | 211 ++++++++++++++++++-
 crates/blit-core/src/transfer_session/mod.rs       | 223 ++++++++++++++-------
 crates/blit-core/tests/transfer_session_roles.rs   |  82 ++++++++
 .../src/service/transfer_session_e2e.rs            |  59 +++++-
 6 files changed, 665 insertions(+), 97 deletions(-)

exec
/usr/bin/zsh -lc "sed -n '1,240p' .agents/state.md" in /home/michael/dev/Blit
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
/usr/bin/zsh -lc "sed -n '1,220p' .agents/repo-map.json" in /home/michael/dev/Blit
 succeeded in 0ms:
{
  "validated_against": {
    "commit": "1ff36c712de9f8372238b5608227a0cd6f9ad232",
    "date": "2026-07-05"
  },
  "projects": [
    {
      "name": "blit",
      "type": "rust-cargo-workspace",
      "path": ".",
      "members": [
        "crates/blit-core",
        "crates/blit-cli",
        "crates/blit-daemon",
        "crates/blit-app",
        "crates/blit-tui",
        "crates/blit-prometheus-bridge"
      ],
      "notes": "proto/blit.proto holds the gRPC definitions; blit-core's build script vendors protoc. Integration tests live per-crate (e.g. crates/blit-cli/tests/, crates/blit-core/tests/); the root Cargo.toml is a virtual workspace, so a root-level tests/ dir would never be compiled (w9-2 relocated the old one). blit-utils was intentionally removed; its admin verbs now live in blit-cli."
    }
  ],
  "verification": {
    "status": "confirmed",
    "commands": [
      "cargo fmt --all -- --check",
      "cargo clippy --workspace --all-targets -- -D warnings",
      "cargo test --workspace",
      "bash scripts/agent/check-docs.sh"
    ],
    "policy": {
      "code_changes": "Run the full validation suite (fmt, clippy, test) before claiming completion or writing a review sentinel. Test count never drops versus the prior baseline unless the removal is called out in the finding doc.",
      "docs_only": "Code verification is not required, but scripts/agent/check-docs.sh must pass before pushing docs changes.",
      "manual_behavior": "Windows parity: after touching platform-specific code (win_fs, planners), run scripts/windows/run-blit-tests.ps1, or state clearly that it was not run.",
      "ci_gate": "A push touching crates/** or proto/** must also touch docs/STATE.md unless a commit message contains [state: skip] (docs-gate.yml)."
    }
  },
  "fact_bearing_paths": [
    "docs/STATE.md",
    "docs/DECISIONS.md",
    "REVIEW.md",
    ".review/",
    "docs/agent/PROTOCOL.md",
    "docs/plan/"
  ],
  "guidance_paths": [
    "AGENTS.md",
    "CLAUDE.md",
    "GEMINI.md",
    "docs/agent/PROTOCOL.md",
    ".agents/repo-guidance.md",
    ".agents/push-policy.md",
    ".agents/state.md",
    ".agents/decisions.md",
    ".agents/repo-map.json",
    ".agents/artifact-manifest.json"
  ],
  "notes": [
    "State lives canonically in docs/STATE.md and decisions in docs/DECISIONS.md; .agents/state.md and .agents/decisions.md are pointer stubs only. CI and hook scripts are wired to the docs/ paths.",
    "As of the 2026-07-03 governance reconciliation, AGENTS.md is a byte-identical copy of the toolkit template; every repo-specific rule (project map, style, git safety, source-of-truth order, operator procedures) lives in .agents/repo-guidance.md instead.",
    "DEVLOG.md is an append-only journal: write to it, never read it for current state. TODO.md is backlog-only.",
    "Verification commands confirmed against .agents/repo-guidance.md, .review/README.md, and the CI workflows; not executed during the bootstrap run itself (docs-only change).",
    "2026-07-05 update-governance run: re-confirmed workspace members against Cargo.toml, script paths, and CI branch triggers against master with no drift found; updated template to 2026-07-04.1 and added hooks/playbooks."
  ]
}

codex
The commit is on `master` and touches the core data-plane/session paths plus targeted tests. I’m pulling the contract and slice docs next so the review checks behavior against the intended otp-5b-1 boundary, not just the diff shape.
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-06 (**otp-5a landed + graded (codex PASS, no
findings)** — the one served `Transfer` RPC now handles BOTH directions
by the client's declared role; a DESTINATION initiator makes the daemon
the SOURCE (pull-equivalent), in-stream carrier. ONE_TRANSFER_PATH
otp-1 + otp-3 + otp-4a + otp-4b (1/2/3) + otp-5a `[x]`, current slice
**otp-5b** (SOURCE-responder data plane). SMALL_FILE_CEILING stays
paused, D-2026-07-05-1.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
  invariant (plan doc, verbatim): ONE block of transfer code;
  direction/initiator/verb can NEVER affect wall time by blit's doing
  — impossible by construction because the per-direction drivers and
  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
  cells. **D-2026-07-05-2: same-build peers only, refusal at session
  open.** Progress (each through the codex loop; closed-slice detail in
  DEVLOG + `.review/` + REVIEW.md):
  - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
    (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
    in-process transport (invariance property in the role suite); daemon
    serves `Transfer` as Responder, client push over gRPC; A/B
    byte-identical vs old push; SizeMtime = data-safe skip (owner-ack
    open question, below).
  - **otp-4b (1/2/3) `[x]` — data plane fully on the session, closed**:
    4b-1 single-stream TCP data plane; 4b-2 mid-transfer resize/
    multi-stream + sf-2 shape correction; 4b-3 deterministic mid-transfer
    cancel (`CancelJob`→`SessionFault{CANCELLED}` over the data plane, no
    hang). Detail: DEVLOG + `.review/results/otp-4b*`. Suite → 1516/0.
  - **otp-5a `[x]`** (`84be1cc`, codex PASS no findings) — the one served
    `Transfer` RPC serves BOTH roles: new `run_responder` reads the open
    and dispatches on declared `initiator_role` (SOURCE-init→daemon
    DESTINATION = otp-4 push; DESTINATION-init→daemon SOURCE =
    pull-equivalent, streams its module tree, in-stream). `establish` →
    `exchange_hello`+`responder_finish`; bodies → `drive_source`/
    `drive_destination`; new `SourceResponderTarget`; client
    `run_pull_session`. A/B byte-identical vs old `pull_sync`. Suite →
    **1519/0**. (DEVLOG 07:30.)
  - Current: **otp-5b** (SOURCE-responder data plane — the transport/role
    decoupling: the *responder* binds+grants+accepts while the SOURCE
    *sends*, and the *initiator* dials while the DESTINATION *receives*;
    today the data plane is keyed to role, so this is genuine new work,
    not just "roles flipped"). (otp-2 symmetric baseline is rig-gated;
    before otp-10.)
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+
  blocked** until ONE_TRANSFER_PATH ships, then resume/re-derive on
  the unified baseline. Principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
  declarations pending in Blocked); 10 GbE session done; w9-3 + eleven
  review-queue rows landed. Codex loop governs all code + plan changes
  (D-2026-07-04-1). Details: DEVLOG 2026-07-04/05.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a `[x]`. Current: **otp-5b** (SOURCE-responder
   data plane — the responder binds+grants+accepts while SENDING and the
   initiator dials while RECEIVING; the transport/role decoupling the
   in-stream otp-5a deferred). otp-2 (symmetric baseline) is RIG-GATED —
   before otp-10 cutover.
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
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
   test data, everything; nothing written outside it, ever. Zero-copy
   is pre-authorized to be tested there when the post-cutover slice set
   reaches it; no daemon runs on zoey before then without a fresh go.
6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
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

- **(OPEN — owner ack requested, 2026-07-05, otp-4a)** Unified
  SizeMtime semantic: same-size + dest-NEWER — old push clobbers, the
  session adopts the **data-safe SKIP** (converge-up; `--force` still
  overwrites; pinned by
  `same_size_newer_destination_is_skipped_not_clobbered`). So "byte-
  identical trees vs old push" is intentionally not literal in that one
  cell. Owner: confirm, or ask for old-push clobber (one-line change).
  Full reasoning: `.review/findings/otp-4-daemon-serves-transfer.md`.
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
  locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
  on the next push (10d89e0 predates the w9-3 fix).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-06 (31st)** @ `84be1cc` — **otp-5a landed and graded (codex
  PASS, no findings)** (DEVLOG 07:30, finding
  `.review/findings/otp-5-daemon-as-source.md`, verdict
  `…/otp-5a-daemon-as-source.gpt-verdict.md`). The one served `Transfer`
  RPC handles BOTH directions by declared role: `run_responder` reads the
  open and dispatches (SOURCE-init→daemon DESTINATION = push;
  DESTINATION-init→daemon SOURCE = pull, streams its module tree,
  in-stream). `establish`→`exchange_hello`+`responder_finish`; bodies→
  `drive_source`/`drive_destination`; `SourceResponderTarget`; client
  `run_pull_session`. A/B byte-identical vs old `pull_sync`; guard proven
  live. Suite 1516 → **1519/0**; role suite green. Also fixed a review
  gotcha: `codex exec` hangs on open stdin (0.142) — loop doc §4 gained
  `</dev/null`. **Exact first action next session**: otp-5b
  (SOURCE-responder data plane — responder binds+grants+accepts while
  SENDING, initiator dials while RECEIVING; data plane is keyed to role
  today, so real work). In-flight: none. Owner declarations: three 10 GbE
  gates + push go remain in Blocked; `f6e592e`..HEAD unpushed. (Unrelated
  `Cargo.lock` transitive drift left unstaged.)
- **2026-07-06 (30th)** @ `3ae0a5f`+`a530005`+`46cc4bb`+`db9b63d` —
  **otp-4b-3 landed and graded (codex 3 passes, PASS); otp-4b fully
  closed** (DEVLOG 05:37, `.review/results/otp-4b3-*`). Mid-transfer
  `CancelJob`→`SessionFault{CANCELLED}` over the data plane, no
  blocked-reader hang. Suite 1513 → 1516/0. (Older graded slices —
  otp-4b-2 `dce56de`, otp-4b-1 `777dfc5`, otp-4a `fe4ad6d` — in DEVLOG.)

exec
/usr/bin/zsh -lc 'rg -n "otp-5|SOURCE-responder|DataPlaneResize|ONE_TRANSFER_PATH|resizable|single-stream|manifest" docs/plan/ONE_TRANSFER_PATH.md' in /home/michael/dev/Blit
 succeeded in 0ms:
155:2. SOURCE enumerates and **streams** its manifest immediately (no
178:   manifest it received (filter-scoped, scan-complete-guarded) and
208:today's resume path is an explicit single-stream RELIABLE exception
271:5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-

exec
/usr/bin/zsh -lc "sed -n '1,260p' .review/findings/otp-5b-source-responder-data-plane.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# otp-5b — SOURCE-responder data plane (transport/role decoupling)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-4b (the TCP data plane: DESTINATION responder binds+grants+
accepts+receives; SOURCE initiator dials+sends) and otp-5a (the daemon serves
BOTH roles by declared initiator role, pull-equivalent in-stream only).

## Staging (mirrors otp-4b)

otp-5 is staged like otp-4b. **This slice is otp-5b-1**: the single-stream
SOURCE-responder data plane, no resize. otp-5b-2 adds the mid-transfer
shape-correction resize for the pull direction (the accept-based epoch-N
socket); otp-5b-3, if needed, mirrors otp-4b-3's mid-transfer cancel over the
pull data plane.

## The decoupling this slice makes

Today (otp-4b) the data plane is keyed to **role**:

- the **DESTINATION** binds a listener, grants it in `SessionAccept`, accepts
  sockets, and **receives** bytes (`ResponderDataPlane`);
- the **SOURCE** dials the granted sockets and **sends** bytes
  (`SourceDataPlane`).

That coincidence holds only for push, where the DESTINATION *is* the responder
and the SOURCE *is* the initiator. The plan's transport rule (§Design,
§Transport facts) is two independent axes:

- **connection role** — the RESPONDER binds+accepts, the INITIATOR dials (NAT
  reality: the connection-initiating end always dials);
- **byte role** — the SOURCE sends, the DESTINATION receives.

For pull the client is the DESTINATION *initiator* (must dial+receive) and the
daemon is the SOURCE *responder* (must bind+accept+send). This slice adds those
two new combinations without disturbing the push pair:

| byte role \ conn role | initiator (dial)                    | responder (bind+accept)          |
|-----------------------|-------------------------------------|----------------------------------|
| SOURCE (send)         | push: `dial_source_data_plane` ✓    | **pull (new): accept + send**    |
| DESTINATION (receive) | **pull (new): dial + receive**      | push: `ResponderDataPlane` ✓     |

The byte machinery is fully reused — send is `DataPlaneSession` +
`DataPlaneSink` + `execute_sink_pipeline_elastic`; receive is `StallGuard` +
`execute_receive_pipeline`. Only socket **acquisition** (dial vs accept) is new
per byte role, and `DataPlaneSession::from_stream` already builds a send session
from an accepted socket (the old `pull_sync` path uses it).

## Scope of otp-5b-1: single stream, no resize

The pull data plane runs at **exactly the epoch-0 grant (1 stream)**. No
`DataPlaneResize` is proposed by the SOURCE responder and none is handled by the
DESTINATION initiator. Mechanically this is enforced by capping the SOURCE
responder's send dial to `max_streams = 1`, so `propose_resize` returns `None`
and no resize frame is ever emitted — the same suppression otp-4b-1 relied on
before otp-4b-2 lifted it. The DESTINATION initiator treats a `DataPlaneResize`
frame as a protocol violation (there is none in this slice). otp-5b-2 lifts the
cap and adds the accept-based epoch-N socket + the ack→dial choreography.

Resize choreography note (for otp-5b-2, not implemented here): the control-lane
frames are identical in both directions — the SOURCE proposes `Resize{ADD}`, the
DESTINATION acks. Only the transport action flips: in push the SOURCE=initiator
dials the epoch-N socket and the DESTINATION=responder arms+accepts; in pull the
SOURCE=responder arms+accepts and the DESTINATION=initiator dials.

## Approach (as implemented)

- **`responder_finish` binds for either role** (`transfer_session/mod.rs`): the
  `local_role == Destination` gate on the data-plane bind is removed; a
  responder binds a data plane whenever `!open.in_stream_bytes`, regardless of
  role. The bound listener + grant travel in `Negotiated.responder_data_plane`;
  the grant goes out in `SessionAccept`. `receiver_capacity` in the accept stays
  DESTINATION-only (the byte RECEIVER advertises capacity; a DESTINATION
  initiator advertises it in its own `SessionOpen.receiver_capacity`, already so
  since otp-4a).
- **SOURCE responder accept+send** (`transfer_session/data_plane.rs`):
  `accept_source_data_plane(bound, receiver_capacity, source)` accepts the
  epoch-0 socket(s) off the bound listener, wraps each in
  `DataPlaneSession::from_stream` → `DataPlaneSink`, and drives the SAME elastic
  send pipeline `dial_source_data_plane` builds — returning the same
  `SourceDataPlane` handle. Its dial is capped to a single stream (no resize).
  `source_send_half` picks accept-vs-dial by whether it holds a bound responder
  listener (`responder_data_plane`) or a received grant (`accept.data_plane`);
  everything after socket acquisition (`queue`/`finish`) is unchanged.
- **DESTINATION initiator dial+receive** (`transfer_session/data_plane.rs`):
  `dial_destination_data_plane(host, grant)` dials the epoch-0 socket(s), spawns
  one `execute_receive_pipeline` worker per socket into a `JoinSet`, and
  `finish()` joins them for the `ReceiveTotals` (settled stream count + write
  outcome) the sf pin reads. `destination_session` selects it when it holds no
  bound listener but a received grant + a `data_plane_host`.
- **Config threading**: `DestinationSessionConfig` gains `data_plane_host:
  Option<String>` (the initiator dials the responder's host, same host it
  reached the control plane on — symmetric with `SourceSessionConfig`).
  `drive_destination`/`destination_session` take it.
- **Client** (`session_client.rs`): `run_pull_session` sets
  `in_stream_bytes = options.in_stream_bytes` (default `false` = TCP data plane)
  and passes `data_plane_host: Some(endpoint.host)`. A `PullSessionOptions
  { in_stream_bytes }` knob keeps the in-stream fallback reachable (diagnostics),
  matching `PushSessionOptions`.
- **Daemon** (`service/transfer.rs`): unchanged — `run_responder` already routes
  the SOURCE-responder path; the bound listener now flows through it.

## Compare semantics

Unchanged from otp-5a: the DESTINATION is the one diff owner; same-size +
dest-NEWER resolves to the data-safe SKIP (the still-open owner-ack question, not
reopened here). A/B vs old `pull_sync` stays byte-identical with no caveat.

## Files

- `crates/blit-core/src/transfer_session/mod.rs` — `responder_finish` bind gate;
  `drive_source`/`source_send_half` accept-vs-dial selection; `drive_destination`/
  `destination_session` dial-vs-accept receive + `data_plane_host`.
- `crates/blit-core/src/transfer_session/data_plane.rs` —
  `accept_source_data_plane`, `dial_destination_data_plane` (+ its run handle).
- `crates/blit-core/src/remote/transfer/session_client.rs` — `run_pull_session`
  default carrier + `data_plane_host`; `PullSessionOptions.in_stream_bytes`.
- `crates/blit-core/tests/transfer_session_roles.rs` — pull data-plane
  single-stream invariance test.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — pull over the data
  plane (byte-identical, `!in_stream_carrier_used`, `data_plane_streams == 1`).

## Tests

- Roles suite: a DESTINATION-initiator / SOURCE-responder session over a real
  loopback TCP data plane (control frames on the in-process pair, as the otp-4b
  push data-plane test does) lands byte-identically and reports
  `data_plane_streams == Some(1)`, `!in_stream_carrier_used`.
- e2e (real daemon as SOURCE responder):
  - `pull_session_lands_bytes_over_the_data_plane` — default carrier is TCP;
    byte-identical dest; `!in_stream_carrier_used`.
  - `pull_session_lands_bytes_over_in_stream_carrier` — the `in_stream_bytes`
    fallback still lands byte-identically (otp-5a path stays live).
  - the existing `old_pull_and_session_produce_identical_trees_and_counts` A/B
    now runs the NEW arm over the data plane (converge-up bar).

Guard proof: forcing the SOURCE responder to grant no data plane (or forcing the
DESTINATION initiator onto the in-stream branch) makes
`pull_session_lands_bytes_over_the_data_plane` fail its `!in_stream_carrier_used`
assertion; restoring passes. The A/B byte-identity guards correctness.

## Known gaps (carried into otp-5b-2 / later)

- **Multi-stream / resize on the pull data plane**: otp-5b-2 (accept-based
  epoch-N socket; SOURCE responder proposes, DESTINATION initiator dials+acks).
  This slice is single-stream by the dial cap.
- **Mid-transfer cancel over the pull data plane**: the otp-4b-3 analog for pull
  (otp-5b-3 if a distinct guard is warranted; the control-lane CANCELLED framing
  is already role-agnostic).
- Mirror/filters otp-6; resume otp-7; fallback-carrier otp-8; delegated otp-9;
  cutover/deletion otp-10.
</content>
</invoke>

exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/TRANSFER_SESSION.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# TransferSession wire + session contract (otp-1)

**Status**: Active (contract; implementation lands otp-3..otp-10)
**Created**: 2026-07-05
**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)

This document is the authoritative contract for the single `Transfer`
RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
session"; this doc explains the state machine the proto cannot.

## Invariants

1. **One vocabulary, role-tagged.** Both wire directions carry the
   same frame type (`TransferFrame`). Which frames an end may send is
   determined by its ROLE (SOURCE or DESTINATION), never by whether
   it is the gRPC client or server. This is the structural form of
   the owner's invariant: there is no push-shaped or pull-shaped
   message set to diverge.
2. **Same build only (D-2026-07-05-2).** The first frame each way is
   `SessionHello{build_id, contract_version}`. Both ends compare for
   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
   naming both ids, then stream close. No negotiate-down, no advisory
   fields, no feature-capability bits — same build implies same
   features. `build_id` = `<crate version>+<git commit hash>`
   composed at compile time; `contract_version` is a belt-and-braces
   integer bumped on any wire-shape change (exact match required).
   Imprecise identities never false-match (otp-3 codex F1): a dirty
   tree composes `<sha>.dirty.<content hash>` (deterministic — only
   byte-identical dirty trees match), and a build without git
   identity composes `unknown.<per-compilation entropy>` (only the
   selfsame binary matches itself).
3. **Roles.** The initiator (the end that opened the RPC — a CLI
   client, or a daemon acting as delegated initiator) declares in
   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
   (always a daemon) takes the other role. All four
   initiator/role combinations run the identical state machine.
4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
   from live enumeration (immediate start — no buffered-enumeration
   phase in any direction). DESTINATION diffs incrementally against
   its own filesystem and streams need batches back. DESTINATION is
   authoritative for what it has; SOURCE is authoritative for what
   exists to send.
5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
   (whichever end holds DESTINATION) advertises its
   `CapacityProfile` at session open — in `SessionOpen` when the
   initiator is DESTINATION, in `SessionAccept` when the responder
   is. The byte SENDER (SOURCE) owns the live dial bounded by that
   profile. Absent/0 profile fields mean "unknown hardware value" —
   conservative defaults, never unlimited, and NEVER "old peer"
   (there are no old peers).
6. **One stream policy.** The data plane opens at the dial floor
   immediately; SOURCE shape-corrects the stream count upward via
   resize as the need list accumulates (the sf-2 mechanism —
   `TransferDial::propose_shape_resize` — now the only policy).
   SOURCE is the resize controller in every session.

## Phase state machine

```
INITIATOR                                RESPONDER
  |-- SessionHello ----------------------->|   (phase: HELLO)
  |<------------------------ SessionHello--|
  |     both verify build_id exact match; mismatch => SessionError + close
  |-- SessionOpen ------------------------>|   (phase: OPEN)
  |<---------------------- SessionAccept --|
  |     responder validates module/path/read-only/gate here;
  |     refusal is a SessionError, never a silent close
  |                                        |
  |==== from here the lanes are ROLES, not initiator/responder ====|
  |  (whichever end holds SOURCE sends source-lane frames,          |
  |   regardless of which end opened the RPC)                       |
  |                                                                 |
  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
  |  DEST streams:    NeedBatch* ... NeedComplete                  |
  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
  |                   frames when the in-stream carrier is chosen) |
  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
  |                                                                 |
  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
  |  `resume=true` is followed by DEST's BlockHashList for that     |
  |  file BEFORE SOURCE may send any byte of that file; stale or    |
  |  mismatched partials fall back to full-file transfer.           |
  |                                                                 |
  |  mirror: DEST computes deletions LOCALLY from the completed     |
  |  source manifest (filter-scoped, scan-complete-guarded) and     |
  |  executes them itself. No delete list crosses the wire.         |
  |                                                                 |
  |  CLOSING (role-directed, both initiator layouts):               |
  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
  |  then the INITIATOR closes the RPC stream.                      |
```

- Phase violations (a frame arriving in a phase where its role may
  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
  fail-fast, no tolerant parsing.
- `NeedComplete` is DESTINATION's promise that no further need
  batches follow (SOURCE may finish after flushing what was asked).
  It may be sent only after BOTH: the source's `ManifestComplete`
  has been received AND the destination has finished diffing every
  received manifest entry. Mirror deletions additionally require the
  scan-complete guard, as above.
- **Flow control is the transport's, deliberately:** manifest, need,
  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
  each end holds only bounded internal queues (the engine's existing
  batching — 128-entry manifest check chunks, need-list batcher).
  Nothing in the contract requires unbounded buffering of the peer's
  stream, and implementations must not introduce it.
- `TransferSummary` always travels DESTINATION → SOURCE (the end
  that wrote bytes and executed deletes is the end that can attest
  to them), then the initiator surfaces it to the operator.

## Frame set and field numbers

`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`

`TransferFrame.frame` oneof (field numbers frozen by this doc):

| # | frame | sender | phase |
|---|-------|--------|-------|
| 1 | `SessionHello` | both, first frame | HELLO |
| 2 | `SessionOpen` | initiator | OPEN |
| 3 | `SessionAccept` | responder | OPEN |
| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
| 10 | `FileData file_data` | SOURCE | in-stream carrier |
| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
| 14 | `BlockTransfer block` | SOURCE | resume |
| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
| 18 | `SourceDone source_done` | SOURCE | closing |
| 19 | `TransferSummary summary` | DESTINATION | closing |
| 20 | `SessionError error` | both | any |

Reused messages (`FileHeader`, `FileData`, `TarShard*`,
`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
existing shapes — the session reuses the engine's payload vocabulary
verbatim. New messages (`SessionHello`, `SessionOpen`,
`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
defined in the proto with their field numbers.

Deliberately absent: `PeerCapabilities` (same build = same
features), `spec_version` negotiation (the hello's exact match
replaces it), any delete list (mirror is destination-local), any
push/pull-specific message.

## Transport selection

- **TCP data plane (default):** the RESPONDER binds the listener and
  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
  dials (NAT/firewall reality — connection topology, not
  choreography). Byte direction on the sockets is set by role:
  SOURCE writes, DESTINATION reads.
  **`initial_streams` is an ACCEPT ceiling, not a dial order**
  (D-2026-06-20-1/-2 preserved): it is the number of epoch-0 accept
  slots the responder arms, computed as min(engine dial floor,
  DESTINATION's capacity ceiling). SOURCE — wherever it sits — owns
  the dial and may use fewer epoch-0 sockets than armed; unclaimed
  slots expire harmlessly. Growth beyond epoch 0 happens only via
  SOURCE-initiated resize (sf-2 shape correction / tuner), one armed
  accept per ADD epoch, exactly as ue-r2-2 built.
  **Socket auth, exact:** every epoch-0 socket opens with
  `session_token` (16 bytes) immediately followed by
  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
  `session_token` followed by that epoch's `sub_token` from the
  `DataPlaneResize` frame. Tokens are single-session; each armed
  accept slot admits exactly one socket (no replay within a
  session); armed slots that go unclaimed expire, as today's resize
  wiring already does. A socket presenting anything else is closed
  without response.
- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
  (operator `--force-grpc` diagnostics) or granted by the responder
  when it cannot bind a data plane (`SessionAccept` with no grant).
  Payload frames 9-15 ride the RPC itself. Same choreography, same
  planner decisions, different byte carrier.
  **Record grammar (fail-fast):** payload records on the
  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
  only `file_data` frames for that file may follow on the lane until
  the record completes; completion is inferred at exactly
  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
  `block` arriving early, or bytes overrunning `size`, is
  `PROTOCOL_VIOLATION`). Tar-shard records run
  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block
  records complete with `block_complete`. Payload records may begin
  only AFTER the source's `ManifestComplete` — this per-transport
  ordering rule applies identically to both roles and mirrors the
  design-4-proven fallback ordering, so manifest frames and payload
  records never interleave. DESTINATION-lane frames (need batches,
  acks, summary) are unaffected — they travel the other direction.
- **Local (in-process):** the identical session state machine runs
  with both roles in one process over an in-process frame channel —
  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
  file vs block) is planner-owned and reads workload shape +
  capability, never role/initiator/transport.

## Errors, cancel, stall

- `SessionError{code, message}` codes (plus both build ids on
  BUILD_MISMATCH):
  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
  or aborts says why before closing; operators never diagnose from a
  bare stream reset.
- `CancelJob` interop: the responder registers the session in
  `ActiveJobs` at OPEN (same transfer_id contract as today); the
  cancel token races the session exactly as w4-3 wired, and the
  peer receives `SessionError{CANCELLED}`.
- StallGuard, byte-accounting, and progress events (w6-1 contract)
  attach at the same boundaries they do today; the session emits the
  existing `DaemonEvent` payloads.

## What this replaces

At cutover (otp-10): `Push`, `PullSync`, and their message
choreographies are deleted from the proto and the tree; the four
per-direction drivers die with them; `DelegatedPull` shrinks to
trigger + progress relay (no payload bytes). Until then the old paths
stay live alongside the session as migration scaffolding.

Progress: otp-3 landed the role-parameterized drivers over the
in-process transport; **otp-4a** made the daemon serve `Transfer` for
real (runs `run_destination` as Responder; a client `run_source`s as
SOURCE initiator over gRPC, in-stream carrier) — the RPC no longer
returns `UNIMPLEMENTED`. The TCP data plane grant + resize land at
otp-4b; the daemon-as-SOURCE (pull-equivalent) layout at otp-5.

exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --stat --patch --decorate --src-prefix=a/ --dst-prefix=b/ e6a0b3b -- crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/src/transfer_session/mod.rs crates/blit-core/src/remote/transfer/session_client.rs crates/blit-core/tests/transfer_session_roles.rs crates/blit-daemon/src/service/transfer_session_e2e.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
commit e6a0b3b234d4f175082cb8c7b6a74cfb5940a61c (HEAD -> master)
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Mon Jul 6 14:24:45 2026 -0400

    otp-5b-1: single-stream SOURCE-responder data plane (transport/role decoupling)
    
    The TCP data plane was keyed to ROLE (DESTINATION binds+accepts+receives;
    SOURCE dials+sends) — a coincidence that holds only for push. otp-5b splits
    the two axes: the RESPONDER binds+accepts and the INITIATOR dials (NAT
    reality), while byte direction stays set by role. This slice (5b-1) adds the
    pull pair single-stream: a SOURCE responder accepts+sends, a DESTINATION
    initiator dials+receives. Resize (the accept-based epoch-N socket) is 5b-2.
    
    blit-core:
    - responder_finish binds a data plane for EITHER role when !in_stream_bytes
      (was DESTINATION-only); the bound listener travels in
      Negotiated.responder_data_plane and is consumed by whichever role's driver
      runs. receiver_capacity in the accept stays DESTINATION-only (the byte
      RECEIVER advertises; a DESTINATION initiator advertises in its own
      SessionOpen.receiver_capacity).
    - data_plane::accept_source_data_plane: accept the granted epoch-0 socket(s)
      off the bound listener, wrap each via DataPlaneSession::from_stream ->
      DataPlaneSink, and drive the SAME elastic send pipeline dial_source builds.
      Single-stream (resizable=false), so propose_resize returns None and no
      DataPlaneResize flows.
    - data_plane::dial_destination_data_plane + InitiatorReceivePlaneRun: dial the
      granted epoch-0 socket(s) and drain each via execute_receive_pipeline (the
      same NeedListSink strictness). DestRecvPlane enum tags the DESTINATION
      receive by connection role (Responder accept / Initiator dial); finish()
      reports the settled stream count either way.
    - source_send_half selects accept-vs-dial by whether it holds a bound listener
      or a received grant; destination_session selects dial-vs-accept the mirror
      way. A Resize on the single-stream pull data plane is a protocol violation.
    - DestinationSessionConfig gains data_plane_host (the initiator dials the
      responder's host), threaded through drive_destination/destination_session,
      symmetric with SourceSessionConfig.
    
    client: run_pull_session defaults to the TCP data plane (in_stream_bytes from
    PullSessionOptions, default false) with data_plane_host = endpoint host;
    PullSessionOptions gains the in_stream_bytes fallback knob.
    
    Tests (1519 -> 1521):
    - roles suite: pull_data_plane_single_stream_lands_bytes — DESTINATION
      initiator / SOURCE responder over a real loopback TCP data plane; byte-
      identical, data_plane_streams == Some(1), !in_stream_carrier_used.
    - e2e: pull_session_lands_bytes_over_the_data_plane (default TCP; the old
      in-stream-only pull test is split so the in-stream path stays covered via
      pull_session_lands_bytes_over_in_stream_carrier). The existing A/B vs old
      pull_sync now runs over the data plane (converge-up bar).
    Guard: reverting the SOURCE-responder bind (grant no data plane) makes both
    new data-plane tests fall back to in-stream and FAIL !in_stream_carrier_used;
    restored, all pass.
    
    Finding: .review/findings/otp-5b-source-responder-data-plane.md.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
---
 .../src/remote/transfer/session_client.rs          |  34 +++-
 .../blit-core/src/transfer_session/data_plane.rs   | 211 ++++++++++++++++++-
 crates/blit-core/src/transfer_session/mod.rs       | 223 ++++++++++++++-------
 crates/blit-core/tests/transfer_session_roles.rs   |  82 ++++++++
 .../src/service/transfer_session_e2e.rs            |  59 +++++-
 5 files changed, 512 insertions(+), 97 deletions(-)

diff --git a/crates/blit-core/src/remote/transfer/session_client.rs b/crates/blit-core/src/remote/transfer/session_client.rs
index a9f9389..f4d0738 100644
--- a/crates/blit-core/src/remote/transfer/session_client.rs
+++ b/crates/blit-core/src/remote/transfer/session_client.rs
@@ -10,9 +10,9 @@
 //! carried in `SessionOpen.initiator_role`, never a second code path.
 //!
 //! Not yet wired to CLI verbs — the verbs keep riding the old paths
-//! until the otp-10 cutover; today the parity tests drive this. push
-//! defaults to the TCP data plane (otp-4b); pull is in-stream only until
-//! otp-5b adds the SOURCE-responder data plane.
+//! until the otp-10 cutover; today the parity tests drive this. Both push
+//! (otp-4b) and pull (otp-5b) default to the TCP data plane; the in-stream
+//! carrier is the requested fallback either direction.
 
 use std::path::PathBuf;
 use std::sync::Arc;
@@ -120,6 +120,12 @@ pub struct PullSessionOptions {
     pub compare_mode: ComparisonMode,
     pub ignore_existing: bool,
     pub require_complete_scan: bool,
+    /// Force the in-stream byte carrier instead of the TCP data plane
+    /// (otp-5b). Default `false` = the SOURCE responder grants a data
+    /// plane and this DESTINATION initiator dials + receives over TCP
+    /// sockets; `true` is the diagnostics / unreachable data-plane
+    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
+    pub in_stream_bytes: bool,
 }
 
 impl Default for PullSessionOptions {
@@ -128,6 +134,7 @@ impl Default for PullSessionOptions {
             compare_mode: ComparisonMode::SizeMtime,
             ignore_existing: false,
             require_complete_scan: false,
+            in_stream_bytes: false,
         }
     }
 }
@@ -139,10 +146,12 @@ impl Default for PullSessionOptions {
 /// its module tree). Returns the [`DestinationOutcome`] this end
 /// computed (contract: the DESTINATION is the scorer).
 ///
-/// otp-5a rides the in-stream byte carrier: the SOURCE responder grants
-/// no TCP data plane yet (the transport/role decoupling that lets a
-/// SOURCE responder bind+grant lands at otp-5b), so `in_stream_bytes` is
-/// set to make the carrier explicit. Not wired to CLI verbs (otp-10).
+/// otp-5b: the default carrier is the TCP data plane — the SOURCE
+/// responder binds+grants+accepts sockets while sending, and this
+/// DESTINATION initiator dials + receives over them (the transport/role
+/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
+/// fallback (diagnostics / unreachable data plane). Not wired to CLI verbs
+/// (otp-10).
 pub async fn run_pull_session(
     endpoint: &RemoteEndpoint,
     dest_root: PathBuf,
@@ -159,10 +168,10 @@ pub async fn run_pull_session(
         compare_mode: options.compare_mode as i32,
         ignore_existing: options.ignore_existing,
         require_complete_scan: options.require_complete_scan,
-        // otp-5a is in-stream only (the SOURCE responder grants no data
-        // plane); set the flag so the carrier is explicit and stable if
-        // a data-plane grant is added at otp-5b.
-        in_stream_bytes: true,
+        // otp-5b: default to the TCP data plane; the SOURCE responder
+        // grants it in SessionAccept unless this asks for the in-stream
+        // fallback.
+        in_stream_bytes: options.in_stream_bytes,
         ..Default::default()
     };
 
@@ -177,6 +186,9 @@ pub async fn run_pull_session(
     let cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(open),
+        // The initiator dials the data plane on the same host it reached
+        // the control plane on (contract §Transport: initiator dials).
+        data_plane_host: Some(endpoint.host.clone()),
     };
     run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
 }
diff --git a/crates/blit-core/src/transfer_session/data_plane.rs b/crates/blit-core/src/transfer_session/data_plane.rs
index 521e4f7..d40e62a 100644
--- a/crates/blit-core/src/transfer_session/data_plane.rs
+++ b/crates/blit-core/src/transfer_session/data_plane.rs
@@ -8,12 +8,20 @@
 //! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
 //! deletes at cutover (otp-10), so nothing in this file calls into them.
 //!
-//! The RESPONDER (whichever end is DESTINATION for otp-4/-5) binds a
-//! listener, mints the tokens, grants them in `SessionAccept`, and
-//! accepts + receives; the INITIATOR (SOURCE here) dials, authenticates,
-//! and sends. Because the grant is issued before any manifest is seen,
-//! the zero-knowledge `initial_stream_proposal` is 1 — the session data
-//! plane always starts single-stream (otp-4b-1).
+//! Two orthogonal axes (otp-5b): the **connection role** — the RESPONDER
+//! binds+accepts, the INITIATOR dials (NAT reality) — and the **byte
+//! role** — the SOURCE sends, the DESTINATION receives. otp-4b wired the
+//! push pair (DESTINATION responder accepts+receives; SOURCE initiator
+//! dials+sends); otp-5b adds the pull pair (SOURCE responder accepts+
+//! sends via [`accept_source_data_plane`]; DESTINATION initiator dials+
+//! receives via [`dial_destination_data_plane`]). The byte machinery is
+//! shared — send is `DataPlaneSession`/`DataPlaneSink`/the elastic
+//! pipeline, receive is `execute_receive_pipeline` — only socket
+//! acquisition differs per byte role. Because the grant is issued before
+//! any manifest is seen, the zero-knowledge `initial_stream_proposal` is
+//! 1 — the session data plane always starts single-stream (otp-4b-1); the
+//! pull data plane stays single-stream through otp-5b-1 (resize is
+//! otp-5b-2).
 //!
 //! otp-4b-2 adds mid-transfer growth: the SOURCE owns a [`TransferDial`]
 //! (bounded by the receiver's advertised capacity) and drives the sf-2
@@ -43,7 +51,7 @@ use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
 use crate::remote::transfer::pipeline::execute_receive_pipeline;
 use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
 use crate::remote::transfer::socket::{
-    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
+    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
 };
 use crate::remote::transfer::source::TransferSource;
 use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
@@ -410,6 +418,99 @@ async fn authenticate_resize(
     }
 }
 
+// ---------------------------------------------------------------------------
+// Initiator (DESTINATION) — dial, receive (otp-5b-1)
+// ---------------------------------------------------------------------------
+
+/// Live handle to a DESTINATION **initiator** receive data plane
+/// (otp-5b-1, the pull direction): the initiator dials the granted
+/// epoch-0 socket(s) and drains each into the sink through the shared
+/// receive pipeline — the same byte machinery the DESTINATION responder
+/// uses, only the socket is dialed instead of accepted. Single-stream: no
+/// resize arming (otp-5b-2 adds the accept-based epoch-N socket + dial).
+/// [`Self::finish`] joins the workers for the aggregated write outcome +
+/// settled stream count.
+pub(super) struct InitiatorReceivePlaneRun {
+    receives: JoinSet<Result<SinkOutcome>>,
+    streams: usize,
+}
+
+/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
+/// socket. `host` is the responder's host (the initiator reached the
+/// control plane there; the data plane rides the same host on the granted
+/// port — contract §Transport: the initiator always dials). Each worker
+/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
+/// in-stream carrier applies inline).
+pub(super) async fn dial_destination_data_plane(
+    host: &str,
+    grant: &DataPlaneGrant,
+    sink: Arc<dyn TransferSink>,
+) -> Result<InitiatorReceivePlaneRun> {
+    let initial = grant.initial_streams.max(1) as usize;
+    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
+    let mut handshake = grant.session_token.clone();
+    handshake.extend_from_slice(&grant.epoch0_sub_token);
+    let addr = format!("{host}:{}", grant.tcp_port);
+
+    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
+    let mut streams = 0usize;
+    for _ in 0..initial {
+        // `dial_data_plane` connects, applies the data-socket policy, and
+        // writes the handshake credential — the same bounded dial the
+        // SOURCE initiator uses (design-3: one owner for every client-side
+        // data-plane dial, both directions).
+        let socket = dial_data_plane(&addr, &handshake, None)
+            .await
+            .map_err(|err| dp_fault(format!("dialing session data plane (receive): {err:#}")))?;
+        streams += 1;
+        spawn_receive(&mut receives, socket, &sink);
+    }
+    Ok(InitiatorReceivePlaneRun { receives, streams })
+}
+
+impl InitiatorReceivePlaneRun {
+    /// Join every receive worker for the aggregated write totals. A worker
+    /// error (receive failure / stall) surfaces here; each drains to its
+    /// socket's END record on a clean transfer.
+    async fn finish(mut self) -> Result<ReceiveTotals> {
+        let mut total = SinkOutcome::default();
+        while let Some(joined) = self.receives.join_next().await {
+            let outcome =
+                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
+            total.files_written += outcome.files_written;
+            total.bytes_written += outcome.bytes_written;
+        }
+        Ok(ReceiveTotals {
+            outcome: total,
+            streams: self.streams,
+        })
+    }
+}
+
+/// The DESTINATION end's receive data plane, tagged by connection role.
+/// Both drain socket bytes into the sink through the same receive
+/// pipeline; they differ only in how sockets are obtained (accept vs dial)
+/// and whether resize is armable (push only, otp-4b-2).
+pub(super) enum DestRecvPlane {
+    /// DESTINATION **responder** (push, otp-4b): accepts sockets, resize-
+    /// armable via the control loop.
+    Responder(ResponderDataPlaneRun),
+    /// DESTINATION **initiator** (pull, otp-5b-1): dialed single-stream
+    /// receive, no resize.
+    Initiator(InitiatorReceivePlaneRun),
+}
+
+impl DestRecvPlane {
+    /// Drain the data plane to completion and report the settled stream
+    /// count + write outcome (the DESTINATION is the scorer).
+    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
+        match self {
+            DestRecvPlane::Responder(run) => run.finish().await,
+            DestRecvPlane::Initiator(run) => run.finish().await,
+        }
+    }
+}
+
 // ---------------------------------------------------------------------------
 // Initiator (SOURCE) — dial, authenticate, send, resize
 // ---------------------------------------------------------------------------
@@ -447,6 +548,13 @@ pub(super) struct SourceDataPlane {
     tcp_port: u32,
     session_token: Vec<u8>,
     pool: Arc<BufferPool>,
+    /// Whether this data plane grows mid-transfer via `DataPlaneResize`.
+    /// True for the SOURCE **initiator** (push, otp-4b-2: it dials each
+    /// epoch-N socket on the ack). False for the SOURCE **responder**
+    /// (pull, otp-5b-1): the accept-based epoch-N socket + ack→accept
+    /// choreography is otp-5b-2, so this slice stays single-stream and
+    /// `propose_resize` returns `None` regardless of the need list.
+    resizable: bool,
 }
 
 /// Dial the granted data plane and start the elastic send pipeline.
@@ -530,6 +638,90 @@ pub(super) async fn dial_source_data_plane(
         tcp_port: grant.tcp_port,
         session_token: grant.session_token.clone(),
         pool,
+        resizable: true,
+    })
+}
+
+/// Accept the granted epoch-0 socket(s) off a bound responder listener and
+/// start the elastic SEND pipeline over them — the SOURCE **responder**
+/// half of the pull data plane (otp-5b-1). Symmetric with
+/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
+/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
+/// socket acquisition differs (accept here, dial there).
+/// `DataPlaneSession::from_stream` builds a send session from an already-
+/// accepted socket — the same primitive the old `pull_sync` daemon-send
+/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
+/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
+/// wherever it initiates). Single-stream: `resizable: false`, so no
+/// `DataPlaneResize` is ever proposed on the pull data plane in this slice.
+pub(super) async fn accept_source_data_plane(
+    bound: ResponderDataPlane,
+    receiver_capacity: Option<&CapacityProfile>,
+    source: Arc<dyn TransferSource>,
+) -> Result<SourceDataPlane> {
+    let initial = bound.initial_streams.max(1) as usize;
+    // The byte sender's dial, bounded by the receiver's advertised
+    // capacity; seed the live count to the granted epoch-0 streams. Growth
+    // is disabled below (resizable=false), so the count stays here.
+    let dial = TransferDial::conservative_within(receiver_capacity).shared();
+    dial.set_negotiated_streams(initial);
+
+    // Epoch-0 credential the dialing DESTINATION presents:
+    // session_token ‖ epoch0_sub_token (contract §Transport).
+    let mut epoch0 = bound.session_token.clone();
+    epoch0.extend_from_slice(&bound.epoch0_sub_token);
+
+    let pool = Arc::new(BufferPool::for_data_plane(
+        dial.chunk_bytes(),
+        dial.ceiling_max_streams().max(1),
+    ));
+    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
+    for _ in 0..initial {
+        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
+        let session = DataPlaneSession::from_stream(
+            socket,
+            false,
+            dial.chunk_bytes(),
+            dial.prefetch_count(),
+            Arc::clone(&pool),
+        )
+        .await;
+        sinks.push(Arc::new(DataPlaneSink::new(
+            session,
+            Arc::clone(&source),
+            PathBuf::new(),
+        )));
+    }
+
+    let prefetch = dial.prefetch_count().max(1);
+    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
+    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
+    let pipe_source = Arc::clone(&source);
+    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
+        execute_sink_pipeline_elastic(
+            pipe_source,
+            sinks,
+            payload_rx,
+            prefetch,
+            None,
+            Some(control_rx),
+        )
+        .await
+    }));
+    Ok(SourceDataPlane {
+        payload_tx: Some(payload_tx),
+        control_tx,
+        pipeline: Some(pipeline),
+        dial,
+        source,
+        // Accept-based: this end never dials an epoch-N socket, so the
+        // dial-target fields are unused (add_stream is unreachable while
+        // resizable is false).
+        host: String::new(),
+        tcp_port: 0,
+        session_token: bound.session_token,
+        pool,
+        resizable: false,
     })
 }
 
@@ -551,6 +743,11 @@ impl SourceDataPlane {
         needed_bytes: u64,
         needed_count: usize,
     ) -> Result<Option<PendingResize>> {
+        // A non-resizable data plane (the SOURCE responder, otp-5b-1)
+        // never grows: the accept-based epoch-N socket is otp-5b-2.
+        if !self.resizable {
+            return Ok(None);
+        }
         let desired =
             initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
                 as usize;
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 0ecb287..21ca670 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -130,6 +130,14 @@ pub struct SourceSessionConfig {
 pub struct DestinationSessionConfig {
     pub hello: HelloConfig,
     pub endpoint: SessionEndpoint,
+    /// Host to dial the granted TCP data plane on when this end is the
+    /// **initiator** (pull-equivalent, otp-5b): the DESTINATION initiator
+    /// dials the SOURCE responder's granted sockets on the same host it
+    /// reached the control plane on (contract §Transport: the initiator
+    /// always dials). `None` — or a DESTINATION responder, which binds
+    /// rather than dials — falls back to the in-stream carrier. Symmetric
+    /// with [`SourceSessionConfig::data_plane_host`].
+    pub data_plane_host: Option<String>,
 }
 
 /// A session-terminating fault: either end refusing, aborting, or
@@ -502,16 +510,18 @@ async fn responder_finish(
         },
         None => None,
     };
-    // Data plane (otp-4b): a DESTINATION responder binds a TCP
-    // listener and grants it, unless the initiator requested the
-    // in-stream carrier or the bind fails (grant-less accept ⇒
-    // in-stream fallback). A SOURCE responder (otp-5, daemon-send)
-    // grants no data plane in otp-5a — the transport/role decoupling
-    // that lets a SOURCE responder bind+grant lands at otp-5b.
-    let responder_data_plane = if local_role == TransferRole::Destination && !open.in_stream_bytes {
-        data_plane::prepare_responder_data_plane().await
-    } else {
+    // Data plane (otp-4b/5b): a responder binds a TCP listener and grants
+    // it, unless the initiator requested the in-stream carrier or the bind
+    // fails (grant-less accept ⇒ in-stream fallback). This is role-agnostic
+    // (otp-5b): the RESPONDER binds+accepts and the INITIATOR dials, while
+    // byte direction is set by role — a DESTINATION responder accepts+
+    // receives (push, otp-4b), a SOURCE responder accepts+sends (pull,
+    // otp-5b). The bound listener travels in `Negotiated.responder_data_plane`
+    // and is consumed by whichever role's driver runs.
+    let responder_data_plane = if open.in_stream_bytes {
         None
+    } else {
+        data_plane::prepare_responder_data_plane().await
     };
     let accept = SessionAccept {
         // The byte RECEIVER advertises capacity at session
@@ -675,7 +685,7 @@ pub async fn run_source(
     drive_source(
         cfg.plan_options,
         cfg.data_plane_host,
-        &negotiated,
+        negotiated,
         transport,
         source,
     )
@@ -690,10 +700,14 @@ pub async fn run_source(
 async fn drive_source(
     plan_options: PlanOptions,
     data_plane_host: Option<String>,
-    negotiated: &Negotiated,
+    mut negotiated: Negotiated,
     transport: FrameTransport,
     source: Arc<dyn TransferSource>,
 ) -> Result<TransferSummary> {
+    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
+    // its send sockets on; a SOURCE initiator (push) has none and dials the
+    // grant it received instead. Take it here so the send half owns it.
+    let responder_data_plane = negotiated.responder_data_plane.take();
     let (mut tx, rx) = transport.split();
     let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
     // Set by the send half the moment ManifestComplete goes out. On
@@ -715,7 +729,8 @@ async fn drive_source(
     match source_send_half(
         plan_options,
         data_plane_host.as_deref(),
-        negotiated,
+        &negotiated,
+        responder_data_plane,
         &mut tx,
         source,
         sent,
@@ -836,6 +851,7 @@ async fn source_send_half(
     plan_options: PlanOptions,
     data_plane_host: Option<&str>,
     negotiated: &Negotiated,
+    responder_data_plane: Option<data_plane::ResponderDataPlane>,
     tx: &mut Box<dyn FrameTx>,
     source: Arc<dyn TransferSource>,
     sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
@@ -845,31 +861,49 @@ async fn source_send_half(
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut need_complete = false;
 
-    // Data plane (otp-4b): dial the granted TCP sockets up front —
-    // BEFORE streaming the manifest — so the destination's accept loop
-    // (armed the moment it sent SessionAccept) sees the connections
-    // promptly rather than waiting out its bounded-accept timeout while
-    // a long manifest streams. The sockets sit idle (keepalive covers
-    // that) until payloads are queued below. `None` = the in-stream
-    // carrier (fallback), which needs no early setup.
-    let mut data_plane = match &negotiated.accept.data_plane {
-        Some(grant) => {
-            let host = data_plane_host.ok_or_else(|| {
-                eyre::Report::new(SessionFault::internal(
-                    "responder granted a TCP data plane but this initiator has no host to dial",
-                ))
-            })?;
-            Some(
-                data_plane::dial_source_data_plane(
-                    host,
-                    grant,
-                    negotiated.accept.receiver_capacity.as_ref(),
-                    Arc::clone(&source),
-                )
-                .await?,
+    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
+    // streaming the manifest — so the peer sees the connections promptly
+    // rather than waiting out a bounded-accept/connect timeout while a long
+    // manifest streams. Which end connects depends on connection role
+    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
+    // listener; a SOURCE **initiator** (push) dials the grant it received.
+    // Byte direction is the same either way (SOURCE sends), so both yield a
+    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
+    // in-stream carrier (fallback), which needs no early setup.
+    let mut data_plane = match responder_data_plane {
+        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
+        // initiator advertised its capacity in the open (byte RECEIVER
+        // advertises, wherever it initiates); the accept plane is single-
+        // stream (otp-5b-1).
+        Some(bound) => Some(
+            data_plane::accept_source_data_plane(
+                bound,
+                negotiated.open.receiver_capacity.as_ref(),
+                Arc::clone(&source),
             )
-        }
-        None => None,
+            .await?,
+        ),
+        // SOURCE initiator (push, otp-4b): dial the grant if the responder
+        // granted a data plane; else in-stream.
+        None => match &negotiated.accept.data_plane {
+            Some(grant) => {
+                let host = data_plane_host.ok_or_else(|| {
+                    eyre::Report::new(SessionFault::internal(
+                        "responder granted a TCP data plane but this initiator has no host to dial",
+                    ))
+                })?;
+                Some(
+                    data_plane::dial_source_data_plane(
+                        host,
+                        grant,
+                        negotiated.accept.receiver_capacity.as_ref(),
+                        Arc::clone(&source),
+                    )
+                    .await?,
+                )
+            }
+            None => None,
+        },
     };
 
     // sf-2 shape correction (otp-4b-2): running totals of the need list,
@@ -1471,7 +1505,13 @@ pub async fn run_destination(
         },
     };
 
-    drive_destination(&mut transport, negotiated, &dst_root).await
+    drive_destination(
+        &mut transport,
+        negotiated,
+        &dst_root,
+        cfg.data_plane_host.as_deref(),
+    )
+    .await
 }
 
 /// The DESTINATION session body: run the diff/receive loop and map a
@@ -1482,8 +1522,9 @@ async fn drive_destination(
     transport: &mut FrameTransport,
     negotiated: Negotiated,
     dst_root: &Path,
+    data_plane_host: Option<&str>,
 ) -> Result<DestinationOutcome> {
-    match destination_session(transport, negotiated, dst_root).await {
+    match destination_session(transport, negotiated, dst_root, data_plane_host).await {
         Ok(outcome) => Ok(outcome),
         Err(report) => {
             let mut fault = fault_from_report(report);
@@ -1553,7 +1594,9 @@ pub async fn run_responder(
                     }
                 },
             };
-            let outcome = drive_destination(&mut transport, negotiated, &dst_root).await?;
+            // A DESTINATION responder (push) binds+accepts its receive
+            // sockets — it never dials, so it needs no data-plane host.
+            let outcome = drive_destination(&mut transport, negotiated, &dst_root, None).await?;
             Ok(ResponderOutcome::Destination(outcome))
         }
         // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
@@ -1585,10 +1628,11 @@ pub async fn run_responder(
                 }
             };
             // The SOURCE owns its planner knobs; a daemon-served source
-            // has no client-supplied ones (§Transport selection). otp-5a
-            // is in-stream only, so there is no data-plane host to dial.
+            // has no client-supplied ones (§Transport selection). A SOURCE
+            // responder binds+accepts its send sockets (otp-5b) — it never
+            // dials, so it needs no data-plane host.
             let summary =
-                drive_source(PlanOptions::default(), None, &negotiated, transport, source).await?;
+                drive_source(PlanOptions::default(), None, negotiated, transport, source).await?;
             Ok(ResponderOutcome::Source(summary))
         }
         TransferRole::Unspecified => Err(notify_and_wrap(
@@ -1609,6 +1653,7 @@ async fn destination_session(
     transport: &mut FrameTransport,
     negotiated: Negotiated,
     dst_root: &Path,
+    data_plane_host: Option<&str>,
 ) -> Result<DestinationOutcome> {
     let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
         .unwrap_or(ComparisonMode::Unspecified);
@@ -1651,31 +1696,56 @@ async fn destination_session(
     let mut granted: HashSet<String> = HashSet::new();
     let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
 
-    // Data plane (otp-4b): when the responder granted a TCP data plane,
-    // payload bytes arrive on sockets (not the control lane). Arm the
-    // accept+receive task NOW — concurrent with the diff loop below, and
-    // before the source dials — so the connections are accepted promptly.
-    // The NeedListSink gives the socket receive the same need-list
-    // strictness the in-stream control loop applies inline. AbortOnDrop
-    // bounds it to this future: a control-lane fault that returns from
-    // this fn aborts the receive task instead of leaking it.
-    // `resize_live` tracks the stream count this end has granted (epoch-0
-    // plus each accepted resize ADD); `resize_ceiling` is the receiver's
-    // advertised max_streams, the cumulative bound a resize may not cross.
-    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
-        match negotiated.responder_data_plane {
-            Some(rdp) => {
-                let initial = rdp.initial_streams() as usize;
-                let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
-                    Arc::clone(&sink) as Arc<dyn TransferSink>,
-                    Arc::clone(&outstanding),
-                ));
-                let run = rdp.spawn(recv_sink);
-                let ceiling = run.ceiling;
-                (Some(run), initial, ceiling)
+    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
+    // bytes arrive on sockets (not the control lane). Set it up NOW —
+    // concurrent with the diff loop below, and before the peer sends — so
+    // the connections are established promptly. Which end connects depends
+    // on connection role (otp-5b): a DESTINATION **responder** (push)
+    // accepts sockets off its bound listener; a DESTINATION **initiator**
+    // (pull) dials the grant it received on `data_plane_host`. Byte
+    // direction is the same either way (DESTINATION receives). The
+    // NeedListSink gives the socket receive the same need-list strictness
+    // the in-stream control loop applies inline; AbortOnDrop (inside the
+    // responder run) bounds the accept task to this future. `resize_live`
+    // tracks the stream count this end has granted (epoch-0 plus each
+    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
+    // max_streams — both meaningful only for the resize-armable responder
+    // path (push); the pull initiator path is single-stream (otp-5b-1).
+    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
+        Arc::clone(&sink) as Arc<dyn TransferSink>,
+        Arc::clone(&outstanding),
+    ));
+    let (mut data_plane_recv, mut resize_live, resize_ceiling) = match negotiated
+        .responder_data_plane
+    {
+        // DESTINATION responder (push, otp-4b): accept + receive.
+        Some(rdp) => {
+            let initial = rdp.initial_streams() as usize;
+            let run = rdp.spawn(recv_sink);
+            let ceiling = run.ceiling;
+            (
+                Some(data_plane::DestRecvPlane::Responder(run)),
+                initial,
+                ceiling,
+            )
+        }
+        // DESTINATION initiator (pull, otp-5b): dial + receive when the
+        // SOURCE responder granted a data plane and we have a host to
+        // dial; otherwise the in-stream carrier.
+        None => match (&negotiated.accept.data_plane, data_plane_host) {
+            (Some(grant), Some(host)) => {
+                let run = data_plane::dial_destination_data_plane(host, grant, recv_sink).await?;
+                // Single-stream (otp-5b-1): no resize is accepted, so
+                // the ceiling stays 0 and a Resize frame is a violation.
+                (
+                    Some(data_plane::DestRecvPlane::Initiator(run)),
+                    0usize,
+                    0usize,
+                )
             }
-            None => (None, 0usize, 0usize),
-        };
+            _ => (None, 0usize, 0usize),
+        },
+    };
 
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut needed_paths: Vec<String> = Vec::new();
@@ -1802,9 +1872,24 @@ async fn destination_session(
                 // and ack so the SOURCE dials the epoch-N socket. Only ADD
                 // occurs on the session (REMOVE is a tuner concern, future
                 // work); anything else fails fast.
-                let run = data_plane_recv.as_ref().ok_or_else(|| {
-                    violation("DataPlaneResize on a session with no data plane".into())
-                })?;
+                let run = match data_plane_recv.as_ref() {
+                    Some(data_plane::DestRecvPlane::Responder(run)) => run,
+                    // The pull data plane is single-stream (otp-5b-1): the
+                    // SOURCE responder never proposes a resize, so one here
+                    // is a protocol violation (otp-5b-2 adds the accept-based
+                    // epoch-N socket + dial).
+                    Some(data_plane::DestRecvPlane::Initiator(_)) => {
+                        return Err(violation(
+                            "DataPlaneResize on the single-stream pull data plane (otp-5b-1)"
+                                .into(),
+                        ))
+                    }
+                    None => {
+                        return Err(violation(
+                            "DataPlaneResize on a session with no data plane".into(),
+                        ))
+                    }
+                };
                 let op = DataPlaneResizeOp::try_from(resize.op)
                     .unwrap_or(DataPlaneResizeOp::Unspecified);
                 if op != DataPlaneResizeOp::Add {
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
index 0d74277..4eccff3 100644
--- a/crates/blit-core/tests/transfer_session_roles.rs
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -125,6 +125,7 @@ async fn run_session(
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: dest_endpoint,
+        data_plane_host: None,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
@@ -354,6 +355,7 @@ async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
+        data_plane_host: None,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root.clone()));
@@ -384,6 +386,81 @@ async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
     assert_trees_identical(&src_root, &dst_root);
 }
 
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn pull_data_plane_single_stream_lands_bytes() {
+    // otp-5b-1: the transport/role decoupling in the PULL direction — the
+    // mirror of the push data-plane test above. Here the DESTINATION is the
+    // *initiator* (dials + receives) and the SOURCE is the *responder*
+    // (binds + accepts + sends). Control frames ride the in-process
+    // transport; the data-plane socket rides loopback TCP (the SOURCE
+    // responder binds 0.0.0.0:0, the DESTINATION initiator dials
+    // 127.0.0.1). Single-stream by construction: the SOURCE responder's
+    // dial is non-resizable, so no `DataPlaneResize` flows (otp-5b-2 adds
+    // the accept-based epoch-N socket).
+    let tmp = tempfile::tempdir().unwrap();
+    let src_root = tmp.path().join("src");
+    let dst_root = tmp.path().join("dst");
+    std::fs::create_dir_all(&src_root).unwrap();
+    std::fs::create_dir_all(&dst_root).unwrap();
+    write_tree(
+        &src_root,
+        &[
+            ("a.txt", b"alpha".to_vec(), 1_600_000_001),
+            ("empty.bin", b"".to_vec(), 1_600_000_002),
+            ("dir/b.log", b"beta beta beta".to_vec(), 1_600_000_003),
+            ("dir/deep/c.dat", b"gamma-content".to_vec(), 1_600_000_004),
+        ],
+    );
+
+    // DESTINATION initiator; SOURCE responder — the roles flipped from the
+    // push data-plane test, the data plane following connection role.
+    let open = SessionOpen {
+        initiator_role: TransferRole::Destination as i32,
+        compare_mode: ComparisonMode::SizeMtime as i32,
+        in_stream_bytes: false,
+        ..Default::default()
+    };
+    let source_cfg = SourceSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
+        plan_options: PlanOptions::default(),
+        data_plane_host: None, // a responder never dials
+    };
+    let dest_cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(open), // dials + receives
+        data_plane_host: Some("127.0.0.1".into()),
+    };
+    let (a, b) = in_process_pair();
+    let source = Arc::new(FsTransferSource::new(src_root.clone()));
+    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
+        tokio::join!(
+            run_source(source_cfg, a, source),
+            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
+        )
+    })
+    .await
+    .expect("session run timed out");
+
+    let summary = source_result.expect("source responder succeeds");
+    let outcome = dest_result.expect("destination initiator succeeds");
+    assert!(
+        !summary.in_stream_carrier_used,
+        "the pull data plane must ride TCP, not the in-stream carrier"
+    );
+    assert_eq!(
+        summary, outcome.summary,
+        "both ends must hold the same summary"
+    );
+    assert_eq!(outcome.summary.files_transferred, 4);
+    assert_eq!(
+        outcome.data_plane_streams,
+        Some(1),
+        "otp-5b-1 pull is single-stream (no resize until otp-5b-2)"
+    );
+    assert_trees_identical(&src_root, &dst_root);
+}
+
 #[tokio::test]
 async fn preserves_mtime_on_streamed_files() {
     // Not part of the role matrix — pins that the file-record write
@@ -447,6 +524,7 @@ async fn build_mismatch_refused_under_both_initiators() {
                 contract_version: CONTRACT_VERSION,
             },
             endpoint: dest_endpoint,
+            data_plane_host: None,
         };
         let (a, b) = in_process_pair();
         let source = Arc::new(FsTransferSource::new(src_root.clone()));
@@ -502,6 +580,7 @@ async fn contract_version_mismatch_is_refused() {
             contract_version: CONTRACT_VERSION + 1,
         },
         endpoint: SessionEndpoint::Responder,
+        data_plane_host: None,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
@@ -542,6 +621,7 @@ async fn mirror_request_is_refused_until_its_slice_lands() {
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
+        data_plane_host: None,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
@@ -593,6 +673,7 @@ async fn payload_record_before_manifest_complete_is_protocol_violation() {
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
+        data_plane_host: None,
     };
     let (mut peer, dest_transport) = in_process_pair();
     let dest = tokio::spawn(run_destination(
@@ -825,6 +906,7 @@ async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
+        data_plane_host: None,
     };
     let (mut peer, dest_transport) = in_process_pair();
     let dest = tokio::spawn(run_destination(
diff --git a/crates/blit-daemon/src/service/transfer_session_e2e.rs b/crates/blit-daemon/src/service/transfer_session_e2e.rs
index fd3da5f..8d58b2f 100644
--- a/crates/blit-daemon/src/service/transfer_session_e2e.rs
+++ b/crates/blit-daemon/src/service/transfer_session_e2e.rs
@@ -17,11 +17,15 @@
 //!   is NEWER than the source is SKIPPED (the data-safe, pull-style
 //!   converged behavior — see the finding doc's compare decision).
 //!
-//! otp-5a adds the pull-equivalent (roles flipped): the client initiates
+//! otp-5a/5b add the pull-equivalent (roles flipped): the client initiates
 //! as DESTINATION and the daemon streams its module tree as the SOURCE
-//! Responder over the in-stream carrier. Those tests pin a byte-identical
-//! landing + A/B parity vs old `pull_sync`, proving the one served RPC
-//! handles both directions by the declared role, not a second code path.
+//! Responder. otp-5b makes the default carrier the TCP data plane too — the
+//! daemon (SOURCE responder) binds+grants+accepts sockets while sending and
+//! the client (DESTINATION initiator) dials + receives — with the in-stream
+//! carrier as the requested fallback. Those tests pin a byte-identical
+//! landing over both carriers + A/B parity vs old `pull_sync`, proving the
+//! one served RPC handles both directions by the declared role, not a
+//! second code path.
 //!
 //! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
 //! `BlitService` on loopback + a real client. Only in-crate tests can
@@ -570,12 +574,13 @@ async fn same_size_newer_destination_is_skipped_not_clobbered() {
 // ---------------------------------------------------------------------------
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn pull_session_lands_bytes_and_scores_them() {
+async fn pull_session_lands_bytes_over_the_data_plane() {
     // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
-    // initiates as DESTINATION and the daemon streams its module tree
-    // (otp-5a). The SOURCE responder grants no data plane, so the carrier
-    // is the in-stream fallback. `dest_root` here is the module (source)
-    // root — the harness field name is push-oriented.
+    // initiates as DESTINATION and the daemon streams its module tree. With
+    // otp-5b the default carrier is the TCP data plane — the daemon (SOURCE
+    // responder) binds+grants+accepts sockets while sending, and the client
+    // (DESTINATION initiator) dials + receives over them. `dest_root` here
+    // is the module (source) root — the harness field name is push-oriented.
     let daemon = Daemon::start(false).await;
     write_tree(&daemon.dest_root, &small_tree());
 
@@ -596,9 +601,43 @@ async fn pull_session_lands_bytes_and_scores_them() {
             .map(|(_, c, _)| c.len() as u64)
             .sum::<u64>()
     );
+    assert!(
+        !outcome.summary.in_stream_carrier_used,
+        "otp-5b pull default rides the TCP data plane, not the in-stream carrier"
+    );
+    assert_eq!(
+        outcome.data_plane_streams,
+        Some(1),
+        "otp-5b-1 pull is single-stream (no resize until otp-5b-2)"
+    );
+    assert_trees_identical(&daemon.dest_root, dest.path());
+    daemon.stop().await;
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn pull_session_lands_bytes_over_in_stream_carrier() {
+    // The in-stream carrier is the pull fallback (diagnostics / unreachable
+    // data plane). Requesting it must still land bytes byte-identically and
+    // score them — the otp-5a path stays live under otp-5b.
+    let daemon = Daemon::start(false).await;
+    write_tree(&daemon.dest_root, &small_tree());
+
+    let dest = tempfile::tempdir().unwrap();
+    let outcome = run_pull_session(
+        &daemon.endpoint,
+        dest.path().to_path_buf(),
+        PullSessionOptions {
+            in_stream_bytes: true,
+            ..PullSessionOptions::default()
+        },
+    )
+    .await
+    .expect("in-stream session pull succeeds");
+
+    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
     assert!(
         outcome.summary.in_stream_carrier_used,
-        "otp-5a pull rides the in-stream carrier (no SOURCE-responder data plane yet)"
+        "an in_stream_bytes request rides the in-stream carrier"
     );
     assert_trees_identical(&daemon.dest_root, dest.path());
     daemon.stop().await;

codex
The plan and finding confirm the intended slice is strictly single-stream pull over TCP, with resize deliberately deferred. I’m now reading the live functions with line numbers and comparing the pre/post behavior around push data-plane setup, resize handling, and summary accounting.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1,260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '260,560p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Session-side TCP data-plane orchestration (otp-4b).
     2	//!
     3	//! The unified session reuses blit-core's data-plane byte plumbing —
     4	//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
     5	//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
     6	//! its OWN choreography here. The push-specific bind/arm/accept loop
     7	//! (`blit-daemon` push service) and the multi-stream send driver
     8	//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
     9	//! deletes at cutover (otp-10), so nothing in this file calls into them.
    10	//!
    11	//! Two orthogonal axes (otp-5b): the **connection role** — the RESPONDER
    12	//! binds+accepts, the INITIATOR dials (NAT reality) — and the **byte
    13	//! role** — the SOURCE sends, the DESTINATION receives. otp-4b wired the
    14	//! push pair (DESTINATION responder accepts+receives; SOURCE initiator
    15	//! dials+sends); otp-5b adds the pull pair (SOURCE responder accepts+
    16	//! sends via [`accept_source_data_plane`]; DESTINATION initiator dials+
    17	//! receives via [`dial_destination_data_plane`]). The byte machinery is
    18	//! shared — send is `DataPlaneSession`/`DataPlaneSink`/the elastic
    19	//! pipeline, receive is `execute_receive_pipeline` — only socket
    20	//! acquisition differs per byte role. Because the grant is issued before
    21	//! any manifest is seen, the zero-knowledge `initial_stream_proposal` is
    22	//! 1 — the session data plane always starts single-stream (otp-4b-1); the
    23	//! pull data plane stays single-stream through otp-5b-1 (resize is
    24	//! otp-5b-2).
    25	//!
    26	//! otp-4b-2 adds mid-transfer growth: the SOURCE owns a [`TransferDial`]
    27	//! (bounded by the receiver's advertised capacity) and drives the sf-2
    28	//! shape correction — as the need list accumulates it re-runs the shape
    29	//! table and proposes `DataPlaneResize{ADD}` (one stream per epoch) on
    30	//! the control lane; the DESTINATION arms the credential, replies
    31	//! `DataPlaneResizeAck`, and accepts one more socket; the SOURCE dials
    32	//! the epoch-N socket and hands it to the running elastic pipeline via
    33	//! [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
    34	//! still future work — otp-4b-2 moves only the stream count.
    35	
    36	use std::collections::HashSet;
    37	use std::path::{Path, PathBuf};
    38	use std::sync::{Arc, Mutex as StdMutex};
    39	
    40	use async_trait::async_trait;
    41	use eyre::Result;
    42	use tokio::io::AsyncReadExt;
    43	use tokio::net::{TcpListener, TcpStream};
    44	use tokio::sync::mpsc;
    45	use tokio::task::JoinSet;
    46	
    47	use crate::buffer::BufferPool;
    48	use crate::engine::{initial_stream_proposal, local_receiver_capacity, TransferDial};
    49	use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
    50	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    51	use crate::remote::transfer::pipeline::execute_receive_pipeline;
    52	use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
    53	use crate::remote::transfer::socket::{
    54	    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    55	};
    56	use crate::remote::transfer::source::TransferSource;
    57	use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    58	use crate::remote::transfer::{
    59	    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
    60	    SUB_TOKEN_LEN,
    61	};
    62	
    63	use super::SessionFault;
    64	
    65	/// The set of granted-but-not-yet-received needs, shared between the
    66	/// destination's control loop (which inserts each path before sending
    67	/// its `NeedBatch`) and the data-plane receive (which claims each path
    68	/// as its payload lands). Completion is an empty set — the same signal
    69	/// the in-stream carrier uses via its inline `outstanding.remove`.
    70	pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;
    71	
    72	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    73	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    74	}
    75	
    76	// ---------------------------------------------------------------------------
    77	// Responder (DESTINATION) — bind, grant, accept, receive
    78	// ---------------------------------------------------------------------------
    79	
    80	/// A bound data-plane listener plus the credentials the responder
    81	/// advertises in its `SessionAccept`. Held by the responder driver
    82	/// across the handshake so the accept loop can run after establish.
    83	pub(super) struct ResponderDataPlane {
    84	    listener: TcpListener,
    85	    session_token: Vec<u8>,
    86	    epoch0_sub_token: Vec<u8>,
    87	    initial_streams: u32,
    88	    port: u16,
    89	}
    90	
    91	/// Bind a data-plane listener and mint credentials for the grant. Any
    92	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
    93	/// issues a grant-less `SessionAccept` and the session falls back to the
    94	/// in-stream carrier (contract §Transport selection: a responder that
    95	/// cannot bind grants no data plane).
    96	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
    97	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
    98	        Ok(listener) => listener,
    99	        Err(err) => {
   100	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
   101	            return None;
   102	        }
   103	    };
   104	    let port = match listener.local_addr() {
   105	        Ok(addr) => addr.port(),
   106	        Err(err) => {
   107	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   108	            return None;
   109	        }
   110	    };
   111	    // Two independent 16-byte credentials (contract §Transport: a socket
   112	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   113	    // is the fallible-RNG minter — a missing system RNG is an error, not
   114	    // a weaker credential.
   115	    let session_token = match generate_sub_token() {
   116	        Ok(token) => token,
   117	        Err(err) => {
   118	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   119	            return None;
   120	        }
   121	    };
   122	    let epoch0_sub_token = match generate_sub_token() {
   123	        Ok(token) => token,
   124	        Err(err) => {
   125	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   126	            return None;
   127	        }
   128	    };
   129	    // The grant is issued before any manifest is seen, so the proposal
   130	    // has zero knowledge: initial_streams == 1. All growth is via resize
   131	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   132	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   133	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   134	    Some(ResponderDataPlane {
   135	        listener,
   136	        session_token,
   137	        epoch0_sub_token,
   138	        initial_streams,
   139	        port,
   140	    })
   141	}
   142	
   143	/// Aggregated destination-side receive result: the write outcome plus
   144	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   145	/// which IS the settled live stream count this end observed. The sf-2
   146	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   147	pub(super) struct ReceiveTotals {
   148	    pub(super) outcome: SinkOutcome,
   149	    pub(super) streams: usize,
   150	}
   151	
   152	/// Live handle to a running responder data plane. The control loop arms
   153	/// resize credentials through [`Self::arm`] and joins the accept loop at
   154	/// `SourceDone` via [`Self::finish`].
   155	pub(super) struct ResponderDataPlaneRun {
   156	    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
   157	    task: AbortOnDrop<Result<ReceiveTotals>>,
   158	    /// The `session_token` half of every socket credential (the control
   159	    /// loop does not need it, but keeping it here documents the shape).
   160	    #[allow(dead_code)]
   161	    session_token: Vec<u8>,
   162	    /// The receiver's advertised `max_streams` — the control loop refuses
   163	    /// a resize that would grow past it (defense in depth; the source's
   164	    /// dial already clamps to the same ceiling).
   165	    pub(super) ceiling: usize,
   166	}
   167	
   168	impl ResponderDataPlane {
   169	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   170	    pub(super) fn grant(&self) -> DataPlaneGrant {
   171	        DataPlaneGrant {
   172	            tcp_port: self.port as u32,
   173	            session_token: self.session_token.clone(),
   174	            initial_streams: self.initial_streams,
   175	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   176	        }
   177	    }
   178	
   179	    /// The epoch-0 stream count this responder granted (always 1 — the
   180	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   181	    /// counter from it.
   182	    pub(super) fn initial_streams(&self) -> u32 {
   183	        self.initial_streams
   184	    }
   185	
   186	    /// Spawn the accept+receive loop and return a live handle. The loop
   187	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   188	    /// socket per armed resize credential until the control loop signals
   189	    /// `SourceDone` (drops the arm sender) and every receive worker has
   190	    /// drained its END. Runs concurrently with the control-stream diff
   191	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   192	    pub(super) fn spawn(self, sink: Arc<dyn TransferSink>) -> ResponderDataPlaneRun {
   193	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   194	        let session_token = self.session_token.clone();
   195	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
   196	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, arm_rx)));
   197	        ResponderDataPlaneRun {
   198	            arm_tx,
   199	            task,
   200	            session_token,
   201	            ceiling,
   202	        }
   203	    }
   204	
   205	    async fn accept_loop(
   206	        self,
   207	        sink: Arc<dyn TransferSink>,
   208	        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
   209	    ) -> Result<ReceiveTotals> {
   210	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   211	        let mut epoch0 = self.session_token.clone();
   212	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   213	
   214	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   215	        let mut total = SinkOutcome::default();
   216	        let mut streams = 0usize;
   217	
   218	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   219	        // grant is always 1; the loop handles N for symmetry).
   220	        for _ in 0..self.initial_streams {
   221	            let socket = accept_authenticated(&self.listener, &epoch0).await?;
   222	            streams += 1;
   223	            spawn_receive(&mut receives, socket, &sink);
   224	        }
   225	
   226	        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
   227	        // whose socket the SOURCE dials right after its ack. `no_more` is
   228	        // set when the control loop drops the arm sender at `SourceDone`;
   229	        // the loop then drains the last armed sockets and workers. Because
   230	        // the SOURCE only dials a credential it was acked for (and a dial
   231	        // failure faults the whole session, aborting this task via
   232	        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
   233	        let mut armed: Vec<Vec<u8>> = Vec::new();
   234	        let mut arm_rx = Some(arm_rx);
   235	        let mut no_more = false;
   236	        loop {
   237	            if no_more && armed.is_empty() && receives.is_empty() {
   238	                break;
   239	            }
   240	            // A closed arm channel resolves `recv()` instantly to `None`
   241	            // every poll; parking it on `pending()` once closed keeps the
   242	            // biased select from starving the accept/join arms (otherwise
   243	            // the None arm wins every race and the loop spins without ever
   244	            // collecting a finished worker).
   245	            let arm_recv = async {
   246	                match arm_rx.as_mut() {
   247	                    Some(rx) => rx.recv().await,
   248	                    None => std::future::pending().await,
   249	                }
   250	            };
   251	            tokio::select! {
   252	                biased;
   253	                // Control FIRST: an arm must register before its socket
   254	                // (which the SOURCE dials only after the ack the control
   255	                // loop sends right after arming), so the accept arm below
   256	                // always sees a populated `armed` set.
   257	                arm = arm_recv => match arm {
   258	                    Some(sub_token) => armed.push(sub_token),
   259	                    // Arm sender dropped at SourceDone: no more resizes.
   260	                    None => {

 succeeded in 0ms:
   260	                    None => {
   261	                        arm_rx = None;
   262	                        no_more = true;
   263	                    }
   264	                },
   265	                // Accept only when a resize credential is armed. `accept`
   266	                // is cancel-safe, so losing this arm to another (its
   267	                // pending connection stays queued) drops no socket. The
   268	                // credential read happens OUTSIDE the select (below) so a
   269	                // select cancel can never truncate a half-read socket.
   270	                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
   271	                    let socket = accepted?;
   272	                    let socket =
   273	                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
   274	                    streams += 1;
   275	                    spawn_receive(&mut receives, socket, &sink);
   276	                }
   277	                joined = receives.join_next(), if !receives.is_empty() => {
   278	                    let outcome = joined
   279	                        .expect("join_next is None only when empty, guarded above")
   280	                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   281	                    total.files_written += outcome.files_written;
   282	                    total.bytes_written += outcome.bytes_written;
   283	                }
   284	            }
   285	        }
   286	        Ok(ReceiveTotals {
   287	            outcome: total,
   288	            streams,
   289	        })
   290	    }
   291	}
   292	
   293	impl ResponderDataPlaneRun {
   294	    /// Arm a resize credential so the next socket presenting
   295	    /// `session_token ‖ sub_token` is accepted. Returns false if the
   296	    /// accept loop is gone (its receiver dropped) — the control loop then
   297	    /// acks the resize as refused.
   298	    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
   299	        self.arm_tx.send(sub_token).is_ok()
   300	    }
   301	
   302	    /// Signal `SourceDone` (no more resizes) and join the accept loop for
   303	    /// the aggregated receive totals.
   304	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   305	        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
   306	        // Dropping the arm sender is the "no more resizes" signal.
   307	        drop(arm_tx);
   308	        task.join()
   309	            .await
   310	            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
   311	    }
   312	}
   313	
   314	/// Spawn one receive worker draining `socket` into `sink` via the shared
   315	/// receive pipeline, guarded by the transfer stall timeout (carried REV4
   316	/// RELIABLE invariant, matching the old push receive: a peer that
   317	/// authenticates then stalls mid-record trips the stall timeout rather
   318	/// than pinning the task until TCP keepalive).
   319	fn spawn_receive(
   320	    receives: &mut JoinSet<Result<SinkOutcome>>,
   321	    socket: TcpStream,
   322	    sink: &Arc<dyn TransferSink>,
   323	) {
   324	    let sink = Arc::clone(sink);
   325	    receives.spawn(async move {
   326	        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
   327	        execute_receive_pipeline(&mut guarded, sink, None).await
   328	    });
   329	}
   330	
   331	/// Accept one data socket under the shared bounded-accept timeout and
   332	/// apply the data-plane socket policy. Cancel-safe (the accept itself is;
   333	/// no bytes are read here).
   334	async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
   335	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   336	    let socket = match accept {
   337	        Ok(Ok((socket, _peer))) => socket,
   338	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   339	        Err(_) => {
   340	            return Err(dp_fault(format!(
   341	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   342	        )))
   343	        }
   344	    };
   345	    configure_data_socket(&socket, None)
   346	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   347	    Ok(socket)
   348	}
   349	
   350	/// Read the fixed-length epoch-0 credential and verify it whole. A socket
   351	/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
   352	/// arms exactly the sockets it dials, so a mismatch is fatal here).
   353	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   354	    let mut socket = accept_raw(listener).await?;
   355	    let mut buf = vec![0u8; expected.len()];
   356	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   357	    match read {
   358	        Ok(Ok(_)) => {}
   359	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   360	        Err(_) => {
   361	            return Err(dp_fault(format!(
   362	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   363	            )))
   364	        }
   365	    }
   366	    // Constant-time comparison is not required: the tokens are 16 random
   367	    // bytes read once per socket, single-session; a timing oracle buys
   368	    // nothing against per-transfer secrets (same posture as the old push
   369	    // acceptor's `token == expected_token`).
   370	    if buf != expected {
   371	        return Err(dp_fault(
   372	            "data-plane socket presented an invalid credential",
   373	        ));
   374	    }
   375	    Ok(socket)
   376	}
   377	
   378	/// Read a resize socket's `session_token ‖ sub_token(16)` credential
   379	/// (bounded), verify the session token, and match the sub-token against
   380	/// an armed credential — removing it so each arm is consumed once. Runs
   381	/// in the accept loop body (never a select arm), so a select cancel can
   382	/// never truncate a half-read socket.
   383	async fn authenticate_resize(
   384	    socket: TcpStream,
   385	    session_token: &[u8],
   386	    armed: &mut Vec<Vec<u8>>,
   387	) -> Result<TcpStream> {
   388	    let mut socket = socket;
   389	    let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
   390	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   391	    match read {
   392	        Ok(Ok(_)) => {}
   393	        Ok(Err(err)) => {
   394	            return Err(dp_fault(format!(
   395	                "reading resize data-plane credential: {err}"
   396	            )))
   397	        }
   398	        Err(_) => {
   399	            return Err(dp_fault(format!(
   400	                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   401	            )))
   402	        }
   403	    }
   404	    if buf[..session_token.len()] != *session_token {
   405	        return Err(dp_fault(
   406	            "resize data socket presented a wrong session token",
   407	        ));
   408	    }
   409	    let sub = &buf[session_token.len()..];
   410	    match armed.iter().position(|t| t.as_slice() == sub) {
   411	        Some(idx) => {
   412	            armed.swap_remove(idx);
   413	            Ok(socket)
   414	        }
   415	        None => Err(dp_fault(
   416	            "resize data socket presented an unarmed credential",
   417	        )),
   418	    }
   419	}
   420	
   421	// ---------------------------------------------------------------------------
   422	// Initiator (DESTINATION) — dial, receive (otp-5b-1)
   423	// ---------------------------------------------------------------------------
   424	
   425	/// Live handle to a DESTINATION **initiator** receive data plane
   426	/// (otp-5b-1, the pull direction): the initiator dials the granted
   427	/// epoch-0 socket(s) and drains each into the sink through the shared
   428	/// receive pipeline — the same byte machinery the DESTINATION responder
   429	/// uses, only the socket is dialed instead of accepted. Single-stream: no
   430	/// resize arming (otp-5b-2 adds the accept-based epoch-N socket + dial).
   431	/// [`Self::finish`] joins the workers for the aggregated write outcome +
   432	/// settled stream count.
   433	pub(super) struct InitiatorReceivePlaneRun {
   434	    receives: JoinSet<Result<SinkOutcome>>,
   435	    streams: usize,
   436	}
   437	
   438	/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
   439	/// socket. `host` is the responder's host (the initiator reached the
   440	/// control plane there; the data plane rides the same host on the granted
   441	/// port — contract §Transport: the initiator always dials). Each worker
   442	/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
   443	/// in-stream carrier applies inline).
   444	pub(super) async fn dial_destination_data_plane(
   445	    host: &str,
   446	    grant: &DataPlaneGrant,
   447	    sink: Arc<dyn TransferSink>,
   448	) -> Result<InitiatorReceivePlaneRun> {
   449	    let initial = grant.initial_streams.max(1) as usize;
   450	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   451	    let mut handshake = grant.session_token.clone();
   452	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   453	    let addr = format!("{host}:{}", grant.tcp_port);
   454	
   455	    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   456	    let mut streams = 0usize;
   457	    for _ in 0..initial {
   458	        // `dial_data_plane` connects, applies the data-socket policy, and
   459	        // writes the handshake credential — the same bounded dial the
   460	        // SOURCE initiator uses (design-3: one owner for every client-side
   461	        // data-plane dial, both directions).
   462	        let socket = dial_data_plane(&addr, &handshake, None)
   463	            .await
   464	            .map_err(|err| dp_fault(format!("dialing session data plane (receive): {err:#}")))?;
   465	        streams += 1;
   466	        spawn_receive(&mut receives, socket, &sink);
   467	    }
   468	    Ok(InitiatorReceivePlaneRun { receives, streams })
   469	}
   470	
   471	impl InitiatorReceivePlaneRun {
   472	    /// Join every receive worker for the aggregated write totals. A worker
   473	    /// error (receive failure / stall) surfaces here; each drains to its
   474	    /// socket's END record on a clean transfer.
   475	    async fn finish(mut self) -> Result<ReceiveTotals> {
   476	        let mut total = SinkOutcome::default();
   477	        while let Some(joined) = self.receives.join_next().await {
   478	            let outcome =
   479	                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   480	            total.files_written += outcome.files_written;
   481	            total.bytes_written += outcome.bytes_written;
   482	        }
   483	        Ok(ReceiveTotals {
   484	            outcome: total,
   485	            streams: self.streams,
   486	        })
   487	    }
   488	}
   489	
   490	/// The DESTINATION end's receive data plane, tagged by connection role.
   491	/// Both drain socket bytes into the sink through the same receive
   492	/// pipeline; they differ only in how sockets are obtained (accept vs dial)
   493	/// and whether resize is armable (push only, otp-4b-2).
   494	pub(super) enum DestRecvPlane {
   495	    /// DESTINATION **responder** (push, otp-4b): accepts sockets, resize-
   496	    /// armable via the control loop.
   497	    Responder(ResponderDataPlaneRun),
   498	    /// DESTINATION **initiator** (pull, otp-5b-1): dialed single-stream
   499	    /// receive, no resize.
   500	    Initiator(InitiatorReceivePlaneRun),
   501	}
   502	
   503	impl DestRecvPlane {
   504	    /// Drain the data plane to completion and report the settled stream
   505	    /// count + write outcome (the DESTINATION is the scorer).
   506	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   507	        match self {
   508	            DestRecvPlane::Responder(run) => run.finish().await,
   509	            DestRecvPlane::Initiator(run) => run.finish().await,
   510	        }
   511	    }
   512	}
   513	
   514	// ---------------------------------------------------------------------------
   515	// Initiator (SOURCE) — dial, authenticate, send, resize
   516	// ---------------------------------------------------------------------------
   517	
   518	/// A resize the SOURCE has proposed and minted a credential for but not
   519	/// yet completed: the driver has sent (or will send) the matching
   520	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   521	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   522	/// flight (the dial's `pending_epoch` enforces it; this is the
   523	/// driver-side record the ack is matched against).
   524	pub(super) struct PendingResize {
   525	    pub(super) epoch: u32,
   526	    pub(super) target_streams: u32,
   527	    pub(super) sub_token: Vec<u8>,
   528	}
   529	
   530	/// A running source-side data plane: the dialed socket(s) wrapped as an
   531	/// ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the sf-2
   532	/// shape correction). Planned payloads are fed via [`Self::queue`];
   533	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   534	/// END record, and returns the bytes this end sent.
   535	pub(super) struct SourceDataPlane {
   536	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   537	    control_tx: mpsc::UnboundedSender<SinkControl>,
   538	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   539	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   540	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   541	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   542	    // advertised capacity (contract §Invariants 5). otp-4b-2 drives only
   543	    // its shape-correction stream count; the cheap-dial tuner is future
   544	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   545	    dial: Arc<TransferDial>,
   546	    source: Arc<dyn TransferSource>,
   547	    host: String,
   548	    tcp_port: u32,
   549	    session_token: Vec<u8>,
   550	    pool: Arc<BufferPool>,
   551	    /// Whether this data plane grows mid-transfer via `DataPlaneResize`.
   552	    /// True for the SOURCE **initiator** (push, otp-4b-2: it dials each
   553	    /// epoch-N socket on the ack). False for the SOURCE **responder**
   554	    /// (pull, otp-5b-1): the accept-based epoch-N socket + ack→accept
   555	    /// choreography is otp-5b-2, so this slice stays single-stream and
   556	    /// `propose_resize` returns `None` regardless of the need list.
   557	    resizable: bool,
   558	}
   559	
   560	/// Dial the granted data plane and start the elastic send pipeline.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '560,860p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   560	/// Dial the granted data plane and start the elastic send pipeline.
   561	/// `host` is the responder's host (the initiator connected the control
   562	/// plane to it; the data plane rides the same host on the granted port —
   563	/// contract §Transport: the initiator always dials). `receiver_capacity`
   564	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   565	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   566	/// never unlimited).
   567	pub(super) async fn dial_source_data_plane(
   568	    host: &str,
   569	    grant: &DataPlaneGrant,
   570	    receiver_capacity: Option<&CapacityProfile>,
   571	    source: Arc<dyn TransferSource>,
   572	) -> Result<SourceDataPlane> {
   573	    let initial = grant.initial_streams.max(1) as usize;
   574	    // The byte sender's dial, bounded by the receiver's advertised
   575	    // capacity. Seed the settled live count to the granted epoch-0
   576	    // streams — every shape-resize proposal steps from here.
   577	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   578	    dial.set_negotiated_streams(initial);
   579	
   580	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   581	    let mut handshake = grant.session_token.clone();
   582	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   583	
   584	    // Provision the pool for the dial ceiling so resize-added sockets
   585	    // draw buffers from the same pool without re-pooling (as old push
   586	    // does — a shared pool sized for the maximum stream count).
   587	    let pool = Arc::new(BufferPool::for_data_plane(
   588	        dial.chunk_bytes(),
   589	        dial.ceiling_max_streams().max(1),
   590	    ));
   591	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   592	    for _ in 0..initial {
   593	        let session = DataPlaneSession::connect(
   594	            host,
   595	            grant.tcp_port,
   596	            &handshake,
   597	            dial.chunk_bytes(),
   598	            dial.prefetch_count(),
   599	            false,
   600	            dial.tcp_buffer_bytes(),
   601	            Arc::clone(&pool),
   602	        )
   603	        .await
   604	        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
   605	        // The source-side sink never reads its dst_root (it only sends);
   606	        // `root()` is consulted by the relay/receive case, not here.
   607	        sinks.push(Arc::new(DataPlaneSink::new(
   608	            session,
   609	            Arc::clone(&source),
   610	            PathBuf::new(),
   611	        )));
   612	    }
   613	
   614	    let prefetch = dial.prefetch_count().max(1);
   615	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   616	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   617	    let pipe_source = Arc::clone(&source);
   618	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   619	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   620	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   621	        execute_sink_pipeline_elastic(
   622	            pipe_source,
   623	            sinks,
   624	            payload_rx,
   625	            prefetch,
   626	            None,
   627	            Some(control_rx),
   628	        )
   629	        .await
   630	    }));
   631	    Ok(SourceDataPlane {
   632	        payload_tx: Some(payload_tx),
   633	        control_tx,
   634	        pipeline: Some(pipeline),
   635	        dial,
   636	        source,
   637	        host: host.to_string(),
   638	        tcp_port: grant.tcp_port,
   639	        session_token: grant.session_token.clone(),
   640	        pool,
   641	        resizable: true,
   642	    })
   643	}
   644	
   645	/// Accept the granted epoch-0 socket(s) off a bound responder listener and
   646	/// start the elastic SEND pipeline over them — the SOURCE **responder**
   647	/// half of the pull data plane (otp-5b-1). Symmetric with
   648	/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
   649	/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
   650	/// socket acquisition differs (accept here, dial there).
   651	/// `DataPlaneSession::from_stream` builds a send session from an already-
   652	/// accepted socket — the same primitive the old `pull_sync` daemon-send
   653	/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
   654	/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
   655	/// wherever it initiates). Single-stream: `resizable: false`, so no
   656	/// `DataPlaneResize` is ever proposed on the pull data plane in this slice.
   657	pub(super) async fn accept_source_data_plane(
   658	    bound: ResponderDataPlane,
   659	    receiver_capacity: Option<&CapacityProfile>,
   660	    source: Arc<dyn TransferSource>,
   661	) -> Result<SourceDataPlane> {
   662	    let initial = bound.initial_streams.max(1) as usize;
   663	    // The byte sender's dial, bounded by the receiver's advertised
   664	    // capacity; seed the live count to the granted epoch-0 streams. Growth
   665	    // is disabled below (resizable=false), so the count stays here.
   666	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   667	    dial.set_negotiated_streams(initial);
   668	
   669	    // Epoch-0 credential the dialing DESTINATION presents:
   670	    // session_token ‖ epoch0_sub_token (contract §Transport).
   671	    let mut epoch0 = bound.session_token.clone();
   672	    epoch0.extend_from_slice(&bound.epoch0_sub_token);
   673	
   674	    let pool = Arc::new(BufferPool::for_data_plane(
   675	        dial.chunk_bytes(),
   676	        dial.ceiling_max_streams().max(1),
   677	    ));
   678	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   679	    for _ in 0..initial {
   680	        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
   681	        let session = DataPlaneSession::from_stream(
   682	            socket,
   683	            false,
   684	            dial.chunk_bytes(),
   685	            dial.prefetch_count(),
   686	            Arc::clone(&pool),
   687	        )
   688	        .await;
   689	        sinks.push(Arc::new(DataPlaneSink::new(
   690	            session,
   691	            Arc::clone(&source),
   692	            PathBuf::new(),
   693	        )));
   694	    }
   695	
   696	    let prefetch = dial.prefetch_count().max(1);
   697	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   698	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   699	    let pipe_source = Arc::clone(&source);
   700	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   701	        execute_sink_pipeline_elastic(
   702	            pipe_source,
   703	            sinks,
   704	            payload_rx,
   705	            prefetch,
   706	            None,
   707	            Some(control_rx),
   708	        )
   709	        .await
   710	    }));
   711	    Ok(SourceDataPlane {
   712	        payload_tx: Some(payload_tx),
   713	        control_tx,
   714	        pipeline: Some(pipeline),
   715	        dial,
   716	        source,
   717	        // Accept-based: this end never dials an epoch-N socket, so the
   718	        // dial-target fields are unused (add_stream is unreachable while
   719	        // resizable is false).
   720	        host: String::new(),
   721	        tcp_port: 0,
   722	        session_token: bound.session_token,
   723	        pool,
   724	        resizable: false,
   725	    })
   726	}
   727	
   728	impl SourceDataPlane {
   729	    /// The live dial (the byte sender owns it). The driver reads
   730	    /// `live_streams()` for observability and calls `resize_settled` as
   731	    /// each proposal completes.
   732	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   733	        &self.dial
   734	    }
   735	
   736	    /// sf-2 shape correction: propose one ADD toward the stream count the
   737	    /// accumulated need list implies, if none is in flight and the shape
   738	    /// wants more than the current live count. Mints the resize
   739	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   740	    /// the record back on the matching ack.
   741	    pub(super) fn propose_resize(
   742	        &self,
   743	        needed_bytes: u64,
   744	        needed_count: usize,
   745	    ) -> Result<Option<PendingResize>> {
   746	        // A non-resizable data plane (the SOURCE responder, otp-5b-1)
   747	        // never grows: the accept-based epoch-N socket is otp-5b-2.
   748	        if !self.resizable {
   749	            return Ok(None);
   750	        }
   751	        let desired =
   752	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   753	                as usize;
   754	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   755	            return Ok(None);
   756	        };
   757	        let sub_token = generate_sub_token()
   758	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   759	        Ok(Some(PendingResize {
   760	            epoch: proposal.epoch,
   761	            target_streams: proposal.target_streams as u32,
   762	            sub_token,
   763	        }))
   764	    }
   765	
   766	    /// Dial the epoch-N data socket for an accepted resize and hand it to
   767	    /// the running pipeline (`SinkControl::Add`). A dial failure is FATAL
   768	    /// (fail-fast): a same-build peer whose listener already accepted
   769	    /// epoch-0 failing an epoch-N dial is a transport fault worth
   770	    /// surfacing — and faulting the session aborts the peer's accept loop
   771	    /// via AbortOnDrop, so its armed slot never orphans. (Old push
   772	    /// recovers non-fatally via an arm TTL; the session trades that for
   773	    /// simplicity — noted in the finding doc.) If the pipeline is already
   774	    /// gone (transfer completing under the ADD), the just-dialed socket
   775	    /// is closed cleanly so the peer's worker sees its END, not a reset.
   776	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   777	        let mut handshake = self.session_token.clone();
   778	        handshake.extend_from_slice(sub_token);
   779	        let session = DataPlaneSession::connect(
   780	            &self.host,
   781	            self.tcp_port,
   782	            &handshake,
   783	            self.dial.chunk_bytes(),
   784	            self.dial.prefetch_count(),
   785	            false,
   786	            self.dial.tcp_buffer_bytes(),
   787	            Arc::clone(&self.pool),
   788	        )
   789	        .await
   790	        .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?;
   791	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   792	            session,
   793	            Arc::clone(&self.source),
   794	            PathBuf::new(),
   795	        ));
   796	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   797	            if let SinkControl::Add(sink) = returned.0 {
   798	                let _ = sink.finish().await;
   799	            }
   800	        }
   801	        Ok(())
   802	    }
   803	
   804	    /// Feed one planned batch into the send pipeline. The pipeline
   805	    /// prepares each payload (tar-shard/file) and writes it through the
   806	    /// data-plane record framing across the live socket(s).
   807	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   808	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   809	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   810	        })?;
   811	        for payload in payloads {
   812	            tx.send(payload).await.map_err(|_| {
   813	                dp_fault("data-plane send pipeline closed before all payloads sent")
   814	            })?;
   815	        }
   816	        Ok(())
   817	    }
   818	
   819	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   820	    /// socket's END record on drain), and return the bytes sent. Must be
   821	    /// awaited before `SourceDone` goes out so the destination's receive
   822	    /// pipeline sees END and completes.
   823	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   824	        // Drop the sender: workers observe the closed queue, drain what
   825	        // is left, then `finish()` (END record) and exit.
   826	        self.payload_tx = None;
   827	        let pipeline = self
   828	            .pipeline
   829	            .take()
   830	            .expect("SourceDataPlane::finish called once");
   831	        pipeline
   832	            .join()
   833	            .await
   834	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   835	    }
   836	}
   837	
   838	// ---------------------------------------------------------------------------
   839	// Need-list enforcement for the data-plane receive
   840	// ---------------------------------------------------------------------------
   841	
   842	/// Sink decorator that enforces the session's need-list contract on the
   843	/// data-plane receive, giving it the SAME strictness the in-stream
   844	/// carrier applies inline in the control loop (`outstanding.remove`).
   845	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   846	/// without this a peer could substitute an off-need-list path for a
   847	/// needed one (count-preserving), duplicate one, or send resume block
   848	/// records the non-resume session never negotiated (codex otp-4b-1 F1).
   849	/// Every written path must be a granted, not-yet-received need; resume
   850	/// block records are rejected outright. The shared [`OutstandingNeeds`]
   851	/// set makes completion `is_empty()` for both carriers.
   852	pub(super) struct NeedListSink {
   853	    inner: Arc<dyn TransferSink>,
   854	    outstanding: OutstandingNeeds,
   855	}
   856	
   857	impl NeedListSink {
   858	    pub(super) fn new(inner: Arc<dyn TransferSink>, outstanding: OutstandingNeeds) -> Self {
   859	        Self { inner, outstanding }
   860	    }

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified transfer session — the ONE block of transfer code
     2	//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
     3	//!
     4	//! A transfer has a SOURCE role and a DESTINATION role; which end
     5	//! initiated and which CLI verb was used select roles, never code.
     6	//! Both roles run the drivers below over a [`transport::FrameTransport`];
     7	//! the wire contract they implement — phases, frame table, record
     8	//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
     9	//!
    10	//! otp-3 scope: the role-parameterized state machine over the existing
    11	//! engine with the in-process transport and the in-stream byte
    12	//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
    13	//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
    14	//! delegated otp-9 (see the slice list in the plan).
    15	
    16	mod data_plane;
    17	pub mod transport;
    18	
    19	use std::collections::{HashMap, HashSet};
    20	use std::fmt;
    21	use std::future::Future;
    22	use std::path::{Path, PathBuf};
    23	use std::pin::Pin;
    24	use std::sync::atomic::{AtomicBool, Ordering};
    25	use std::sync::{Arc, Mutex as StdMutex};
    26	
    27	use eyre::Result;
    28	use tokio::io::{AsyncReadExt, AsyncWriteExt};
    29	use tokio::sync::mpsc;
    30	
    31	use crate::generated::transfer_frame::Frame;
    32	use crate::generated::{
    33	    session_error, ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp,
    34	    FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch, NeedComplete, NeedEntry,
    35	    SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone, TarShardComplete,
    36	    TarShardHeader, TransferFrame, TransferRole, TransferSummary,
    37	};
    38	use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
    39	use crate::remote::transfer::diff_planner;
    40	use crate::remote::transfer::payload::PreparedPayload;
    41	use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
    42	use crate::remote::transfer::source::{FsTransferSource, TransferSource};
    43	use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
    44	use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
    45	use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
    46	use crate::transfer_plan::PlanOptions;
    47	use transport::{FrameRx, FrameTransport, FrameTx};
    48	
    49	/// Belt-and-braces wire-shape version, bumped on any change to the
    50	/// frame set or grammar. Exchanged (and exact-matched) in
    51	/// `SessionHello` alongside the build id (D-2026-07-05-2).
    52	pub const CONTRACT_VERSION: u32 = 1;
    53	
    54	/// Payload chunk size on the in-stream carrier. Same unit the gRPC
    55	/// control plane uses today; the data plane (otp-4) has its own.
    56	const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
    57	
    58	/// Manifest entries buffered per destination diff batch. Mirrors the
    59	/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
    60	/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
    61	/// blocking pool instead of inline per entry.
    62	const DEST_DIFF_CHUNK: usize = 128;
    63	
    64	/// Buffer of the in-memory pipe that feeds wire file-record bytes
    65	/// into `FsTransferSink::write_file_stream`. Bounds destination-side
    66	/// buffering per file record.
    67	const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
    68	
    69	/// This build's session identity: `<crate version>+<git sha>[.dirty]`
    70	/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
    71	/// "unknown" when git was unavailable at compile time.
    72	pub fn session_build_id() -> &'static str {
    73	    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
    74	}
    75	
    76	/// The identity this end presents in `SessionHello`. Defaults to the
    77	/// real compile-time identity; tests inject mismatches.
    78	#[derive(Debug, Clone)]
    79	pub struct HelloConfig {
    80	    pub build_id: String,
    81	    pub contract_version: u32,
    82	}
    83	
    84	impl Default for HelloConfig {
    85	    fn default() -> Self {
    86	        Self {
    87	            build_id: session_build_id().to_string(),
    88	            contract_version: CONTRACT_VERSION,
    89	        }
    90	    }
    91	}
    92	
    93	/// Which handshake part this end plays. Orthogonal to role: all four
    94	/// initiator/role combinations run the same state machine (contract
    95	/// §Invariants 3).
    96	pub enum SessionEndpoint {
    97	    /// This end opened the transport; it sends `SessionOpen`.
    98	    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
    99	    Initiator { open: Box<SessionOpen> },
   100	    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
   101	    /// module/path/read-only validation attaches here at otp-4.
   102	    Responder,
   103	}
   104	
   105	impl SessionEndpoint {
   106	    /// Convenience constructor so callers don't spell the `Box`.
   107	    pub fn initiator(open: SessionOpen) -> Self {
   108	        SessionEndpoint::Initiator {
   109	            open: Box::new(open),
   110	        }
   111	    }
   112	}
   113	
   114	pub struct SourceSessionConfig {
   115	    pub hello: HelloConfig,
   116	    pub endpoint: SessionEndpoint,
   117	    /// Engine planner knobs (tar/large/raw thresholds). Local to the
   118	    /// source end — strategy selection is planner-owned and never
   119	    /// crosses the wire (contract §Transport selection).
   120	    pub plan_options: PlanOptions,
   121	    /// Host to dial the granted TCP data plane on (otp-4b). The
   122	    /// initiator connected the control plane to this host; the data
   123	    /// plane rides the same host on the granted port (contract
   124	    /// §Transport: the initiator always dials). `None` disables the
   125	    /// data plane at this end — a grant then faults, since the responder
   126	    /// is waiting to accept sockets that would never arrive.
   127	    pub data_plane_host: Option<String>,
   128	}
   129	
   130	pub struct DestinationSessionConfig {
   131	    pub hello: HelloConfig,
   132	    pub endpoint: SessionEndpoint,
   133	    /// Host to dial the granted TCP data plane on when this end is the
   134	    /// **initiator** (pull-equivalent, otp-5b): the DESTINATION initiator
   135	    /// dials the SOURCE responder's granted sockets on the same host it
   136	    /// reached the control plane on (contract §Transport: the initiator
   137	    /// always dials). `None` — or a DESTINATION responder, which binds
   138	    /// rather than dials — falls back to the in-stream carrier. Symmetric
   139	    /// with [`SourceSessionConfig::data_plane_host`].
   140	    pub data_plane_host: Option<String>,
   141	}
   142	
   143	/// A session-terminating fault: either end refusing, aborting, or
   144	/// catching the peer in a protocol violation. Carried as the error
   145	/// payload of the drivers' `eyre::Report`s — downcast to inspect the
   146	/// wire code.
   147	#[derive(Debug, Clone)]
   148	pub struct SessionFault {
   149	    pub code: session_error::Code,
   150	    pub message: String,
   151	    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
   152	    /// which end is stale (contract §Errors).
   153	    pub local_build_id: String,
   154	    pub peer_build_id: String,
   155	    /// True when the peer already knows about this fault — it sent
   156	    /// the `SessionError` frame itself, or this end already emitted
   157	    /// one. Drivers must not send another.
   158	    pub peer_notified: bool,
   159	}
   160	
   161	impl SessionFault {
   162	    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
   163	        Self {
   164	            code,
   165	            message: message.into(),
   166	            local_build_id: String::new(),
   167	            peer_build_id: String::new(),
   168	            peer_notified: false,
   169	        }
   170	    }
   171	
   172	    fn protocol_violation(message: impl Into<String>) -> Self {
   173	        Self::new(session_error::Code::ProtocolViolation, message)
   174	    }
   175	
   176	    fn internal(message: impl Into<String>) -> Self {
   177	        Self::new(session_error::Code::Internal, message)
   178	    }
   179	
   180	    fn read_only(message: impl Into<String>) -> Self {
   181	        Self::new(session_error::Code::ReadOnly, message)
   182	    }
   183	
   184	    /// Public constructor for a caller-side refusal (e.g. the daemon's
   185	    /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
   186	    /// code). blit-core stays free of `tonic::Status`, so the caller
   187	    /// picks the wire code.
   188	    pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
   189	        Self::new(code, message)
   190	    }
   191	
   192	    fn from_wire(err: SessionError) -> Self {
   193	        Self {
   194	            code: session_error::Code::try_from(err.code)
   195	                .unwrap_or(session_error::Code::SessionErrorUnspecified),
   196	            message: err.message,
   197	            // The peer reports its view: its "local" is our peer.
   198	            local_build_id: err.peer_build_id,
   199	            peer_build_id: err.local_build_id,
   200	            peer_notified: true,
   201	        }
   202	    }
   203	
   204	    fn to_wire(&self) -> SessionError {
   205	        SessionError {
   206	            code: self.code as i32,
   207	            message: self.message.clone(),
   208	            local_build_id: self.local_build_id.clone(),
   209	            peer_build_id: self.peer_build_id.clone(),
   210	        }
   211	    }
   212	}
   213	
   214	impl fmt::Display for SessionFault {
   215	    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
   216	        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
   217	    }
   218	}
   219	
   220	impl std::error::Error for SessionFault {}
   221	
   222	/// Downcast a driver-internal error back to its fault, wrapping
   223	/// non-fault failures (fs errors, planner errors, transport failures)
   224	/// as INTERNAL — an end that aborts says why before closing.
   225	fn fault_from_report(report: eyre::Report) -> SessionFault {
   226	    match report.downcast::<SessionFault>() {
   227	        Ok(fault) => fault,
   228	        Err(other) => SessionFault::internal(format!("{other:#}")),
   229	    }
   230	}
   231	
   232	fn frame(f: Frame) -> TransferFrame {
   233	    TransferFrame { frame: Some(f) }
   234	}
   235	
   236	fn error_frame(fault: &SessionFault) -> TransferFrame {
   237	    frame(Frame::Error(fault.to_wire()))
   238	}
   239	
   240	/// Short frame identifier for protocol-violation messages.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '240,560p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   240	/// Short frame identifier for protocol-violation messages.
   241	fn frame_name(f: &Option<Frame>) -> &'static str {
   242	    match f {
   243	        Some(Frame::Hello(_)) => "SessionHello",
   244	        Some(Frame::Open(_)) => "SessionOpen",
   245	        Some(Frame::Accept(_)) => "SessionAccept",
   246	        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
   247	        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
   248	        Some(Frame::NeedBatch(_)) => "NeedBatch",
   249	        Some(Frame::NeedComplete(_)) => "NeedComplete",
   250	        Some(Frame::BlockHashes(_)) => "BlockHashList",
   251	        Some(Frame::FileBegin(_)) => "FileBegin",
   252	        Some(Frame::FileData(_)) => "FileData",
   253	        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
   254	        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
   255	        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
   256	        Some(Frame::Block(_)) => "BlockTransfer",
   257	        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
   258	        Some(Frame::Resize(_)) => "DataPlaneResize",
   259	        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
   260	        Some(Frame::SourceDone(_)) => "SourceDone",
   261	        Some(Frame::Summary(_)) => "TransferSummary",
   262	        Some(Frame::Error(_)) => "SessionError",
   263	        None => "empty frame",
   264	    }
   265	}
   266	
   267	fn complement(role: TransferRole) -> TransferRole {
   268	    match role {
   269	        TransferRole::Source => TransferRole::Destination,
   270	        TransferRole::Destination => TransferRole::Source,
   271	        TransferRole::Unspecified => TransferRole::Unspecified,
   272	    }
   273	}
   274	
   275	/// Build a `SessionError` frame with the given code and message — the
   276	/// wire form an end sends to tell its peer why it is aborting. Public
   277	/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
   278	/// fires mid-session (the session future is aborted by the select and
   279	/// cannot send it itself — otp-4a codex F1); blit-core stays the one
   280	/// owner of the frame grammar. The build-id fields are left empty:
   281	/// they are only meaningful for `BUILD_MISMATCH`.
   282	pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
   283	    frame(Frame::Error(SessionError {
   284	        code: code as i32,
   285	        message: message.into(),
   286	        local_build_id: String::new(),
   287	        peer_build_id: String::new(),
   288	    }))
   289	}
   290	
   291	/// Per-role capability check of the operation a `SessionOpen`
   292	/// describes. otp-3 refuses what later slices implement rather than
   293	/// silently ignoring it (fail-fast; contract §Errors).
   294	type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
   295	
   296	/// The local endpoint a Responder resolves a received `SessionOpen`
   297	/// to. The daemon maps the wire module name + path here; a test can
   298	/// hand a fixed root with no module semantics via
   299	/// [`DestinationTarget::Fixed`] instead.
   300	#[derive(Debug, Clone)]
   301	pub struct ResolvedEndpoint {
   302	    /// Absolute local root this end targets.
   303	    pub root: PathBuf,
   304	    /// Whether the resolved module forbids writes. A DESTINATION
   305	    /// responder refuses `READ_ONLY`; a SOURCE responder (otp-5,
   306	    /// daemon-send) does not care — reading a read-only module is fine.
   307	    pub read_only: bool,
   308	}
   309	
   310	/// Async callback a Responder uses to turn a received (and
   311	/// capability-validated) `SessionOpen` into its local endpoint. It
   312	/// lives caller-side — the daemon resolves modules and maps its own
   313	/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
   314	/// of module/Status types. A returned fault (unknown module,
   315	/// containment failure) becomes a `SessionError` at OPEN, never a
   316	/// silent close (contract §Phase state machine).
   317	pub type OpenResolver = dyn Fn(
   318	        &SessionOpen,
   319	    )
   320	        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
   321	    + Send
   322	    + Sync;
   323	
   324	/// Where a DESTINATION driver writes. `Fixed` is a root known up front
   325	/// (an initiator's own local root, or a test's temp dir). `Resolve`
   326	/// defers to a caller callback that maps the received `SessionOpen` to
   327	/// a local root — the daemon path, where the root depends on the wire
   328	/// module name and so can only be resolved mid-handshake (after HELLO,
   329	/// before SessionAccept). A `Resolve` target is meaningful only on a
   330	/// Responder; an Initiator always knows its own root.
   331	pub enum DestinationTarget {
   332	    Fixed(PathBuf),
   333	    Resolve(Box<OpenResolver>),
   334	}
   335	
   336	/// Where a SOURCE responder reads from. Symmetric with
   337	/// [`DestinationTarget`]: `Fixed` is a source known up front (an
   338	/// initiator's own tree, or a test), `Resolve` defers to the same
   339	/// [`OpenResolver`] the destination path uses to map a received
   340	/// `SessionOpen`'s module name to a local root, from which a
   341	/// [`FsTransferSource`] is built inside blit-core (so callers stay free
   342	/// of the concrete source type, exactly as `run_destination` builds its
   343	/// sink from `dst_root`). A `Resolve` target is meaningful only on a
   344	/// Responder; an Initiator always knows its own source. Used by
   345	/// [`run_responder`] for the daemon-as-SOURCE (pull-equivalent, otp-5).
   346	pub enum SourceResponderTarget {
   347	    Fixed(Arc<dyn TransferSource>),
   348	    Resolve(Box<OpenResolver>),
   349	}
   350	
   351	/// What a served session produced, tagged by which role the responder
   352	/// played. `run_responder` dispatches on the initiator's declared role,
   353	/// so the caller (the daemon) learns after the fact which half ran.
   354	pub enum ResponderOutcome {
   355	    /// The initiator was SOURCE; this end received (push-equivalent).
   356	    Destination(DestinationOutcome),
   357	    /// The initiator was DESTINATION; this end sent (pull-equivalent).
   358	    Source(TransferSummary),
   359	}
   360	
   361	fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   362	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   363	        return Err(SessionFault::internal(
   364	            "resume is not implemented on the unified session yet (otp-7)",
   365	        ));
   366	    }
   367	    if open
   368	        .filter
   369	        .as_ref()
   370	        .is_some_and(|f| *f != FilterSpec::default())
   371	    {
   372	        return Err(SessionFault::internal(
   373	            "filters are not implemented on the unified session yet (otp-6)",
   374	        ));
   375	    }
   376	    Ok(())
   377	}
   378	
   379	fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   380	    if open.mirror_enabled {
   381	        return Err(SessionFault::internal(
   382	            "mirror is not implemented on the unified session yet (otp-6)",
   383	        ));
   384	    }
   385	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   386	        return Err(SessionFault::internal(
   387	            "resume is not implemented on the unified session yet (otp-7)",
   388	        ));
   389	    }
   390	    Ok(())
   391	}
   392	
   393	/// Outcome of the HELLO + OPEN phases.
   394	struct Negotiated {
   395	    open: SessionOpen,
   396	    /// The responder's reply. The SOURCE initiator reads
   397	    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
   398	    accept: SessionAccept,
   399	    /// The write root a Responder's [`OpenResolver`] produced from the
   400	    /// received open, if one was supplied; `None` for an Initiator or a
   401	    /// fixed-root Responder (the caller supplies the root then).
   402	    resolved_root: Option<PathBuf>,
   403	    /// The bound data-plane listener + credentials a DESTINATION
   404	    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
   405	    /// on an Initiator, or when the responder granted no data plane
   406	    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
   407	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
   408	}
   409	
   410	/// HELLO both ways, exact match (D-2026-07-05-2). First frame each
   411	/// direction; no ordering between the two directions. Factored out so a
   412	/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
   413	/// and dispatch on the declared role before running a role driver.
   414	async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
   415	    transport
   416	        .send(frame(Frame::Hello(SessionHello {
   417	            build_id: hello.build_id.clone(),
   418	            contract_version: hello.contract_version,
   419	        })))
   420	        .await?;
   421	
   422	    let peer_hello = match expect_frame(transport).await? {
   423	        Frame::Hello(h) => h,
   424	        other => {
   425	            return Err(notify_and_wrap(
   426	                transport,
   427	                SessionFault::protocol_violation(format!(
   428	                    "expected SessionHello, got {}",
   429	                    frame_name(&Some(other))
   430	                )),
   431	            )
   432	            .await)
   433	        }
   434	    };
   435	
   436	    if peer_hello.build_id != hello.build_id
   437	        || peer_hello.contract_version != hello.contract_version
   438	    {
   439	        let fault = SessionFault {
   440	            code: session_error::Code::BuildMismatch,
   441	            message: format!(
   442	                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
   443	                hello.build_id, hello.contract_version,
   444	                peer_hello.build_id, peer_hello.contract_version,
   445	            ),
   446	            local_build_id: hello.build_id.clone(),
   447	            peer_build_id: peer_hello.build_id.clone(),
   448	            peer_notified: false,
   449	        };
   450	        return Err(notify_and_wrap(transport, fault).await);
   451	    }
   452	    Ok(())
   453	}
   454	
   455	/// The responder half of establish AFTER the `SessionOpen` is read:
   456	/// complement check, `validate_open`, endpoint resolution, data-plane
   457	/// prepare, and `SessionAccept`. Factored out so both `establish` (which
   458	/// reads the open then calls this) and `run_responder` (which reads the
   459	/// open, dispatches on the declared role, then calls this with the
   460	/// resolved local role) share one implementation. Sends the refusal
   461	/// `SessionError` itself; returned faults are `peer_notified`.
   462	async fn responder_finish(
   463	    transport: &mut FrameTransport,
   464	    open: SessionOpen,
   465	    local_role: TransferRole,
   466	    validate_open: &OpenValidator,
   467	    resolve_open: Option<&OpenResolver>,
   468	) -> Result<Negotiated> {
   469	    // The initiator declares ITS role; this responder end must
   470	    // hold the complement.
   471	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   472	    if declared != complement(local_role) {
   473	        return Err(notify_and_wrap(
   474	            transport,
   475	            SessionFault::protocol_violation(format!(
   476	                "initiator declared role {} but this responder is {}",
   477	                declared.as_str_name(),
   478	                local_role.as_str_name()
   479	            )),
   480	        )
   481	        .await);
   482	    }
   483	    if let Err(fault) = validate_open(&open) {
   484	        // Refusal is a SessionError instead of SessionAccept,
   485	        // never a silent close (contract §Phase state machine).
   486	        return Err(notify_and_wrap(transport, fault).await);
   487	    }
   488	    // Responder endpoint resolution (otp-4): map the wire
   489	    // module/path to a local root and enforce read-only, both
   490	    // BEFORE SessionAccept so a refusal replaces the accept
   491	    // (never follows it). The resolver is caller-supplied
   492	    // (daemon module lookup); a fixed-root responder passes
   493	    // None and resolves nothing here.
   494	    let resolved_root = match resolve_open {
   495	        Some(resolve) => match resolve(&open).await {
   496	            Ok(resolved) => {
   497	                // A read-only module is fatal only for a
   498	                // DESTINATION (it would write); a SOURCE
   499	                // responder (otp-5, daemon-send) reads happily.
   500	                if local_role == TransferRole::Destination && resolved.read_only {
   501	                    return Err(notify_and_wrap(
   502	                        transport,
   503	                        SessionFault::read_only("destination module is read-only".to_string()),
   504	                    )
   505	                    .await);
   506	                }
   507	                Some(resolved.root)
   508	            }
   509	            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
   510	        },
   511	        None => None,
   512	    };
   513	    // Data plane (otp-4b/5b): a responder binds a TCP listener and grants
   514	    // it, unless the initiator requested the in-stream carrier or the bind
   515	    // fails (grant-less accept ⇒ in-stream fallback). This is role-agnostic
   516	    // (otp-5b): the RESPONDER binds+accepts and the INITIATOR dials, while
   517	    // byte direction is set by role — a DESTINATION responder accepts+
   518	    // receives (push, otp-4b), a SOURCE responder accepts+sends (pull,
   519	    // otp-5b). The bound listener travels in `Negotiated.responder_data_plane`
   520	    // and is consumed by whichever role's driver runs.
   521	    let responder_data_plane = if open.in_stream_bytes {
   522	        None
   523	    } else {
   524	        data_plane::prepare_responder_data_plane().await
   525	    };
   526	    let accept = SessionAccept {
   527	        // The byte RECEIVER advertises capacity at session
   528	        // open (D-2026-06-20-1/-2); consumed by the dial when
   529	        // the data plane lands (otp-4b).
   530	        receiver_capacity: if local_role == TransferRole::Destination {
   531	            Some(crate::engine::local_receiver_capacity())
   532	        } else {
   533	            None
   534	        },
   535	        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
   536	        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
   537	    };
   538	    transport.send(frame(Frame::Accept(accept.clone()))).await?;
   539	    Ok(Negotiated {
   540	        open,
   541	        accept,
   542	        resolved_root,
   543	        responder_data_plane,
   544	    })
   545	}
   546	
   547	/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
   548	/// scoping requirement). Sends the refusal `SessionError` itself when
   549	/// it detects the fault locally; returned faults are `peer_notified`.
   550	async fn establish(
   551	    transport: &mut FrameTransport,
   552	    hello: &HelloConfig,
   553	    endpoint: &SessionEndpoint,
   554	    local_role: TransferRole,
   555	    validate_open: &OpenValidator,
   556	    // Consulted only on the Responder branch, after the received open
   557	    // passes `validate_open` and before SessionAccept. `None` = the
   558	    // caller supplies the root itself (Initiator, or fixed-root test).
   559	    resolve_open: Option<&OpenResolver>,
   560	) -> Result<Negotiated> {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1040,1480p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1040	    // propose further here: exactly the one in-flight resize is drained.
  1041	    if let Some(dp) = &data_plane {
  1042	        if let Some(pending) = pending_resize.take() {
  1043	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1044	        }
  1045	    }
  1046	
  1047	    // Close the data plane BEFORE SourceDone so the destination's receive
  1048	    // pipeline sees each socket's END record and completes; SourceDone on
  1049	    // the control lane then lets the destination score and summarize.
  1050	    //
  1051	    // The drain is the byte-transfer phase's wall-time sink, so a
  1052	    // mid-transfer cancel almost always lands here. Race it against a
  1053	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
  1054	    // the served session frames `SessionError{CANCELLED}`, and the source
  1055	    // must surface THAT — not the data-plane transport break it also
  1056	    // causes. Two orderings, both covered:
  1057	    //   * fault arrives while the drain is still pending (e.g. a worker
  1058	    //     blocked reading a slow file, so the socket break never unblocks
  1059	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
  1060	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
  1061	    //     stops the in-flight workers.
  1062	    //   * the socket break makes `finish()` return `Err` first → prefer
  1063	    //     the framed reason if the control lane delivers one within the
  1064	    //     stall window (`prefer_peer_fault`).
  1065	    if let Some(dp) = data_plane.take() {
  1066	        tokio::select! {
  1067	            biased;
  1068	            fault = recv_peer_fault(&mut events) => {
  1069	                return Err(eyre::Report::new(fault));
  1070	            }
  1071	            res = dp.finish() => {
  1072	                if let Err(dp_err) = res {
  1073	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
  1074	                }
  1075	            }
  1076	        }
  1077	    }
  1078	
  1079	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
  1080	
  1081	    // CLOSING: the destination is the scorer; the next event must be
  1082	    // its summary (the receive half ends after forwarding it).
  1083	    match events.recv().await {
  1084	        Some(SourceEvent::Summary(summary)) => Ok(summary),
  1085	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
  1086	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1087	            format!("need for '{}' after NeedComplete", h.relative_path),
  1088	        ))),
  1089	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
  1090	            SessionFault::protocol_violation("duplicate NeedComplete"),
  1091	        )),
  1092	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
  1093	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
  1094	        )),
  1095	        None => Err(eyre::Report::new(SessionFault::internal(
  1096	            "source receive half ended before TransferSummary",
  1097	        ))),
  1098	    }
  1099	}
  1100	
  1101	/// Process every event ready right now (needs accumulating, resize acks
  1102	/// dialing their epoch-N socket) without blocking. Called between
  1103	/// manifest sends and at the top of the payload loop.
  1104	#[allow(clippy::too_many_arguments)]
  1105	async fn drain_ready_source_events(
  1106	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1107	    pending: &mut Vec<FileHeader>,
  1108	    need_complete: &mut bool,
  1109	    needed_bytes: &mut u64,
  1110	    needed_count: &mut usize,
  1111	    data_plane: Option<&data_plane::SourceDataPlane>,
  1112	    tx: &mut Box<dyn FrameTx>,
  1113	    pending_resize: &mut Option<data_plane::PendingResize>,
  1114	) -> Result<()> {
  1115	    while let Ok(event) = events.try_recv() {
  1116	        process_source_event(
  1117	            event,
  1118	            pending,
  1119	            need_complete,
  1120	            needed_bytes,
  1121	            needed_count,
  1122	            data_plane,
  1123	            tx,
  1124	            pending_resize,
  1125	        )
  1126	        .await?;
  1127	    }
  1128	    Ok(())
  1129	}
  1130	
  1131	/// Handle one source event. Needs accumulate into `pending` and the
  1132	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1133	/// next ADD (the one-per-epoch ramp).
  1134	#[allow(clippy::too_many_arguments)]
  1135	async fn process_source_event(
  1136	    event: SourceEvent,
  1137	    pending: &mut Vec<FileHeader>,
  1138	    need_complete: &mut bool,
  1139	    needed_bytes: &mut u64,
  1140	    needed_count: &mut usize,
  1141	    data_plane: Option<&data_plane::SourceDataPlane>,
  1142	    tx: &mut Box<dyn FrameTx>,
  1143	    pending_resize: &mut Option<data_plane::PendingResize>,
  1144	) -> Result<()> {
  1145	    match event {
  1146	        SourceEvent::Need(header) => {
  1147	            if *need_complete {
  1148	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1149	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1150	                )));
  1151	            }
  1152	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1153	            *needed_count += 1;
  1154	            pending.push(header);
  1155	            Ok(())
  1156	        }
  1157	        SourceEvent::NeedComplete => {
  1158	            if *need_complete {
  1159	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1160	                    "duplicate NeedComplete",
  1161	                )));
  1162	            }
  1163	            *need_complete = true;
  1164	            Ok(())
  1165	        }
  1166	        SourceEvent::ResizeAck(ack) => {
  1167	            let dp = data_plane.ok_or_else(|| {
  1168	                eyre::Report::new(SessionFault::protocol_violation(
  1169	                    "DataPlaneResizeAck on a session with no data plane",
  1170	                ))
  1171	            })?;
  1172	            // Match the ack to the in-flight proposal; stale/unsolicited
  1173	            // acks (wrong epoch, or none pending) are ignored, matching
  1174	            // old push. `take()` + restore keeps the borrow simple.
  1175	            let pending_r = match pending_resize.take() {
  1176	                Some(p) if p.epoch == ack.epoch => p,
  1177	                restored => {
  1178	                    *pending_resize = restored;
  1179	                    return Ok(());
  1180	                }
  1181	            };
  1182	            if ack.accepted {
  1183	                dp.add_stream(&pending_r.sub_token).await?;
  1184	                dp.dial()
  1185	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1186	            } else {
  1187	                dp.dial()
  1188	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1189	            }
  1190	            // Ramp one stream per accepted epoch: propose the next ADD.
  1191	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1192	        }
  1193	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1194	            "TransferSummary before SourceDone",
  1195	        ))),
  1196	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1197	    }
  1198	}
  1199	
  1200	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1201	/// the stream count the accumulated need list implies, if none is in
  1202	/// flight. A no-op when the shape wants no more than the live count (the
  1203	/// dial returns `None`). Sends the frame and records the in-flight
  1204	/// proposal for the ack to match.
  1205	async fn maybe_propose_resize(
  1206	    dp: &data_plane::SourceDataPlane,
  1207	    tx: &mut Box<dyn FrameTx>,
  1208	    needed_bytes: u64,
  1209	    needed_count: usize,
  1210	    pending_resize: &mut Option<data_plane::PendingResize>,
  1211	) -> Result<()> {
  1212	    if pending_resize.is_some() {
  1213	        return Ok(());
  1214	    }
  1215	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1216	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1217	            op: DataPlaneResizeOp::Add as i32,
  1218	            epoch: proposal.epoch,
  1219	            target_stream_count: proposal.target_streams,
  1220	            sub_token: proposal.sub_token.clone(),
  1221	        })))
  1222	        .await?;
  1223	        *pending_resize = Some(proposal);
  1224	    }
  1225	    Ok(())
  1226	}
  1227	
  1228	/// Block for the ack of the one in-flight resize and dial its socket (or
  1229	/// settle it refused). Does NOT propose further — it resolves exactly the
  1230	/// pending proposal so the destination's armed slot is consumed before we
  1231	/// finish the data plane.
  1232	async fn resolve_in_flight_resize(
  1233	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1234	    dp: &data_plane::SourceDataPlane,
  1235	    pending: data_plane::PendingResize,
  1236	) -> Result<()> {
  1237	    loop {
  1238	        match events.recv().await {
  1239	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1240	                if ack.accepted {
  1241	                    dp.add_stream(&pending.sub_token).await?;
  1242	                    dp.dial()
  1243	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1244	                } else {
  1245	                    dp.dial()
  1246	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1247	                }
  1248	                return Ok(());
  1249	            }
  1250	            // A stale ack for an already-settled epoch: ignore, keep
  1251	            // waiting for ours.
  1252	            Some(SourceEvent::ResizeAck(_)) => continue,
  1253	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1254	            Some(SourceEvent::Need(h)) => {
  1255	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1256	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1257	                )))
  1258	            }
  1259	            Some(SourceEvent::NeedComplete) => {
  1260	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1261	                    "duplicate NeedComplete",
  1262	                )))
  1263	            }
  1264	            Some(SourceEvent::Summary(_)) => {
  1265	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1266	                    "TransferSummary before SourceDone",
  1267	                )))
  1268	            }
  1269	            None => {
  1270	                return Err(eyre::Report::new(SessionFault::internal(
  1271	                    "source receive half ended with a resize in flight",
  1272	                )))
  1273	            }
  1274	        }
  1275	    }
  1276	}
  1277	
  1278	/// Await the next terminal signal the receive half forwards while the
  1279	/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
  1280	/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
  1281	/// abort the send and surface as the fault.
  1282	///
  1283	/// The drain runs after `resolve_in_flight_resize` and before `SourceDone`
  1284	/// goes out, so the event channel is drained and the peer sends nothing
  1285	/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
  1286	/// or `Summary` is legitimate here. So a `Fault` is returned as-is and any
  1287	/// OTHER event is surfaced as a protocol violation rather than silently
  1288	/// dropped (codex otp-4b-3 F3): dropping it would defer or lose a
  1289	/// fail-fast error and, if the drain is itself stuck, hang. Parks forever
  1290	/// once the channel closes with no event so the data-plane future it
  1291	/// races decides the outcome instead.
  1292	async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
  1293	    match events.recv().await {
  1294	        Some(SourceEvent::Fault(fault)) => fault,
  1295	        Some(SourceEvent::Need(h)) => SessionFault::protocol_violation(format!(
  1296	            "need for '{}' during the data-plane drain (after NeedComplete)",
  1297	            h.relative_path
  1298	        )),
  1299	        Some(SourceEvent::NeedComplete) => {
  1300	            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
  1301	        }
  1302	        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
  1303	            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
  1304	        ),
  1305	        Some(SourceEvent::Summary(_)) => {
  1306	            SessionFault::protocol_violation("TransferSummary before SourceDone")
  1307	        }
  1308	        None => std::future::pending().await,
  1309	    }
  1310	}
  1311	
  1312	/// A data-plane operation (`queue`/`finish`) failed mid-transfer. The
  1313	/// break is usually the *symptom* of a peer abort — within
  1314	/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
  1315	/// its receive workers) always frames the real reason on the control
  1316	/// lane. Prefer that framed fault; fall back to the raw data-plane error
  1317	/// if the channel closes first or none arrives in that window.
  1318	///
  1319	/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
  1320	/// fast on any stray event), this is called from BOTH error sites,
  1321	/// including the `queue()` error inside the payload loop — where a
  1322	/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
  1323	/// ahead of the peer's `SessionError` (codex otp-4b-3 pass-2 F1). So it
  1324	/// SKIPS non-fault events rather than treating them as violations: we are
  1325	/// already unwinding on a data-plane error, and the framed fault (or the
  1326	/// dp error) is the correct outcome, never a spurious protocol violation.
  1327	async fn prefer_peer_fault(
  1328	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1329	    dp_err: eyre::Report,
  1330	) -> eyre::Report {
  1331	    let framed = async {
  1332	        loop {
  1333	            match events.recv().await {
  1334	                Some(SourceEvent::Fault(fault)) => break Some(fault),
  1335	                // Skip a still-in-flight need/ack/complete: on this error
  1336	                // path the transfer is aborting, so the framed reason (or
  1337	                // the dp error) wins, not a stray-event violation.
  1338	                Some(_) => continue,
  1339	                // Receive half ended without framing a fault → the raw
  1340	                // data-plane error is the best available cause.
  1341	                None => break None,
  1342	            }
  1343	        }
  1344	    };
  1345	    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, framed).await {
  1346	        Ok(Some(fault)) => eyre::Report::new(fault),
  1347	        Ok(None) | Err(_) => dp_err,
  1348	    }
  1349	}
  1350	
  1351	/// Plan one batch of needed headers with the engine planner and emit
  1352	/// the resulting payload records per the in-stream grammar.
  1353	async fn send_payload_records(
  1354	    tx: &mut Box<dyn FrameTx>,
  1355	    source: &Arc<dyn TransferSource>,
  1356	    plan_options: PlanOptions,
  1357	    batch: Vec<FileHeader>,
  1358	    read_buf: &mut [u8],
  1359	) -> Result<()> {
  1360	    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1361	    for payload in payloads {
  1362	        match source.prepare_payload(payload).await? {
  1363	            PreparedPayload::File(header) => {
  1364	                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
  1365	                if header.size == 0 {
  1366	                    continue; // record complete at 0 cumulative bytes
  1367	                }
  1368	                let mut reader = source.open_file(&header).await?;
  1369	                let mut remaining = header.size;
  1370	                while remaining > 0 {
  1371	                    let want = read_buf.len().min(remaining as usize);
  1372	                    let got = reader.read(&mut read_buf[..want]).await?;
  1373	                    if got == 0 {
  1374	                        // Shorter on disk than the manifest promised —
  1375	                        // the record can no longer complete at
  1376	                        // header.size; abort rather than pad.
  1377	                        eyre::bail!(
  1378	                            "'{}' hit EOF with {} bytes still promised",
  1379	                            header.relative_path,
  1380	                            remaining
  1381	                        );
  1382	                    }
  1383	                    tx.send(frame(Frame::FileData(FileData {
  1384	                        content: read_buf[..got].to_vec(),
  1385	                    })))
  1386	                    .await?;
  1387	                    remaining -= got as u64;
  1388	                }
  1389	            }
  1390	            PreparedPayload::TarShard { headers, data } => {
  1391	                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
  1392	                    files: headers,
  1393	                    archive_size: data.len() as u64,
  1394	                })))
  1395	                .await?;
  1396	                for chunk in data.chunks(IN_STREAM_CHUNK) {
  1397	                    tx.send(frame(Frame::TarShardChunk(
  1398	                        crate::generated::TarShardChunk {
  1399	                            content: chunk.to_vec(),
  1400	                        },
  1401	                    )))
  1402	                    .await?;
  1403	                }
  1404	                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
  1405	                    .await?;
  1406	            }
  1407	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
  1408	                // The outbound planner never emits these (resume is
  1409	                // receive-originated and lands at otp-7).
  1410	                eyre::bail!("resume payload planned in a non-resume session");
  1411	            }
  1412	        }
  1413	    }
  1414	    Ok(())
  1415	}
  1416	
  1417	// ---------------------------------------------------------------------------
  1418	// DESTINATION driver
  1419	// ---------------------------------------------------------------------------
  1420	
  1421	/// What the destination end can report after a completed session.
  1422	#[derive(Debug, Clone)]
  1423	pub struct DestinationOutcome {
  1424	    /// The summary this end computed and sent (contract: DESTINATION
  1425	    /// is the scorer).
  1426	    pub summary: TransferSummary,
  1427	    /// Paths this end put on the need list, in emission order. The
  1428	    /// role suite pins these identical across role assignments — the
  1429	    /// executable form of the owner's invariance requirement.
  1430	    pub needed_paths: Vec<String>,
  1431	    /// The settled data-plane stream count this end observed (epoch-0 +
  1432	    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
  1433	    /// pin (otp-4b-2) reads it to assert shape correction grew the
  1434	    /// stream set past the zero-knowledge single-stream grant.
  1435	    pub data_plane_streams: Option<usize>,
  1436	}
  1437	
  1438	/// Run the DESTINATION role of one transfer session over `transport`,
  1439	/// writing under the root named by `target`. Diffs the streamed
  1440	/// manifest against its own filesystem (the destination is the one
  1441	/// diff owner — plan §Design 3), returns the summary it computed and
  1442	/// sent.
  1443	///
  1444	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  1445	/// front (an Initiator's own local root, or a test), or
  1446	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  1447	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  1448	/// where the wire module name selects the root).
  1449	pub async fn run_destination(
  1450	    cfg: DestinationSessionConfig,
  1451	    transport: FrameTransport,
  1452	    target: DestinationTarget,
  1453	) -> Result<DestinationOutcome> {
  1454	    let mut transport = transport;
  1455	    let endpoint = match cfg.endpoint {
  1456	        SessionEndpoint::Initiator { mut open } => {
  1457	            let declared = TransferRole::try_from(open.initiator_role);
  1458	            if declared != Ok(TransferRole::Destination) {
  1459	                eyre::bail!(
  1460	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  1461	                );
  1462	            }
  1463	            if let Err(fault) = destination_open_validator(&open) {
  1464	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  1465	            }
  1466	            // Dial contract: the byte receiver advertises capacity in
  1467	            // its open when it is the initiator (contract §Invariants 5).
  1468	            if open.receiver_capacity.is_none() {
  1469	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
  1470	            }
  1471	            SessionEndpoint::Initiator { open }
  1472	        }
  1473	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  1474	    };
  1475	
  1476	    let resolve_open: Option<&OpenResolver> = match &target {
  1477	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1478	        DestinationTarget::Fixed(_) => None,
  1479	    };
  1480	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1480,1940p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1480	
  1481	    let negotiated = establish(
  1482	        &mut transport,
  1483	        &cfg.hello,
  1484	        &endpoint,
  1485	        TransferRole::Destination,
  1486	        &destination_open_validator,
  1487	        resolve_open,
  1488	    )
  1489	    .await?;
  1490	
  1491	    // The resolver's root (Responder + Resolve) wins; otherwise the
  1492	    // caller-supplied Fixed root.
  1493	    let dst_root = match negotiated.resolved_root.clone() {
  1494	        Some(root) => root,
  1495	        None => match &target {
  1496	            DestinationTarget::Fixed(root) => root.clone(),
  1497	            // Unreachable: a Resolve target always yields a root on the
  1498	            // Responder branch, and establish only skips resolution on
  1499	            // the Initiator branch (which pairs with a Fixed root).
  1500	            DestinationTarget::Resolve(_) => {
  1501	                return Err(eyre::Report::new(SessionFault::internal(
  1502	                    "resolver target produced no destination root",
  1503	                )));
  1504	            }
  1505	        },
  1506	    };
  1507	
  1508	    drive_destination(
  1509	        &mut transport,
  1510	        negotiated,
  1511	        &dst_root,
  1512	        cfg.data_plane_host.as_deref(),
  1513	    )
  1514	    .await
  1515	}
  1516	
  1517	/// The DESTINATION session body: run the diff/receive loop and map a
  1518	/// fault to a peer-notified report. Shared by [`run_destination`] and
  1519	/// [`run_responder`] (the daemon DESTINATION responder), so the receive
  1520	/// choreography is single-sourced.
  1521	async fn drive_destination(
  1522	    transport: &mut FrameTransport,
  1523	    negotiated: Negotiated,
  1524	    dst_root: &Path,
  1525	    data_plane_host: Option<&str>,
  1526	) -> Result<DestinationOutcome> {
  1527	    match destination_session(transport, negotiated, dst_root, data_plane_host).await {
  1528	        Ok(outcome) => Ok(outcome),
  1529	        Err(report) => {
  1530	            let mut fault = fault_from_report(report);
  1531	            if !fault.peer_notified {
  1532	                let _ = transport.send(error_frame(&fault)).await;
  1533	                fault.peer_notified = true;
  1534	            }
  1535	            Err(eyre::Report::new(fault))
  1536	        }
  1537	    }
  1538	}
  1539	
  1540	/// Serve one transfer session as the RESPONDER, dispatching on the
  1541	/// initiator's declared role — the daemon's single serving entry
  1542	/// (contract §Invariants 3: one handshake, roles not directions). A
  1543	/// client that declares SOURCE makes this end the DESTINATION
  1544	/// (push-equivalent, otp-4); a client that declares DESTINATION makes
  1545	/// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
  1546	/// the endpoint resolution for each role; only the one the initiator
  1547	/// selects is used. Returns a [`ResponderOutcome`] tagged with the role
  1548	/// that ran.
  1549	pub async fn run_responder(
  1550	    hello: HelloConfig,
  1551	    transport: FrameTransport,
  1552	    source_target: SourceResponderTarget,
  1553	    dest_target: DestinationTarget,
  1554	) -> Result<ResponderOutcome> {
  1555	    let mut transport = transport;
  1556	    exchange_hello(&mut transport, &hello).await?;
  1557	    let open = match expect_frame(&mut transport).await? {
  1558	        Frame::Open(o) => o,
  1559	        other => {
  1560	            return Err(notify_and_wrap(
  1561	                &mut transport,
  1562	                SessionFault::protocol_violation(format!(
  1563	                    "expected SessionOpen, got {}",
  1564	                    frame_name(&Some(other))
  1565	                )),
  1566	            )
  1567	            .await)
  1568	        }
  1569	    };
  1570	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
  1571	    match declared {
  1572	        // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
  1573	        TransferRole::Source => {
  1574	            let resolve = match &dest_target {
  1575	                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1576	                DestinationTarget::Fixed(_) => None,
  1577	            };
  1578	            let negotiated = responder_finish(
  1579	                &mut transport,
  1580	                open,
  1581	                TransferRole::Destination,
  1582	                &destination_open_validator,
  1583	                resolve,
  1584	            )
  1585	            .await?;
  1586	            let dst_root = match negotiated.resolved_root.clone() {
  1587	                Some(root) => root,
  1588	                None => match &dest_target {
  1589	                    DestinationTarget::Fixed(root) => root.clone(),
  1590	                    DestinationTarget::Resolve(_) => {
  1591	                        return Err(eyre::Report::new(SessionFault::internal(
  1592	                            "resolver target produced no destination root",
  1593	                        )));
  1594	                    }
  1595	                },
  1596	            };
  1597	            // A DESTINATION responder (push) binds+accepts its receive
  1598	            // sockets — it never dials, so it needs no data-plane host.
  1599	            let outcome = drive_destination(&mut transport, negotiated, &dst_root, None).await?;
  1600	            Ok(ResponderOutcome::Destination(outcome))
  1601	        }
  1602	        // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
  1603	        TransferRole::Destination => {
  1604	            let resolve = match &source_target {
  1605	                SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1606	                SourceResponderTarget::Fixed(_) => None,
  1607	            };
  1608	            let negotiated = responder_finish(
  1609	                &mut transport,
  1610	                open,
  1611	                TransferRole::Source,
  1612	                &source_open_validator,
  1613	                resolve,
  1614	            )
  1615	            .await?;
  1616	            let source: Arc<dyn TransferSource> = match source_target {
  1617	                SourceResponderTarget::Fixed(source) => source,
  1618	                SourceResponderTarget::Resolve(_) => {
  1619	                    // A Resolve target always yields a root on the
  1620	                    // Responder branch (establish only skips resolution
  1621	                    // on the Initiator branch, which uses Fixed).
  1622	                    let root = negotiated.resolved_root.clone().ok_or_else(|| {
  1623	                        eyre::Report::new(SessionFault::internal(
  1624	                            "resolver target produced no source root",
  1625	                        ))
  1626	                    })?;
  1627	                    Arc::new(FsTransferSource::new(root))
  1628	                }
  1629	            };
  1630	            // The SOURCE owns its planner knobs; a daemon-served source
  1631	            // has no client-supplied ones (§Transport selection). A SOURCE
  1632	            // responder binds+accepts its send sockets (otp-5b) — it never
  1633	            // dials, so it needs no data-plane host.
  1634	            let summary =
  1635	                drive_source(PlanOptions::default(), None, negotiated, transport, source).await?;
  1636	            Ok(ResponderOutcome::Source(summary))
  1637	        }
  1638	        TransferRole::Unspecified => Err(notify_and_wrap(
  1639	            &mut transport,
  1640	            SessionFault::protocol_violation(
  1641	                "initiator declared no role (TRANSFER_ROLE_UNSPECIFIED)",
  1642	            ),
  1643	        )
  1644	        .await),
  1645	    }
  1646	}
  1647	
  1648	fn violation(message: String) -> eyre::Report {
  1649	    eyre::Report::new(SessionFault::protocol_violation(message))
  1650	}
  1651	
  1652	async fn destination_session(
  1653	    transport: &mut FrameTransport,
  1654	    negotiated: Negotiated,
  1655	    dst_root: &Path,
  1656	    data_plane_host: Option<&str>,
  1657	) -> Result<DestinationOutcome> {
  1658	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  1659	        .unwrap_or(ComparisonMode::Unspecified);
  1660	    let compare_opts = CompareOptions {
  1661	        mode: compare_mode.into(),
  1662	        ignore_existing: negotiated.open.ignore_existing,
  1663	        include_deletions: false, // mirror lands at otp-6
  1664	    };
  1665	    // src_root is only consumed by local File payloads, which never
  1666	    // occur on a session destination (payload bytes arrive as records
  1667	    // and go through the stream/tar write paths). `Arc` so the data-plane
  1668	    // receive task (otp-4b) can share the one sink across sockets.
  1669	    let sink = Arc::new(FsTransferSink::new(
  1670	        PathBuf::new(),
  1671	        dst_root.to_path_buf(),
  1672	        FsSinkConfig {
  1673	            preserve_times: true,
  1674	            dry_run: false,
  1675	            checksum: None,
  1676	            resume: false,
  1677	            compare_mode,
  1678	        },
  1679	    ));
  1680	    // Same canonical-containment chokepoint the sink write paths use
  1681	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  1682	    // make the destination stat outside its root.
  1683	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  1684	
  1685	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  1686	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  1687	    // insert-only, never removed, so a concurrent data-plane claim can
  1688	    // never re-open a grant (a duplicate manifest path is granted at
  1689	    // most once regardless of delivery timing). `outstanding` is the
  1690	    // not-yet-delivered COMPLETION set — inserted for each freshly
  1691	    // granted path before its NeedBatch, claimed by both carriers (the
  1692	    // in-stream arms inline, the data-plane NeedListSink as payloads
  1693	    // land), and empty at SourceDone. A count proxy was insufficient
  1694	    // (F1); merging the two into one set raced the data-plane claim
  1695	    // against the diff (fix-review F1).
  1696	    let mut granted: HashSet<String> = HashSet::new();
  1697	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  1698	
  1699	    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
  1700	    // bytes arrive on sockets (not the control lane). Set it up NOW —
  1701	    // concurrent with the diff loop below, and before the peer sends — so
  1702	    // the connections are established promptly. Which end connects depends
  1703	    // on connection role (otp-5b): a DESTINATION **responder** (push)
  1704	    // accepts sockets off its bound listener; a DESTINATION **initiator**
  1705	    // (pull) dials the grant it received on `data_plane_host`. Byte
  1706	    // direction is the same either way (DESTINATION receives). The
  1707	    // NeedListSink gives the socket receive the same need-list strictness
  1708	    // the in-stream control loop applies inline; AbortOnDrop (inside the
  1709	    // responder run) bounds the accept task to this future. `resize_live`
  1710	    // tracks the stream count this end has granted (epoch-0 plus each
  1711	    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
  1712	    // max_streams — both meaningful only for the resize-armable responder
  1713	    // path (push); the pull initiator path is single-stream (otp-5b-1).
  1714	    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  1715	        Arc::clone(&sink) as Arc<dyn TransferSink>,
  1716	        Arc::clone(&outstanding),
  1717	    ));
  1718	    let (mut data_plane_recv, mut resize_live, resize_ceiling) = match negotiated
  1719	        .responder_data_plane
  1720	    {
  1721	        // DESTINATION responder (push, otp-4b): accept + receive.
  1722	        Some(rdp) => {
  1723	            let initial = rdp.initial_streams() as usize;
  1724	            let run = rdp.spawn(recv_sink);
  1725	            let ceiling = run.ceiling;
  1726	            (
  1727	                Some(data_plane::DestRecvPlane::Responder(run)),
  1728	                initial,
  1729	                ceiling,
  1730	            )
  1731	        }
  1732	        // DESTINATION initiator (pull, otp-5b): dial + receive when the
  1733	        // SOURCE responder granted a data plane and we have a host to
  1734	        // dial; otherwise the in-stream carrier.
  1735	        None => match (&negotiated.accept.data_plane, data_plane_host) {
  1736	            (Some(grant), Some(host)) => {
  1737	                let run = data_plane::dial_destination_data_plane(host, grant, recv_sink).await?;
  1738	                // Single-stream (otp-5b-1): no resize is accepted, so
  1739	                // the ceiling stays 0 and a Resize frame is a violation.
  1740	                (
  1741	                    Some(data_plane::DestRecvPlane::Initiator(run)),
  1742	                    0usize,
  1743	                    0usize,
  1744	                )
  1745	            }
  1746	            _ => (None, 0usize, 0usize),
  1747	        },
  1748	    };
  1749	
  1750	    let mut pending: Vec<FileHeader> = Vec::new();
  1751	    let mut needed_paths: Vec<String> = Vec::new();
  1752	    let mut manifest_complete = false;
  1753	    let mut files_written: u64 = 0;
  1754	    let mut bytes_written: u64 = 0;
  1755	
  1756	    loop {
  1757	        let received = match transport.recv().await? {
  1758	            Some(f) => f,
  1759	            None => {
  1760	                return Err(eyre::Report::new(SessionFault::internal(
  1761	                    "peer closed mid-session",
  1762	                )))
  1763	            }
  1764	        };
  1765	        match received.frame {
  1766	            Some(Frame::ManifestEntry(header)) => {
  1767	                if manifest_complete {
  1768	                    return Err(violation(format!(
  1769	                        "manifest entry '{}' after ManifestComplete",
  1770	                        header.relative_path
  1771	                    )));
  1772	                }
  1773	                pending.push(header);
  1774	                if pending.len() >= DEST_DIFF_CHUNK {
  1775	                    let chunk = std::mem::take(&mut pending);
  1776	                    diff_chunk_and_send_needs(
  1777	                        transport,
  1778	                        chunk,
  1779	                        dst_root,
  1780	                        canonical_dst_root.as_deref(),
  1781	                        &compare_opts,
  1782	                        &mut granted,
  1783	                        &outstanding,
  1784	                        &mut needed_paths,
  1785	                    )
  1786	                    .await?;
  1787	                }
  1788	            }
  1789	            Some(Frame::ManifestComplete(_complete)) => {
  1790	                if manifest_complete {
  1791	                    return Err(violation("duplicate ManifestComplete".into()));
  1792	                }
  1793	                // (scan_complete gates mirror purges from otp-6 on;
  1794	                // nothing consumes it in otp-3.)
  1795	                let chunk = std::mem::take(&mut pending);
  1796	                diff_chunk_and_send_needs(
  1797	                    transport,
  1798	                    chunk,
  1799	                    dst_root,
  1800	                    canonical_dst_root.as_deref(),
  1801	                    &compare_opts,
  1802	                    &mut granted,
  1803	                    &outstanding,
  1804	                    &mut needed_paths,
  1805	                )
  1806	                .await?;
  1807	                // NeedComplete only after ManifestComplete received
  1808	                // AND every entry diffed — both true here.
  1809	                transport
  1810	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  1811	                    .await?;
  1812	                manifest_complete = true;
  1813	            }
  1814	            Some(Frame::FileBegin(header)) => {
  1815	                // Payload records ride the control lane only under the
  1816	                // in-stream carrier; with a TCP data plane active they
  1817	                // flow over the sockets, so one here is a violation.
  1818	                if data_plane_recv.is_some() {
  1819	                    return Err(violation(format!(
  1820	                        "file record '{}' on the control lane while a TCP data plane is active",
  1821	                        header.relative_path
  1822	                    )));
  1823	                }
  1824	                if !manifest_complete {
  1825	                    return Err(violation(format!(
  1826	                        "payload record for '{}' before ManifestComplete",
  1827	                        header.relative_path
  1828	                    )));
  1829	                }
  1830	                if !outstanding
  1831	                    .lock()
  1832	                    .expect("outstanding-needs lock poisoned")
  1833	                    .remove(&header.relative_path)
  1834	                {
  1835	                    return Err(violation(format!(
  1836	                        "payload for '{}' which is not on the need list",
  1837	                        header.relative_path
  1838	                    )));
  1839	                }
  1840	                let outcome = receive_file_record(transport, &sink, &header).await?;
  1841	                files_written += outcome.files_written as u64;
  1842	                bytes_written += outcome.bytes_written;
  1843	            }
  1844	            Some(Frame::TarShardHeader(shard)) => {
  1845	                if data_plane_recv.is_some() {
  1846	                    return Err(violation(
  1847	                        "tar shard record on the control lane while a TCP data plane is active"
  1848	                            .into(),
  1849	                    ));
  1850	                }
  1851	                if !manifest_complete {
  1852	                    return Err(violation("tar shard record before ManifestComplete".into()));
  1853	                }
  1854	                {
  1855	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  1856	                    for h in &shard.files {
  1857	                        if !out.remove(&h.relative_path) {
  1858	                            return Err(violation(format!(
  1859	                                "tar shard entry '{}' which is not on the need list",
  1860	                                h.relative_path
  1861	                            )));
  1862	                        }
  1863	                    }
  1864	                }
  1865	                let outcome = receive_tar_record(transport, &sink, shard).await?;
  1866	                files_written += outcome.files_written as u64;
  1867	                bytes_written += outcome.bytes_written;
  1868	            }
  1869	            Some(Frame::Resize(resize)) => {
  1870	                // sf-2 shape correction (otp-4b-2): the SOURCE proposes
  1871	                // one ADD; arm the credential, grant it (bump `resize_live`),
  1872	                // and ack so the SOURCE dials the epoch-N socket. Only ADD
  1873	                // occurs on the session (REMOVE is a tuner concern, future
  1874	                // work); anything else fails fast.
  1875	                let run = match data_plane_recv.as_ref() {
  1876	                    Some(data_plane::DestRecvPlane::Responder(run)) => run,
  1877	                    // The pull data plane is single-stream (otp-5b-1): the
  1878	                    // SOURCE responder never proposes a resize, so one here
  1879	                    // is a protocol violation (otp-5b-2 adds the accept-based
  1880	                    // epoch-N socket + dial).
  1881	                    Some(data_plane::DestRecvPlane::Initiator(_)) => {
  1882	                        return Err(violation(
  1883	                            "DataPlaneResize on the single-stream pull data plane (otp-5b-1)"
  1884	                                .into(),
  1885	                        ))
  1886	                    }
  1887	                    None => {
  1888	                        return Err(violation(
  1889	                            "DataPlaneResize on a session with no data plane".into(),
  1890	                        ))
  1891	                    }
  1892	                };
  1893	                let op = DataPlaneResizeOp::try_from(resize.op)
  1894	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  1895	                if op != DataPlaneResizeOp::Add {
  1896	                    return Err(violation(format!(
  1897	                        "unsupported data-plane resize op {}",
  1898	                        op.as_str_name()
  1899	                    )));
  1900	                }
  1901	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  1902	                    return Err(violation(
  1903	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  1904	                    ));
  1905	                }
  1906	                // Cumulative ceiling bound (defense in depth — the
  1907	                // source's dial already clamps to the same profile).
  1908	                let accepted = resize_live < resize_ceiling && run.arm(resize.sub_token.clone());
  1909	                if accepted {
  1910	                    resize_live += 1;
  1911	                }
  1912	                let effective = if accepted {
  1913	                    resize.target_stream_count
  1914	                } else {
  1915	                    resize_live as u32
  1916	                };
  1917	                transport
  1918	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  1919	                        epoch: resize.epoch,
  1920	                        effective_stream_count: effective,
  1921	                        accepted,
  1922	                    })))
  1923	                    .await?;
  1924	            }
  1925	            Some(Frame::SourceDone(_)) => {
  1926	                if !manifest_complete {
  1927	                    return Err(violation("SourceDone before ManifestComplete".into()));
  1928	                }
  1929	                // Completion, both carriers: the shared `outstanding`
  1930	                // set must be empty (every granted need claimed exactly
  1931	                // once). In-stream claims inline above; the data-plane
  1932	                // NeedListSink claims as payloads land, so joining the
  1933	                // receive task first drains the last of them (and
  1934	                // surfaces any receive error / stall). Set membership —
  1935	                // not a file count — is the contract (codex F1: a count
  1936	                // proxy let a peer substitute or duplicate paths).
  1937	                // `finish()` drops the arm sender (no more resizes), joins
  1938	                // the accept loop, and reports the settled stream count.
  1939	                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
  1940	                    Some(run) => {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/session_client.rs | sed -n '1,260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '560,1040p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Client-side entry for initiating a unified transfer session.
     2	//!
     3	//! [`run_push_session`] declares the SOURCE role (push-equivalent,
     4	//! otp-4): open the `Transfer` RPC, stream the manifest + payloads; the
     5	//! daemon answers as the DESTINATION Responder. [`run_pull_session`]
     6	//! declares the DESTINATION role (pull-equivalent, otp-5a): the daemon
     7	//! answers as the SOURCE Responder and streams its module tree, which
     8	//! this end diffs and writes. Both build a gRPC-backed [`FrameTransport`]
     9	//! over `BlitClient::transfer` and run the matching role driver; role is
    10	//! carried in `SessionOpen.initiator_role`, never a second code path.
    11	//!
    12	//! Not yet wired to CLI verbs — the verbs keep riding the old paths
    13	//! until the otp-10 cutover; today the parity tests drive this. Both push
    14	//! (otp-4b) and pull (otp-5b) default to the TCP data plane; the in-stream
    15	//! carrier is the requested fallback either direction.
    16	
    17	use std::path::PathBuf;
    18	use std::sync::Arc;
    19	use std::time::Duration;
    20	
    21	use eyre::{eyre, Result};
    22	use tokio::sync::mpsc;
    23	use tokio_stream::wrappers::ReceiverStream;
    24	use tonic::transport::{Channel, Endpoint};
    25	
    26	use crate::generated::blit_client::BlitClient;
    27	use crate::generated::{ComparisonMode, SessionOpen, TransferRole, TransferSummary};
    28	use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
    29	use crate::remote::transfer::source::TransferSource;
    30	use crate::transfer_plan::PlanOptions;
    31	use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
    32	use crate::transfer_session::{
    33	    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    34	    HelloConfig, SessionEndpoint, SourceSessionConfig,
    35	};
    36	
    37	/// The push-shaped subset of session options otp-4a/4b supports. Mirror,
    38	/// filters, and resume are refused at OPEN until their slices land
    39	/// (otp-6/otp-7), so they are intentionally absent here.
    40	pub struct PushSessionOptions {
    41	    pub compare_mode: ComparisonMode,
    42	    pub ignore_existing: bool,
    43	    pub require_complete_scan: bool,
    44	    pub plan_options: PlanOptions,
    45	    /// Force the in-stream byte carrier instead of the TCP data plane
    46	    /// (otp-4b). Default `false` = the responder grants a data plane and
    47	    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
    48	    /// data-plane fallback (`--force-grpc`-shaped).
    49	    pub in_stream_bytes: bool,
    50	}
    51	
    52	impl Default for PushSessionOptions {
    53	    fn default() -> Self {
    54	        Self {
    55	            compare_mode: ComparisonMode::SizeMtime,
    56	            ignore_existing: false,
    57	            require_complete_scan: false,
    58	            plan_options: PlanOptions::default(),
    59	            in_stream_bytes: false,
    60	        }
    61	    }
    62	}
    63	
    64	/// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
    65	/// session pushing `source`'s tree into the endpoint's module/path.
    66	/// Returns the destination-computed [`TransferSummary`] (contract:
    67	/// DESTINATION is the scorer).
    68	pub async fn run_push_session(
    69	    endpoint: &RemoteEndpoint,
    70	    source: Arc<dyn TransferSource>,
    71	    options: PushSessionOptions,
    72	) -> Result<TransferSummary> {
    73	    // The responder resolves module→root; the initiator's own local
    74	    // path never crosses the wire (contract §SessionOpen).
    75	    let (module, path) = endpoint_module_path(endpoint)?;
    76	
    77	    let mut client = connect_transfer_client(endpoint).await?;
    78	
    79	    let open = SessionOpen {
    80	        initiator_role: TransferRole::Source as i32,
    81	        module,
    82	        path,
    83	        compare_mode: options.compare_mode as i32,
    84	        ignore_existing: options.ignore_existing,
    85	        require_complete_scan: options.require_complete_scan,
    86	        // otp-4b: default to the TCP data plane; the responder grants it
    87	        // in SessionAccept unless this asks for the in-stream fallback.
    88	        in_stream_bytes: options.in_stream_bytes,
    89	        ..Default::default()
    90	    };
    91	
    92	    // Open the bidi RPC: the request stream is fed by `out_tx`, the
    93	    // response stream is the inbound half. The handler returns its
    94	    // response stream immediately (it spawns the session), so this
    95	    // await resolves before any frame flows — no deadlock.
    96	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
    97	    let inbound = client
    98	        .transfer(ReceiverStream::new(out_rx))
    99	        .await
   100	        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
   101	        .into_inner();
   102	    let transport = grpc_client_transport(out_tx, inbound);
   103	
   104	    let cfg = SourceSessionConfig {
   105	        hello: HelloConfig::default(),
   106	        endpoint: SessionEndpoint::initiator(open),
   107	        plan_options: options.plan_options,
   108	        // The initiator dials the data plane on the same host it reached
   109	        // the control plane on (contract §Transport: initiator dials).
   110	        data_plane_host: Some(endpoint.host.clone()),
   111	    };
   112	    run_source(cfg, transport, source).await
   113	}
   114	
   115	/// The pull-shaped subset of session options otp-5a supports. Mirror,
   116	/// filters, and resume are refused at OPEN until their slices land, so
   117	/// they are intentionally absent here. The DESTINATION owns the compare
   118	/// decision; the SOURCE owns the planner knobs (none cross the wire).
   119	pub struct PullSessionOptions {
   120	    pub compare_mode: ComparisonMode,
   121	    pub ignore_existing: bool,
   122	    pub require_complete_scan: bool,
   123	    /// Force the in-stream byte carrier instead of the TCP data plane
   124	    /// (otp-5b). Default `false` = the SOURCE responder grants a data
   125	    /// plane and this DESTINATION initiator dials + receives over TCP
   126	    /// sockets; `true` is the diagnostics / unreachable data-plane
   127	    /// fallback. Symmetric with [`PushSessionOptions::in_stream_bytes`].
   128	    pub in_stream_bytes: bool,
   129	}
   130	
   131	impl Default for PullSessionOptions {
   132	    fn default() -> Self {
   133	        Self {
   134	            compare_mode: ComparisonMode::SizeMtime,
   135	            ignore_existing: false,
   136	            require_complete_scan: false,
   137	            in_stream_bytes: false,
   138	        }
   139	    }
   140	}
   141	
   142	/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
   143	/// session pulling the endpoint's module/path tree into `dest_root`
   144	/// (pull-equivalent, otp-5a). The client initiates and declares
   145	/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
   146	/// its module tree). Returns the [`DestinationOutcome`] this end
   147	/// computed (contract: the DESTINATION is the scorer).
   148	///
   149	/// otp-5b: the default carrier is the TCP data plane — the SOURCE
   150	/// responder binds+grants+accepts sockets while sending, and this
   151	/// DESTINATION initiator dials + receives over them (the transport/role
   152	/// decoupling). `PullSessionOptions::in_stream_bytes` forces the in-stream
   153	/// fallback (diagnostics / unreachable data plane). Not wired to CLI verbs
   154	/// (otp-10).
   155	pub async fn run_pull_session(
   156	    endpoint: &RemoteEndpoint,
   157	    dest_root: PathBuf,
   158	    options: PullSessionOptions,
   159	) -> Result<DestinationOutcome> {
   160	    let (module, path) = endpoint_module_path(endpoint)?;
   161	
   162	    let mut client = connect_transfer_client(endpoint).await?;
   163	
   164	    let open = SessionOpen {
   165	        initiator_role: TransferRole::Destination as i32,
   166	        module,
   167	        path,
   168	        compare_mode: options.compare_mode as i32,
   169	        ignore_existing: options.ignore_existing,
   170	        require_complete_scan: options.require_complete_scan,
   171	        // otp-5b: default to the TCP data plane; the SOURCE responder
   172	        // grants it in SessionAccept unless this asks for the in-stream
   173	        // fallback.
   174	        in_stream_bytes: options.in_stream_bytes,
   175	        ..Default::default()
   176	    };
   177	
   178	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   179	    let inbound = client
   180	        .transfer(ReceiverStream::new(out_rx))
   181	        .await
   182	        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
   183	        .into_inner();
   184	    let transport = grpc_client_transport(out_tx, inbound);
   185	
   186	    let cfg = DestinationSessionConfig {
   187	        hello: HelloConfig::default(),
   188	        endpoint: SessionEndpoint::initiator(open),
   189	        // The initiator dials the data plane on the same host it reached
   190	        // the control plane on (contract §Transport: initiator dials).
   191	        data_plane_host: Some(endpoint.host.clone()),
   192	    };
   193	    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
   194	}
   195	
   196	/// Derive the wire `(module, path)` from a resolved endpoint. Empty
   197	/// module targets the daemon's default root export; a discovery-form
   198	/// endpoint is not resolvable to a transfer target.
   199	fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
   200	    match &endpoint.path {
   201	        RemotePath::Module { module, rel_path } => {
   202	            Ok((module.clone(), rel_path.to_string_lossy().into_owned()))
   203	        }
   204	        RemotePath::Root { rel_path } => {
   205	            Ok((String::new(), rel_path.to_string_lossy().into_owned()))
   206	        }
   207	        RemotePath::Discovery => Err(eyre!(
   208	            "a transfer session needs a resolved module or root endpoint, not a discovery form"
   209	        )),
   210	    }
   211	}
   212	
   213	/// Build a `BlitClient` over `endpoint`'s control-plane URI with the
   214	/// same bounded-connect policy `RemotePushClient::connect` uses.
   215	async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
   216	    let uri = endpoint.control_plane_uri();
   217	    let conn = Endpoint::from_shared(uri.clone())
   218	        .map_err(|e| eyre!("invalid endpoint uri {uri}: {e}"))?
   219	        .connect_timeout(Duration::from_secs(30));
   220	    let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
   221	        .await
   222	        .map_err(|_| eyre!("timed out connecting to {uri}"))?
   223	        .map_err(|e| eyre!("connecting to {uri}: {e}"))?;
   224	    Ok(BlitClient::new(channel))
   225	}

 succeeded in 0ms:
   560	) -> Result<Negotiated> {
   561	    exchange_hello(transport, hello).await?;
   562	
   563	    match endpoint {
   564	        SessionEndpoint::Initiator { open } => {
   565	            let open = open.as_ref().clone();
   566	            transport.send(frame(Frame::Open(open.clone()))).await?;
   567	            let accept = match expect_frame(transport).await? {
   568	                Frame::Accept(a) => a,
   569	                other => {
   570	                    return Err(notify_and_wrap(
   571	                        transport,
   572	                        SessionFault::protocol_violation(format!(
   573	                            "expected SessionAccept, got {}",
   574	                            frame_name(&Some(other))
   575	                        )),
   576	                    )
   577	                    .await)
   578	                }
   579	            };
   580	            Ok(Negotiated {
   581	                open,
   582	                accept,
   583	                resolved_root: None,
   584	                responder_data_plane: None,
   585	            })
   586	        }
   587	        SessionEndpoint::Responder => {
   588	            let open = match expect_frame(transport).await? {
   589	                Frame::Open(o) => o,
   590	                other => {
   591	                    return Err(notify_and_wrap(
   592	                        transport,
   593	                        SessionFault::protocol_violation(format!(
   594	                            "expected SessionOpen, got {}",
   595	                            frame_name(&Some(other))
   596	                        )),
   597	                    )
   598	                    .await)
   599	                }
   600	            };
   601	            responder_finish(transport, open, local_role, validate_open, resolve_open).await
   602	        }
   603	    }
   604	}
   605	
   606	/// Receive one frame during establish; peer errors and closes become
   607	/// terminal faults.
   608	async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
   609	    match transport.recv().await? {
   610	        Some(TransferFrame {
   611	            frame: Some(Frame::Error(err)),
   612	        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
   613	        Some(TransferFrame { frame: Some(f) }) => Ok(f),
   614	        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
   615	            SessionFault::protocol_violation("frame with empty oneof"),
   616	        )),
   617	        None => Err(eyre::Report::new(SessionFault::internal(
   618	            "peer closed during session establish",
   619	        ))),
   620	    }
   621	}
   622	
   623	/// Send the fault to the peer (best effort), mark it notified, and
   624	/// wrap it for return.
   625	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   626	    let _ = transport.send(error_frame(&fault)).await;
   627	    fault.peer_notified = true;
   628	    eyre::Report::new(fault)
   629	}
   630	
   631	// ---------------------------------------------------------------------------
   632	// SOURCE driver
   633	// ---------------------------------------------------------------------------
   634	
   635	/// Events the source's receive half forwards to its send half. The
   636	/// channel is unbounded but bounded by construction: every `Need`
   637	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   638	/// fault the session), so the queue never exceeds the source's own
   639	/// manifest size — the contract's bounded-buffering rule holds.
   640	enum SourceEvent {
   641	    Need(FileHeader),
   642	    NeedComplete,
   643	    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
   644	    /// send half dials the epoch-N socket on `accepted`.
   645	    ResizeAck(DataPlaneResizeAck),
   646	    Summary(TransferSummary),
   647	    Fault(SessionFault),
   648	}
   649	
   650	/// Run the SOURCE role of one transfer session over `transport`.
   651	/// Returns the destination-computed `TransferSummary` (contract: the
   652	/// end that wrote the bytes is the end that attests to them).
   653	pub async fn run_source(
   654	    cfg: SourceSessionConfig,
   655	    transport: FrameTransport,
   656	    source: Arc<dyn TransferSource>,
   657	) -> Result<TransferSummary> {
   658	    let mut transport = transport;
   659	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   660	        // Own-config coherence: a source initiator declares SOURCE.
   661	        let declared = TransferRole::try_from(open.initiator_role);
   662	        if declared != Ok(TransferRole::Source) {
   663	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   664	        }
   665	        if let Err(fault) = source_open_validator(open) {
   666	            eyre::bail!("run_source initiator config unsupported: {fault}");
   667	        }
   668	    }
   669	
   670	    let negotiated = establish(
   671	        &mut transport,
   672	        &cfg.hello,
   673	        &cfg.endpoint,
   674	        TransferRole::Source,
   675	        &source_open_validator,
   676	        // run_source only ever resolves nothing: a SOURCE *initiator*
   677	        // owns its own root, and a SOURCE *responder* driven directly
   678	        // (the in-process role suite) is handed a Fixed source. The
   679	        // daemon SOURCE responder resolves module→root inside
   680	        // `run_responder`, not here (otp-5).
   681	        None,
   682	    )
   683	    .await?;
   684	
   685	    drive_source(
   686	        cfg.plan_options,
   687	        cfg.data_plane_host,
   688	        negotiated,
   689	        transport,
   690	        source,
   691	    )
   692	    .await
   693	}
   694	
   695	/// The SOURCE session body after establish: spawn the receive half,
   696	/// run the send half, and map a fault to a peer-notified report. Shared
   697	/// by [`run_source`] (initiator or direct-responder) and
   698	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
   699	/// choreography is single-sourced.
   700	async fn drive_source(
   701	    plan_options: PlanOptions,
   702	    data_plane_host: Option<String>,
   703	    mut negotiated: Negotiated,
   704	    transport: FrameTransport,
   705	    source: Arc<dyn TransferSource>,
   706	) -> Result<TransferSummary> {
   707	    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
   708	    // its send sockets on; a SOURCE initiator (push) has none and dials the
   709	    // grant it received instead. Take it here so the send half owns it.
   710	    let responder_data_plane = negotiated.responder_data_plane.take();
   711	    let (mut tx, rx) = transport.split();
   712	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   713	    // Set by the send half the moment ManifestComplete goes out. On
   714	    // an ordered transport, a NeedComplete arriving while this is
   715	    // still false is provably premature — the peer cannot have
   716	    // received what we have not sent (contract: NeedComplete only
   717	    // after ManifestComplete received + all entries diffed).
   718	    let manifest_sent = Arc::new(AtomicBool::new(false));
   719	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   720	    // AbortOnDrop: an early error return below must abort the receive
   721	    // half instead of leaking it (same rationale as design-2 / w4-1).
   722	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   723	        rx,
   724	        Arc::clone(&sent),
   725	        Arc::clone(&manifest_sent),
   726	        event_tx,
   727	    )));
   728	
   729	    match source_send_half(
   730	        plan_options,
   731	        data_plane_host.as_deref(),
   732	        &negotiated,
   733	        responder_data_plane,
   734	        &mut tx,
   735	        source,
   736	        sent,
   737	        &manifest_sent,
   738	        event_rx,
   739	    )
   740	    .await
   741	    {
   742	        Ok(summary) => Ok(summary),
   743	        Err(report) => {
   744	            let mut fault = fault_from_report(report);
   745	            if !fault.peer_notified {
   746	                let _ = tx.send(error_frame(&fault)).await;
   747	                fault.peer_notified = true;
   748	            }
   749	            Err(eyre::Report::new(fault))
   750	        }
   751	    }
   752	}
   753	
   754	/// Receive half of the source driver: drains the transport for the
   755	/// whole session so destination sends can never deadlock against a
   756	/// blocked source send, and routes the destination lane to the send
   757	/// half. Terminates on summary, error, close, or violation.
   758	async fn source_recv_half(
   759	    mut rx: Box<dyn FrameRx>,
   760	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   761	    manifest_sent: Arc<AtomicBool>,
   762	    events: mpsc::UnboundedSender<SourceEvent>,
   763	) {
   764	    loop {
   765	        let received = match rx.recv().await {
   766	            Ok(Some(f)) => f,
   767	            Ok(None) => {
   768	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   769	                    "peer closed before TransferSummary",
   770	                )));
   771	                return;
   772	            }
   773	            Err(err) => {
   774	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
   775	                    "transport receive failed: {err:#}"
   776	                ))));
   777	                return;
   778	            }
   779	        };
   780	        match received.frame {
   781	            Some(Frame::NeedBatch(batch)) => {
   782	                for entry in batch.entries {
   783	                    if entry.resume {
   784	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   785	                            format!(
   786	                                "resume-flagged need for '{}' in a session opened without resume",
   787	                                entry.relative_path
   788	                            ),
   789	                        )));
   790	                        return;
   791	                    }
   792	                    let header = sent
   793	                        .lock()
   794	                        .expect("sent-manifest lock poisoned")
   795	                        .remove(&entry.relative_path);
   796	                    match header {
   797	                        Some(h) => {
   798	                            let _ = events.send(SourceEvent::Need(h));
   799	                        }
   800	                        None => {
   801	                            let _ = events.send(SourceEvent::Fault(
   802	                                SessionFault::protocol_violation(format!(
   803	                                    "need for unknown or already-needed path '{}'",
   804	                                    entry.relative_path
   805	                                )),
   806	                            ));
   807	                            return;
   808	                        }
   809	                    }
   810	                }
   811	            }
   812	            Some(Frame::NeedComplete(_)) => {
   813	                if !manifest_sent.load(Ordering::Acquire) {
   814	                    // Fail fast at arrival time (otp-3 codex F2): the
   815	                    // event queue would otherwise let an early
   816	                    // NeedComplete be processed late and pass as
   817	                    // legitimate.
   818	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   819	                        "NeedComplete before the source's ManifestComplete",
   820	                    )));
   821	                    return;
   822	                }
   823	                let _ = events.send(SourceEvent::NeedComplete);
   824	            }
   825	            Some(Frame::ResizeAck(ack)) => {
   826	                // The destination's response to a shape-resize proposal
   827	                // (otp-4b-2). Forward it to the send half, which owns the
   828	                // dial and dials the epoch-N socket on `accepted`.
   829	                let _ = events.send(SourceEvent::ResizeAck(ack));
   830	            }
   831	            Some(Frame::Summary(summary)) => {
   832	                let _ = events.send(SourceEvent::Summary(summary));
   833	                return;
   834	            }
   835	            Some(Frame::Error(err)) => {
   836	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   837	                return;
   838	            }
   839	            other => {
   840	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   841	                    format!("{} on the source's receive lane", frame_name(&other)),
   842	                )));
   843	                return;
   844	            }
   845	        }
   846	    }
   847	}
   848	
   849	#[allow(clippy::too_many_arguments)]
   850	async fn source_send_half(
   851	    plan_options: PlanOptions,
   852	    data_plane_host: Option<&str>,
   853	    negotiated: &Negotiated,
   854	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
   855	    tx: &mut Box<dyn FrameTx>,
   856	    source: Arc<dyn TransferSource>,
   857	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   858	    manifest_sent: &AtomicBool,
   859	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   860	) -> Result<TransferSummary> {
   861	    let mut pending: Vec<FileHeader> = Vec::new();
   862	    let mut need_complete = false;
   863	
   864	    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
   865	    // streaming the manifest — so the peer sees the connections promptly
   866	    // rather than waiting out a bounded-accept/connect timeout while a long
   867	    // manifest streams. Which end connects depends on connection role
   868	    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
   869	    // listener; a SOURCE **initiator** (push) dials the grant it received.
   870	    // Byte direction is the same either way (SOURCE sends), so both yield a
   871	    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
   872	    // in-stream carrier (fallback), which needs no early setup.
   873	    let mut data_plane = match responder_data_plane {
   874	        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
   875	        // initiator advertised its capacity in the open (byte RECEIVER
   876	        // advertises, wherever it initiates); the accept plane is single-
   877	        // stream (otp-5b-1).
   878	        Some(bound) => Some(
   879	            data_plane::accept_source_data_plane(
   880	                bound,
   881	                negotiated.open.receiver_capacity.as_ref(),
   882	                Arc::clone(&source),
   883	            )
   884	            .await?,
   885	        ),
   886	        // SOURCE initiator (push, otp-4b): dial the grant if the responder
   887	        // granted a data plane; else in-stream.
   888	        None => match &negotiated.accept.data_plane {
   889	            Some(grant) => {
   890	                let host = data_plane_host.ok_or_else(|| {
   891	                    eyre::Report::new(SessionFault::internal(
   892	                        "responder granted a TCP data plane but this initiator has no host to dial",
   893	                    ))
   894	                })?;
   895	                Some(
   896	                    data_plane::dial_source_data_plane(
   897	                        host,
   898	                        grant,
   899	                        negotiated.accept.receiver_capacity.as_ref(),
   900	                        Arc::clone(&source),
   901	                    )
   902	                    .await?,
   903	                )
   904	            }
   905	            None => None,
   906	        },
   907	    };
   908	
   909	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
   910	    // fed to the shape table so the SOURCE grows the data-plane stream
   911	    // count as the workload's shape becomes known. Append-only (a need is
   912	    // counted once, when it arrives), and the in-flight resize record the
   913	    // ack is matched against (at most one — the dial enforces it).
   914	    let mut needed_bytes: u64 = 0;
   915	    let mut needed_count: usize = 0;
   916	    let mut pending_resize: Option<data_plane::PendingResize> = None;
   917	
   918	    // Streaming manifest: entries go out as enumeration produces them
   919	    // (immediate start in every direction — plan §Design 2). The open
   920	    // carries no source path: the source end owns its local endpoint.
   921	    let _ = &negotiated.open;
   922	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   923	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   924	    while let Some(header) = header_rx.recv().await {
   925	        sent.lock()
   926	            .expect("sent-manifest lock poisoned")
   927	            .insert(header.relative_path.clone(), header.clone());
   928	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   929	        // Faults detected by the receive half abort the stream now,
   930	        // not after the full scan; needs just accumulate. (Resize acks
   931	        // cannot arrive yet — none is proposed before the payload phase.)
   932	        drain_ready_source_events(
   933	            &mut events,
   934	            &mut pending,
   935	            &mut need_complete,
   936	            &mut needed_bytes,
   937	            &mut needed_count,
   938	            data_plane.as_ref(),
   939	            tx,
   940	            &mut pending_resize,
   941	        )
   942	        .await?;
   943	    }
   944	    let scanned = scan_handle
   945	        .await
   946	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   947	    let scan_complete = unreadable
   948	        .lock()
   949	        .expect("unreadable list lock poisoned")
   950	        .is_empty();
   951	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   952	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   953	        scan_complete,
   954	    })))
   955	    .await?;
   956	    manifest_sent.store(true, Ordering::Release);
   957	
   958	    // Payload phase. The byte carrier is either the TCP data plane
   959	    // (dialed above) or the in-stream record grammar (fallback). Needs
   960	    // accumulated while a batch was being sent become the next planner
   961	    // batch (contract §Transport selection); payloads only flow after
   962	    // ManifestComplete.
   963	    // The in-stream carrier reuses one read buffer across records; the
   964	    // data plane owns its own pooled buffers, so skip that allocation.
   965	    let mut read_buf = if data_plane.is_none() {
   966	        vec![0u8; IN_STREAM_CHUNK]
   967	    } else {
   968	        Vec::new()
   969	    };
   970	    loop {
   971	        drain_ready_source_events(
   972	            &mut events,
   973	            &mut pending,
   974	            &mut need_complete,
   975	            &mut needed_bytes,
   976	            &mut needed_count,
   977	            data_plane.as_ref(),
   978	            tx,
   979	            &mut pending_resize,
   980	        )
   981	        .await?;
   982	        if !pending.is_empty() {
   983	            let batch = std::mem::take(&mut pending);
   984	            match &mut data_plane {
   985	                Some(dp) => {
   986	                    // sf-2: correct the stream count toward the shape the
   987	                    // accumulated need list implies before queueing this
   988	                    // batch (one ADD per epoch; a no-op while one is in
   989	                    // flight or the shape wants no more).
   990	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
   991	                        .await?;
   992	                    let payloads =
   993	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
   994	                    // A cancel while earlier batches are actively moving
   995	                    // closes the send pipeline under backpressure, so this
   996	                    // queue fails with a data-plane error — prefer the
   997	                    // peer's framed reason (CANCELLED) the same way the
   998	                    // finish() drain does (otp-4b-3 codex F1). Not raced
   999	                    // against events like finish(): live `Need`s still
  1000	                    // arrive here, and `recv_peer_fault` would consume them.
  1001	                    if let Err(dp_err) = dp.queue(payloads).await {
  1002	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1003	                    }
  1004	                }
  1005	                None => {
  1006	                    send_payload_records(tx, &source, plan_options, batch, &mut read_buf).await?;
  1007	                }
  1008	            }
  1009	            continue;
  1010	        }
  1011	        if need_complete {
  1012	            break;
  1013	        }
  1014	        match events.recv().await {
  1015	            Some(event) => {
  1016	                process_source_event(
  1017	                    event,
  1018	                    &mut pending,
  1019	                    &mut need_complete,
  1020	                    &mut needed_bytes,
  1021	                    &mut needed_count,
  1022	                    data_plane.as_ref(),
  1023	                    tx,
  1024	                    &mut pending_resize,
  1025	                )
  1026	                .await?;
  1027	            }
  1028	            None => {
  1029	                return Err(eyre::Report::new(SessionFault::internal(
  1030	                    "source receive half ended before NeedComplete",
  1031	                )))
  1032	            }
  1033	        }
  1034	    }
  1035	
  1036	    // A resize proposed on the last batch may still be in flight. Resolve
  1037	    // it BEFORE finishing so the destination's armed slot is consumed by
  1038	    // the dialed socket — an armed-but-never-dialed credential would hang
  1039	    // its accept loop (which waits for every arm to be claimed). We do not
  1040	    // propose further here: exactly the one in-flight resize is drained.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer_session_e2e.rs | sed -n '520,720p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   520	    // sole push/pull divergence is same-size + dest-NEWER. The session
   521	    // adopts the data-safe, converge-up behavior — SKIP, never clobber
   522	    // a newer destination file with older source content. (--force
   523	    // overrides; not exercised here.)
   524	    let daemon = Daemon::start(false).await;
   525	
   526	    // Seed the destination with a NEWER, same-size, different-content
   527	    // file plus a file that genuinely needs updating.
   528	    write_tree(
   529	        &daemon.dest_root,
   530	        &[
   531	            ("keep.txt", b"NEWER-destination", 1_600_100_000),
   532	            ("stale.txt", b"old-destination--", 1_600_000_000),
   533	        ],
   534	    );
   535	    let src = tempfile::tempdir().unwrap();
   536	    write_tree(
   537	        src.path(),
   538	        &[
   539	            // same size (17) as dest keep.txt, but OLDER → must be skipped.
   540	            ("keep.txt", b"older-source-here", 1_600_000_000),
   541	            // same size (17) as dest stale.txt, but NEWER → must transfer.
   542	            ("stale.txt", b"new-source-here--", 1_600_200_000),
   543	        ],
   544	    );
   545	
   546	    let summary = run_push_session(
   547	        &daemon.endpoint,
   548	        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
   549	        PushSessionOptions::default(),
   550	    )
   551	    .await
   552	    .expect("session push succeeds");
   553	
   554	    // Only stale.txt transfers; keep.txt (newer on dest) is left intact.
   555	    assert_eq!(
   556	        summary.files_transferred, 1,
   557	        "only the stale file transfers"
   558	    );
   559	    assert_eq!(
   560	        std::fs::read(daemon.dest_root.join("keep.txt")).unwrap(),
   561	        b"NEWER-destination",
   562	        "a newer same-size destination file must NOT be clobbered"
   563	    );
   564	    assert_eq!(
   565	        std::fs::read(daemon.dest_root.join("stale.txt")).unwrap(),
   566	        b"new-source-here--",
   567	        "a stale destination file must be updated"
   568	    );
   569	    daemon.stop().await;
   570	}
   571	
   572	// ---------------------------------------------------------------------------
   573	// otp-5a: pull-equivalent (client initiates as DESTINATION, daemon is SOURCE)
   574	// ---------------------------------------------------------------------------
   575	
   576	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   577	async fn pull_session_lands_bytes_over_the_data_plane() {
   578	    // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
   579	    // initiates as DESTINATION and the daemon streams its module tree. With
   580	    // otp-5b the default carrier is the TCP data plane — the daemon (SOURCE
   581	    // responder) binds+grants+accepts sockets while sending, and the client
   582	    // (DESTINATION initiator) dials + receives over them. `dest_root` here
   583	    // is the module (source) root — the harness field name is push-oriented.
   584	    let daemon = Daemon::start(false).await;
   585	    write_tree(&daemon.dest_root, &small_tree());
   586	
   587	    let dest = tempfile::tempdir().unwrap();
   588	    let outcome = run_pull_session(
   589	        &daemon.endpoint,
   590	        dest.path().to_path_buf(),
   591	        PullSessionOptions::default(),
   592	    )
   593	    .await
   594	    .expect("session pull succeeds");
   595	
   596	    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
   597	    assert_eq!(
   598	        outcome.summary.bytes_transferred,
   599	        small_tree()
   600	            .iter()
   601	            .map(|(_, c, _)| c.len() as u64)
   602	            .sum::<u64>()
   603	    );
   604	    assert!(
   605	        !outcome.summary.in_stream_carrier_used,
   606	        "otp-5b pull default rides the TCP data plane, not the in-stream carrier"
   607	    );
   608	    assert_eq!(
   609	        outcome.data_plane_streams,
   610	        Some(1),
   611	        "otp-5b-1 pull is single-stream (no resize until otp-5b-2)"
   612	    );
   613	    assert_trees_identical(&daemon.dest_root, dest.path());
   614	    daemon.stop().await;
   615	}
   616	
   617	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   618	async fn pull_session_lands_bytes_over_in_stream_carrier() {
   619	    // The in-stream carrier is the pull fallback (diagnostics / unreachable
   620	    // data plane). Requesting it must still land bytes byte-identically and
   621	    // score them — the otp-5a path stays live under otp-5b.
   622	    let daemon = Daemon::start(false).await;
   623	    write_tree(&daemon.dest_root, &small_tree());
   624	
   625	    let dest = tempfile::tempdir().unwrap();
   626	    let outcome = run_pull_session(
   627	        &daemon.endpoint,
   628	        dest.path().to_path_buf(),
   629	        PullSessionOptions {
   630	            in_stream_bytes: true,
   631	            ..PullSessionOptions::default()
   632	        },
   633	    )
   634	    .await
   635	    .expect("in-stream session pull succeeds");
   636	
   637	    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
   638	    assert!(
   639	        outcome.summary.in_stream_carrier_used,
   640	        "an in_stream_bytes request rides the in-stream carrier"
   641	    );
   642	    assert_trees_identical(&daemon.dest_root, dest.path());
   643	    daemon.stop().await;
   644	}
   645	
   646	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   647	async fn old_pull_and_session_produce_identical_trees_and_counts() {
   648	    // Arm A: OLD pull_sync into a client-local dest.
   649	    let daemon_a = Daemon::start(false).await;
   650	    write_tree(&daemon_a.dest_root, &small_tree());
   651	    let dest_a = tempfile::tempdir().unwrap();
   652	    let mut pull_client = RemotePullClient::connect(daemon_a.endpoint.clone())
   653	        .await
   654	        .expect("old pull client connects");
   655	    let report = pull_client
   656	        .pull_sync(
   657	            dest_a.path(),
   658	            Vec::new(),
   659	            &PullSyncOptions::default(),
   660	            false,
   661	            None,
   662	        )
   663	        .await
   664	        .expect("old pull succeeds");
   665	
   666	    // Arm B: NEW session (client DESTINATION initiator).
   667	    let daemon_b = Daemon::start(false).await;
   668	    write_tree(&daemon_b.dest_root, &small_tree());
   669	    let dest_b = tempfile::tempdir().unwrap();
   670	    let outcome = run_pull_session(
   671	        &daemon_b.endpoint,
   672	        dest_b.path().to_path_buf(),
   673	        PullSessionOptions::default(),
   674	    )
   675	    .await
   676	    .expect("session pull succeeds");
   677	
   678	    // Both dests equal their source module and each other.
   679	    assert_trees_identical(&daemon_a.dest_root, dest_a.path());
   680	    assert_trees_identical(&daemon_b.dest_root, dest_b.path());
   681	    assert_trees_identical(dest_a.path(), dest_b.path());
   682	
   683	    // Shared counters agree (transport-specific fields have no cross
   684	    // analog and are not compared). Old pull already SKIPs the same-size
   685	    // dest-NEWER cell, so this A/B is byte-identical with no caveat —
   686	    // unlike the push A/B where old push clobbers.
   687	    assert_eq!(
   688	        report.files_transferred as u64,
   689	        outcome.summary.files_transferred
   690	    );
   691	    assert_eq!(report.bytes_transferred, outcome.summary.bytes_transferred);
   692	
   693	    daemon_a.stop().await;
   694	    daemon_b.stop().await;
   695	}
   696	
   697	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   698	async fn unknown_module_refuses_the_pull_session() {
   699	    let daemon = Daemon::start(false).await;
   700	    let dest = tempfile::tempdir().unwrap();
   701	    let err = run_pull_session(
   702	        &daemon.endpoint_for_missing_module(),
   703	        dest.path().to_path_buf(),
   704	        PullSessionOptions::default(),
   705	    )
   706	    .await
   707	    .expect_err("unknown module must refuse the pull session");
   708	    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
   709	    daemon.stop().await;
   710	}

codex
I don’t see a push-path behavior change yet: the SOURCE-initiator still dials from `accept.data_plane`, and the DESTINATION-responder still spawns the same accept/receive runner. The remaining risk area is the new pull setup order and whether any early socket task can outlive or deadlock the control-lane driver.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '838,940p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   838	// ---------------------------------------------------------------------------
   839	// Need-list enforcement for the data-plane receive
   840	// ---------------------------------------------------------------------------
   841	
   842	/// Sink decorator that enforces the session's need-list contract on the
   843	/// data-plane receive, giving it the SAME strictness the in-stream
   844	/// carrier applies inline in the control loop (`outstanding.remove`).
   845	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   846	/// without this a peer could substitute an off-need-list path for a
   847	/// needed one (count-preserving), duplicate one, or send resume block
   848	/// records the non-resume session never negotiated (codex otp-4b-1 F1).
   849	/// Every written path must be a granted, not-yet-received need; resume
   850	/// block records are rejected outright. The shared [`OutstandingNeeds`]
   851	/// set makes completion `is_empty()` for both carriers.
   852	pub(super) struct NeedListSink {
   853	    inner: Arc<dyn TransferSink>,
   854	    outstanding: OutstandingNeeds,
   855	}
   856	
   857	impl NeedListSink {
   858	    pub(super) fn new(inner: Arc<dyn TransferSink>, outstanding: OutstandingNeeds) -> Self {
   859	        Self { inner, outstanding }
   860	    }
   861	
   862	    /// Remove `path` from the outstanding set, or fault: a path that is
   863	    /// not present is either off the need list or a duplicate delivery.
   864	    fn claim(&self, path: &str) -> Result<()> {
   865	        if self
   866	            .outstanding
   867	            .lock()
   868	            .expect("outstanding-needs lock poisoned")
   869	            .remove(path)
   870	        {
   871	            Ok(())
   872	        } else {
   873	            Err(eyre::Report::new(SessionFault::protocol_violation(
   874	                format!(
   875	                    "data-plane payload for '{path}' which is not an outstanding need \
   876	                 (off the need list, or a duplicate delivery)"
   877	                ),
   878	            )))
   879	        }
   880	    }
   881	}
   882	
   883	#[async_trait]
   884	impl TransferSink for NeedListSink {
   885	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   886	        match &payload {
   887	            PreparedPayload::File(header) => self.claim(&header.relative_path)?,
   888	            PreparedPayload::TarShard { headers, .. } => {
   889	                for header in headers {
   890	                    self.claim(&header.relative_path)?;
   891	                }
   892	            }
   893	            // The session did not negotiate resume (otp-7), so a block
   894	            // record on the data plane is a protocol violation, not a
   895	            // silently-applied patch.
   896	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   897	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   898	                    "resume block record on the data plane of a non-resume session",
   899	                )));
   900	            }
   901	        }
   902	        self.inner.write_payload(payload).await
   903	    }
   904	
   905	    async fn write_file_stream(
   906	        &self,
   907	        header: &FileHeader,
   908	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   909	    ) -> Result<SinkOutcome> {
   910	        self.claim(&header.relative_path)?;
   911	        self.inner.write_file_stream(header, reader).await
   912	    }
   913	
   914	    async fn finish(&self) -> Result<()> {
   915	        self.inner.finish().await
   916	    }
   917	
   918	    fn root(&self) -> &Path {
   919	        self.inner.root()
   920	    }
   921	}
   922	
   923	#[cfg(test)]
   924	mod tests {
   925	    use super::*;
   926	
   927	    /// The otp-4b-1 grant invariant: the responder always grants a
   928	    /// single epoch-0 stream (the zero-knowledge proposal — no manifest
   929	    /// has been seen when SessionAccept goes out) with two independent
   930	    /// 16-byte credentials on a real port. Multi-stream is resize-only
   931	    /// (otp-4b-2).
   932	    #[tokio::test]
   933	    async fn responder_grant_is_single_stream_with_16_byte_tokens() {
   934	        let rdp = prepare_responder_data_plane()
   935	            .await
   936	            .expect("bind loopback data plane");
   937	        let grant = rdp.grant();
   938	        assert_eq!(
   939	            grant.initial_streams, 1,
   940	            "zero-knowledge grant starts single-stream (otp-4b-1)"

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1938,2115p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1938	                // the accept loop, and reports the settled stream count.
  1939	                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
  1940	                    Some(run) => {
  1941	                        let totals = run.finish().await?;
  1942	                        files_written = totals.outcome.files_written as u64;
  1943	                        bytes_written = totals.outcome.bytes_written;
  1944	                        (false, Some(totals.streams))
  1945	                    }
  1946	                    None => (true, None),
  1947	                };
  1948	                let unfulfilled = outstanding
  1949	                    .lock()
  1950	                    .expect("outstanding-needs lock poisoned")
  1951	                    .len();
  1952	                if unfulfilled != 0 {
  1953	                    return Err(violation(format!(
  1954	                        "SourceDone with {unfulfilled} needed file(s) never delivered"
  1955	                    )));
  1956	                }
  1957	                let summary = TransferSummary {
  1958	                    files_transferred: files_written,
  1959	                    bytes_transferred: bytes_written,
  1960	                    entries_deleted: 0, // mirror lands at otp-6
  1961	                    in_stream_carrier_used,
  1962	                    files_resumed: 0, // resume lands at otp-7
  1963	                };
  1964	                transport.send(frame(Frame::Summary(summary))).await?;
  1965	                return Ok(DestinationOutcome {
  1966	                    summary,
  1967	                    needed_paths,
  1968	                    data_plane_streams,
  1969	                });
  1970	            }
  1971	            Some(Frame::Error(err)) => {
  1972	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  1973	            }
  1974	            other => {
  1975	                // Everything else is off-lane or off-phase here:
  1976	                // destination-lane frames echoed back (a ResizeAck the
  1977	                // destination would never receive), resume frames in a
  1978	                // non-resume session (otp-7), stray handshake frames,
  1979	                // bare FileData/TarShardChunk outside a record. Fail
  1980	                // fast, no tolerant parsing.
  1981	                return Err(violation(format!(
  1982	                    "{} not valid on the destination's receive lane in this phase",
  1983	                    frame_name(&other)
  1984	                )));
  1985	            }
  1986	        }
  1987	    }
  1988	}
  1989	
  1990	/// Stat-and-compare one chunk of manifest entries on the blocking
  1991	/// pool (2+ syscalls per entry — same rationale as the daemon's
  1992	/// w4-4 chunked checks), then stream the resulting need batch.
  1993	async fn diff_chunk_and_send_needs(
  1994	    transport: &mut FrameTransport,
  1995	    chunk: Vec<FileHeader>,
  1996	    dst_root: &Path,
  1997	    canonical_dst_root: Option<&Path>,
  1998	    compare_opts: &CompareOptions,
  1999	    // Ever-granted DEDUP set (control-loop-local, insert-only): a path
  2000	    // the source manifests twice is granted at most once, and because it
  2001	    // is never removed, a concurrent data-plane claim can't re-open the
  2002	    // grant (fix-review F1).
  2003	    granted: &mut HashSet<String>,
  2004	    // Not-yet-delivered COMPLETION set (shared with the receive).
  2005	    outstanding: &data_plane::OutstandingNeeds,
  2006	    needed_paths: &mut Vec<String>,
  2007	) -> Result<()> {
  2008	    if chunk.is_empty() {
  2009	        return Ok(());
  2010	    }
  2011	    let dst_root = dst_root.to_path_buf();
  2012	    let canonical = canonical_dst_root.map(Path::to_path_buf);
  2013	    let opts = compare_opts.clone();
  2014	    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
  2015	        let mut needed = Vec::new();
  2016	        for header in &chunk {
  2017	            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
  2018	                needed.push(header.relative_path.clone());
  2019	            }
  2020	        }
  2021	        Ok(needed)
  2022	    })
  2023	    .await
  2024	    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
  2025	
  2026	    // Dedup on the ever-granted set (no lock — control-loop-local), then
  2027	    // insert the freshly granted paths into the shared `outstanding`
  2028	    // completion set BEFORE the NeedBatch goes out. The source can only
  2029	    // send a payload after receiving its need, so insert-before-send
  2030	    // orders the data-plane receive's `claim` strictly after this insert.
  2031	    let fresh: Vec<String> = needed
  2032	        .into_iter()
  2033	        .filter(|path| granted.insert(path.clone()))
  2034	        .collect();
  2035	    let entries: Vec<NeedEntry> = {
  2036	        let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  2037	        fresh
  2038	            .into_iter()
  2039	            .map(|relative_path| {
  2040	                needed_paths.push(relative_path.clone());
  2041	                out.insert(relative_path.clone());
  2042	                NeedEntry {
  2043	                    relative_path,
  2044	                    resume: false, // resume lands at otp-7
  2045	                }
  2046	            })
  2047	            .collect()
  2048	    };
  2049	    if entries.is_empty() {
  2050	        return Ok(());
  2051	    }
  2052	    transport
  2053	        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
  2054	        .await?;
  2055	    Ok(())
  2056	}
  2057	
  2058	/// Does the destination need this manifest entry? Stats its own file
  2059	/// and delegates the verdict to `manifest::header_transfer_status` —
  2060	/// the same mode-aware owner `compare_manifests` uses, fed from a
  2061	/// live stat instead of a materialized target manifest.
  2062	fn destination_needs(
  2063	    header: &FileHeader,
  2064	    dst_root: &Path,
  2065	    canonical_dst_root: Option<&Path>,
  2066	    opts: &CompareOptions,
  2067	) -> Result<bool> {
  2068	    let dst = match canonical_dst_root {
  2069	        Some(canonical) => {
  2070	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
  2071	        }
  2072	        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
  2073	    }
  2074	    .map_err(|err| {
  2075	        SessionFault::protocol_violation(format!(
  2076	            "manifest path '{}' escapes the destination root: {err:#}",
  2077	            header.relative_path
  2078	        ))
  2079	    })?;
  2080	
  2081	    let target = match std::fs::metadata(&dst) {
  2082	        Ok(meta) if meta.is_file() => {
  2083	            let mtime = match meta.modified() {
  2084	                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
  2085	                    Ok(d) => d.as_secs() as i64,
  2086	                    Err(e) => -(e.duration().as_secs() as i64),
  2087	                },
  2088	                Err(_) => 0,
  2089	            };
  2090	            Some((meta.len(), mtime))
  2091	        }
  2092	        // Absent — or present as a directory/other, which a file
  2093	        // write must replace: both diff as "target does not have it"
  2094	        // (matches the push daemon's file_requires_upload).
  2095	        _ => None,
  2096	    };
  2097	    let status = header_transfer_status(
  2098	        header,
  2099	        // Destination-side checksums are never precomputed; Checksum
  2100	        // mode therefore transfers (the conservative arm of
  2101	        // compare_file), matching what push does today.
  2102	        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
  2103	        opts,
  2104	    );
  2105	    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
  2106	}
  2107	
  2108	/// Receive one strictly-serialized file record (`file_begin` already
  2109	/// consumed) and stream its bytes into the sink through a bounded
  2110	/// in-memory pipe — record completion is exactly `header.size`
  2111	/// cumulative bytes (contract §Transport selection).
  2112	async fn receive_file_record(
  2113	    transport: &mut FrameTransport,
  2114	    sink: &FsTransferSink,
  2115	    header: &FileHeader,

exec
/usr/bin/zsh -lc 'git show e6a0b3b''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'1500,1890p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1500	/// initiator's declared role — the daemon's single serving entry
  1501	/// (contract §Invariants 3: one handshake, roles not directions). A
  1502	/// client that declares SOURCE makes this end the DESTINATION
  1503	/// (push-equivalent, otp-4); a client that declares DESTINATION makes
  1504	/// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
  1505	/// the endpoint resolution for each role; only the one the initiator
  1506	/// selects is used. Returns a [`ResponderOutcome`] tagged with the role
  1507	/// that ran.
  1508	pub async fn run_responder(
  1509	    hello: HelloConfig,
  1510	    transport: FrameTransport,
  1511	    source_target: SourceResponderTarget,
  1512	    dest_target: DestinationTarget,
  1513	) -> Result<ResponderOutcome> {
  1514	    let mut transport = transport;
  1515	    exchange_hello(&mut transport, &hello).await?;
  1516	    let open = match expect_frame(&mut transport).await? {
  1517	        Frame::Open(o) => o,
  1518	        other => {
  1519	            return Err(notify_and_wrap(
  1520	                &mut transport,
  1521	                SessionFault::protocol_violation(format!(
  1522	                    "expected SessionOpen, got {}",
  1523	                    frame_name(&Some(other))
  1524	                )),
  1525	            )
  1526	            .await)
  1527	        }
  1528	    };
  1529	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
  1530	    match declared {
  1531	        // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
  1532	        TransferRole::Source => {
  1533	            let resolve = match &dest_target {
  1534	                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1535	                DestinationTarget::Fixed(_) => None,
  1536	            };
  1537	            let negotiated = responder_finish(
  1538	                &mut transport,
  1539	                open,
  1540	                TransferRole::Destination,
  1541	                &destination_open_validator,
  1542	                resolve,
  1543	            )
  1544	            .await?;
  1545	            let dst_root = match negotiated.resolved_root.clone() {
  1546	                Some(root) => root,
  1547	                None => match &dest_target {
  1548	                    DestinationTarget::Fixed(root) => root.clone(),
  1549	                    DestinationTarget::Resolve(_) => {
  1550	                        return Err(eyre::Report::new(SessionFault::internal(
  1551	                            "resolver target produced no destination root",
  1552	                        )));
  1553	                    }
  1554	                },
  1555	            };
  1556	            let outcome = drive_destination(&mut transport, negotiated, &dst_root).await?;
  1557	            Ok(ResponderOutcome::Destination(outcome))
  1558	        }
  1559	        // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
  1560	        TransferRole::Destination => {
  1561	            let resolve = match &source_target {
  1562	                SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1563	                SourceResponderTarget::Fixed(_) => None,
  1564	            };
  1565	            let negotiated = responder_finish(
  1566	                &mut transport,
  1567	                open,
  1568	                TransferRole::Source,
  1569	                &source_open_validator,
  1570	                resolve,
  1571	            )
  1572	            .await?;
  1573	            let source: Arc<dyn TransferSource> = match source_target {
  1574	                SourceResponderTarget::Fixed(source) => source,
  1575	                SourceResponderTarget::Resolve(_) => {
  1576	                    // A Resolve target always yields a root on the
  1577	                    // Responder branch (establish only skips resolution
  1578	                    // on the Initiator branch, which uses Fixed).
  1579	                    let root = negotiated.resolved_root.clone().ok_or_else(|| {
  1580	                        eyre::Report::new(SessionFault::internal(
  1581	                            "resolver target produced no source root",
  1582	                        ))
  1583	                    })?;
  1584	                    Arc::new(FsTransferSource::new(root))
  1585	                }
  1586	            };
  1587	            // The SOURCE owns its planner knobs; a daemon-served source
  1588	            // has no client-supplied ones (§Transport selection). otp-5a
  1589	            // is in-stream only, so there is no data-plane host to dial.
  1590	            let summary =
  1591	                drive_source(PlanOptions::default(), None, &negotiated, transport, source).await?;
  1592	            Ok(ResponderOutcome::Source(summary))
  1593	        }
  1594	        TransferRole::Unspecified => Err(notify_and_wrap(
  1595	            &mut transport,
  1596	            SessionFault::protocol_violation(
  1597	                "initiator declared no role (TRANSFER_ROLE_UNSPECIFIED)",
  1598	            ),
  1599	        )
  1600	        .await),
  1601	    }
  1602	}
  1603	
  1604	fn violation(message: String) -> eyre::Report {
  1605	    eyre::Report::new(SessionFault::protocol_violation(message))
  1606	}
  1607	
  1608	async fn destination_session(
  1609	    transport: &mut FrameTransport,
  1610	    negotiated: Negotiated,
  1611	    dst_root: &Path,
  1612	) -> Result<DestinationOutcome> {
  1613	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  1614	        .unwrap_or(ComparisonMode::Unspecified);
  1615	    let compare_opts = CompareOptions {
  1616	        mode: compare_mode.into(),
  1617	        ignore_existing: negotiated.open.ignore_existing,
  1618	        include_deletions: false, // mirror lands at otp-6
  1619	    };
  1620	    // src_root is only consumed by local File payloads, which never
  1621	    // occur on a session destination (payload bytes arrive as records
  1622	    // and go through the stream/tar write paths). `Arc` so the data-plane
  1623	    // receive task (otp-4b) can share the one sink across sockets.
  1624	    let sink = Arc::new(FsTransferSink::new(
  1625	        PathBuf::new(),
  1626	        dst_root.to_path_buf(),
  1627	        FsSinkConfig {
  1628	            preserve_times: true,
  1629	            dry_run: false,
  1630	            checksum: None,
  1631	            resume: false,
  1632	            compare_mode,
  1633	        },
  1634	    ));
  1635	    // Same canonical-containment chokepoint the sink write paths use
  1636	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  1637	    // make the destination stat outside its root.
  1638	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  1639	
  1640	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  1641	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  1642	    // insert-only, never removed, so a concurrent data-plane claim can
  1643	    // never re-open a grant (a duplicate manifest path is granted at
  1644	    // most once regardless of delivery timing). `outstanding` is the
  1645	    // not-yet-delivered COMPLETION set — inserted for each freshly
  1646	    // granted path before its NeedBatch, claimed by both carriers (the
  1647	    // in-stream arms inline, the data-plane NeedListSink as payloads
  1648	    // land), and empty at SourceDone. A count proxy was insufficient
  1649	    // (F1); merging the two into one set raced the data-plane claim
  1650	    // against the diff (fix-review F1).
  1651	    let mut granted: HashSet<String> = HashSet::new();
  1652	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  1653	
  1654	    // Data plane (otp-4b): when the responder granted a TCP data plane,
  1655	    // payload bytes arrive on sockets (not the control lane). Arm the
  1656	    // accept+receive task NOW — concurrent with the diff loop below, and
  1657	    // before the source dials — so the connections are accepted promptly.
  1658	    // The NeedListSink gives the socket receive the same need-list
  1659	    // strictness the in-stream control loop applies inline. AbortOnDrop
  1660	    // bounds it to this future: a control-lane fault that returns from
  1661	    // this fn aborts the receive task instead of leaking it.
  1662	    // `resize_live` tracks the stream count this end has granted (epoch-0
  1663	    // plus each accepted resize ADD); `resize_ceiling` is the receiver's
  1664	    // advertised max_streams, the cumulative bound a resize may not cross.
  1665	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  1666	        match negotiated.responder_data_plane {
  1667	            Some(rdp) => {
  1668	                let initial = rdp.initial_streams() as usize;
  1669	                let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  1670	                    Arc::clone(&sink) as Arc<dyn TransferSink>,
  1671	                    Arc::clone(&outstanding),
  1672	                ));
  1673	                let run = rdp.spawn(recv_sink);
  1674	                let ceiling = run.ceiling;
  1675	                (Some(run), initial, ceiling)
  1676	            }
  1677	            None => (None, 0usize, 0usize),
  1678	        };
  1679	
  1680	    let mut pending: Vec<FileHeader> = Vec::new();
  1681	    let mut needed_paths: Vec<String> = Vec::new();
  1682	    let mut manifest_complete = false;
  1683	    let mut files_written: u64 = 0;
  1684	    let mut bytes_written: u64 = 0;
  1685	
  1686	    loop {
  1687	        let received = match transport.recv().await? {
  1688	            Some(f) => f,
  1689	            None => {
  1690	                return Err(eyre::Report::new(SessionFault::internal(
  1691	                    "peer closed mid-session",
  1692	                )))
  1693	            }
  1694	        };
  1695	        match received.frame {
  1696	            Some(Frame::ManifestEntry(header)) => {
  1697	                if manifest_complete {
  1698	                    return Err(violation(format!(
  1699	                        "manifest entry '{}' after ManifestComplete",
  1700	                        header.relative_path
  1701	                    )));
  1702	                }
  1703	                pending.push(header);
  1704	                if pending.len() >= DEST_DIFF_CHUNK {
  1705	                    let chunk = std::mem::take(&mut pending);
  1706	                    diff_chunk_and_send_needs(
  1707	                        transport,
  1708	                        chunk,
  1709	                        dst_root,
  1710	                        canonical_dst_root.as_deref(),
  1711	                        &compare_opts,
  1712	                        &mut granted,
  1713	                        &outstanding,
  1714	                        &mut needed_paths,
  1715	                    )
  1716	                    .await?;
  1717	                }
  1718	            }
  1719	            Some(Frame::ManifestComplete(_complete)) => {
  1720	                if manifest_complete {
  1721	                    return Err(violation("duplicate ManifestComplete".into()));
  1722	                }
  1723	                // (scan_complete gates mirror purges from otp-6 on;
  1724	                // nothing consumes it in otp-3.)
  1725	                let chunk = std::mem::take(&mut pending);
  1726	                diff_chunk_and_send_needs(
  1727	                    transport,
  1728	                    chunk,
  1729	                    dst_root,
  1730	                    canonical_dst_root.as_deref(),
  1731	                    &compare_opts,
  1732	                    &mut granted,
  1733	                    &outstanding,
  1734	                    &mut needed_paths,
  1735	                )
  1736	                .await?;
  1737	                // NeedComplete only after ManifestComplete received
  1738	                // AND every entry diffed — both true here.
  1739	                transport
  1740	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  1741	                    .await?;
  1742	                manifest_complete = true;
  1743	            }
  1744	            Some(Frame::FileBegin(header)) => {
  1745	                // Payload records ride the control lane only under the
  1746	                // in-stream carrier; with a TCP data plane active they
  1747	                // flow over the sockets, so one here is a violation.
  1748	                if data_plane_recv.is_some() {
  1749	                    return Err(violation(format!(
  1750	                        "file record '{}' on the control lane while a TCP data plane is active",
  1751	                        header.relative_path
  1752	                    )));
  1753	                }
  1754	                if !manifest_complete {
  1755	                    return Err(violation(format!(
  1756	                        "payload record for '{}' before ManifestComplete",
  1757	                        header.relative_path
  1758	                    )));
  1759	                }
  1760	                if !outstanding
  1761	                    .lock()
  1762	                    .expect("outstanding-needs lock poisoned")
  1763	                    .remove(&header.relative_path)
  1764	                {
  1765	                    return Err(violation(format!(
  1766	                        "payload for '{}' which is not on the need list",
  1767	                        header.relative_path
  1768	                    )));
  1769	                }
  1770	                let outcome = receive_file_record(transport, &sink, &header).await?;
  1771	                files_written += outcome.files_written as u64;
  1772	                bytes_written += outcome.bytes_written;
  1773	            }
  1774	            Some(Frame::TarShardHeader(shard)) => {
  1775	                if data_plane_recv.is_some() {
  1776	                    return Err(violation(
  1777	                        "tar shard record on the control lane while a TCP data plane is active"
  1778	                            .into(),
  1779	                    ));
  1780	                }
  1781	                if !manifest_complete {
  1782	                    return Err(violation("tar shard record before ManifestComplete".into()));
  1783	                }
  1784	                {
  1785	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  1786	                    for h in &shard.files {
  1787	                        if !out.remove(&h.relative_path) {
  1788	                            return Err(violation(format!(
  1789	                                "tar shard entry '{}' which is not on the need list",
  1790	                                h.relative_path
  1791	                            )));
  1792	                        }
  1793	                    }
  1794	                }
  1795	                let outcome = receive_tar_record(transport, &sink, shard).await?;
  1796	                files_written += outcome.files_written as u64;
  1797	                bytes_written += outcome.bytes_written;
  1798	            }
  1799	            Some(Frame::Resize(resize)) => {
  1800	                // sf-2 shape correction (otp-4b-2): the SOURCE proposes
  1801	                // one ADD; arm the credential, grant it (bump `resize_live`),
  1802	                // and ack so the SOURCE dials the epoch-N socket. Only ADD
  1803	                // occurs on the session (REMOVE is a tuner concern, future
  1804	                // work); anything else fails fast.
  1805	                let run = data_plane_recv.as_ref().ok_or_else(|| {
  1806	                    violation("DataPlaneResize on a session with no data plane".into())
  1807	                })?;
  1808	                let op = DataPlaneResizeOp::try_from(resize.op)
  1809	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  1810	                if op != DataPlaneResizeOp::Add {
  1811	                    return Err(violation(format!(
  1812	                        "unsupported data-plane resize op {}",
  1813	                        op.as_str_name()
  1814	                    )));
  1815	                }
  1816	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  1817	                    return Err(violation(
  1818	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  1819	                    ));
  1820	                }
  1821	                // Cumulative ceiling bound (defense in depth — the
  1822	                // source's dial already clamps to the same profile).
  1823	                let accepted = resize_live < resize_ceiling && run.arm(resize.sub_token.clone());
  1824	                if accepted {
  1825	                    resize_live += 1;
  1826	                }
  1827	                let effective = if accepted {
  1828	                    resize.target_stream_count
  1829	                } else {
  1830	                    resize_live as u32
  1831	                };
  1832	                transport
  1833	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  1834	                        epoch: resize.epoch,
  1835	                        effective_stream_count: effective,
  1836	                        accepted,
  1837	                    })))
  1838	                    .await?;
  1839	            }
  1840	            Some(Frame::SourceDone(_)) => {
  1841	                if !manifest_complete {
  1842	                    return Err(violation("SourceDone before ManifestComplete".into()));
  1843	                }
  1844	                // Completion, both carriers: the shared `outstanding`
  1845	                // set must be empty (every granted need claimed exactly
  1846	                // once). In-stream claims inline above; the data-plane
  1847	                // NeedListSink claims as payloads land, so joining the
  1848	                // receive task first drains the last of them (and
  1849	                // surfaces any receive error / stall). Set membership —
  1850	                // not a file count — is the contract (codex F1: a count
  1851	                // proxy let a peer substitute or duplicate paths).
  1852	                // `finish()` drops the arm sender (no more resizes), joins
  1853	                // the accept loop, and reports the settled stream count.
  1854	                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
  1855	                    Some(run) => {
  1856	                        let totals = run.finish().await?;
  1857	                        files_written = totals.outcome.files_written as u64;
  1858	                        bytes_written = totals.outcome.bytes_written;
  1859	                        (false, Some(totals.streams))
  1860	                    }
  1861	                    None => (true, None),
  1862	                };
  1863	                let unfulfilled = outstanding
  1864	                    .lock()
  1865	                    .expect("outstanding-needs lock poisoned")
  1866	                    .len();
  1867	                if unfulfilled != 0 {
  1868	                    return Err(violation(format!(
  1869	                        "SourceDone with {unfulfilled} needed file(s) never delivered"
  1870	                    )));
  1871	                }
  1872	                let summary = TransferSummary {
  1873	                    files_transferred: files_written,
  1874	                    bytes_transferred: bytes_written,
  1875	                    entries_deleted: 0, // mirror lands at otp-6
  1876	                    in_stream_carrier_used,
  1877	                    files_resumed: 0, // resume lands at otp-7
  1878	                };
  1879	                transport.send(frame(Frame::Summary(summary))).await?;
  1880	                return Ok(DestinationOutcome {
  1881	                    summary,
  1882	                    needed_paths,
  1883	                    data_plane_streams,
  1884	                });
  1885	            }
  1886	            Some(Frame::Error(err)) => {
  1887	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  1888	            }
  1889	            other => {
  1890	                // Everything else is off-lane or off-phase here:

exec
/usr/bin/zsh -lc 'git show e6a0b3b''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'840,1095p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   840	    source: Arc<dyn TransferSource>,
   841	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   842	    manifest_sent: &AtomicBool,
   843	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   844	) -> Result<TransferSummary> {
   845	    let mut pending: Vec<FileHeader> = Vec::new();
   846	    let mut need_complete = false;
   847	
   848	    // Data plane (otp-4b): dial the granted TCP sockets up front —
   849	    // BEFORE streaming the manifest — so the destination's accept loop
   850	    // (armed the moment it sent SessionAccept) sees the connections
   851	    // promptly rather than waiting out its bounded-accept timeout while
   852	    // a long manifest streams. The sockets sit idle (keepalive covers
   853	    // that) until payloads are queued below. `None` = the in-stream
   854	    // carrier (fallback), which needs no early setup.
   855	    let mut data_plane = match &negotiated.accept.data_plane {
   856	        Some(grant) => {
   857	            let host = data_plane_host.ok_or_else(|| {
   858	                eyre::Report::new(SessionFault::internal(
   859	                    "responder granted a TCP data plane but this initiator has no host to dial",
   860	                ))
   861	            })?;
   862	            Some(
   863	                data_plane::dial_source_data_plane(
   864	                    host,
   865	                    grant,
   866	                    negotiated.accept.receiver_capacity.as_ref(),
   867	                    Arc::clone(&source),
   868	                )
   869	                .await?,
   870	            )
   871	        }
   872	        None => None,
   873	    };
   874	
   875	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
   876	    // fed to the shape table so the SOURCE grows the data-plane stream
   877	    // count as the workload's shape becomes known. Append-only (a need is
   878	    // counted once, when it arrives), and the in-flight resize record the
   879	    // ack is matched against (at most one — the dial enforces it).
   880	    let mut needed_bytes: u64 = 0;
   881	    let mut needed_count: usize = 0;
   882	    let mut pending_resize: Option<data_plane::PendingResize> = None;
   883	
   884	    // Streaming manifest: entries go out as enumeration produces them
   885	    // (immediate start in every direction — plan §Design 2). The open
   886	    // carries no source path: the source end owns its local endpoint.
   887	    let _ = &negotiated.open;
   888	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   889	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   890	    while let Some(header) = header_rx.recv().await {
   891	        sent.lock()
   892	            .expect("sent-manifest lock poisoned")
   893	            .insert(header.relative_path.clone(), header.clone());
   894	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   895	        // Faults detected by the receive half abort the stream now,
   896	        // not after the full scan; needs just accumulate. (Resize acks
   897	        // cannot arrive yet — none is proposed before the payload phase.)
   898	        drain_ready_source_events(
   899	            &mut events,
   900	            &mut pending,
   901	            &mut need_complete,
   902	            &mut needed_bytes,
   903	            &mut needed_count,
   904	            data_plane.as_ref(),
   905	            tx,
   906	            &mut pending_resize,
   907	        )
   908	        .await?;
   909	    }
   910	    let scanned = scan_handle
   911	        .await
   912	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   913	    let scan_complete = unreadable
   914	        .lock()
   915	        .expect("unreadable list lock poisoned")
   916	        .is_empty();
   917	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   918	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   919	        scan_complete,
   920	    })))
   921	    .await?;
   922	    manifest_sent.store(true, Ordering::Release);
   923	
   924	    // Payload phase. The byte carrier is either the TCP data plane
   925	    // (dialed above) or the in-stream record grammar (fallback). Needs
   926	    // accumulated while a batch was being sent become the next planner
   927	    // batch (contract §Transport selection); payloads only flow after
   928	    // ManifestComplete.
   929	    // The in-stream carrier reuses one read buffer across records; the
   930	    // data plane owns its own pooled buffers, so skip that allocation.
   931	    let mut read_buf = if data_plane.is_none() {
   932	        vec![0u8; IN_STREAM_CHUNK]
   933	    } else {
   934	        Vec::new()
   935	    };
   936	    loop {
   937	        drain_ready_source_events(
   938	            &mut events,
   939	            &mut pending,
   940	            &mut need_complete,
   941	            &mut needed_bytes,
   942	            &mut needed_count,
   943	            data_plane.as_ref(),
   944	            tx,
   945	            &mut pending_resize,
   946	        )
   947	        .await?;
   948	        if !pending.is_empty() {
   949	            let batch = std::mem::take(&mut pending);
   950	            match &mut data_plane {
   951	                Some(dp) => {
   952	                    // sf-2: correct the stream count toward the shape the
   953	                    // accumulated need list implies before queueing this
   954	                    // batch (one ADD per epoch; a no-op while one is in
   955	                    // flight or the shape wants no more).
   956	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
   957	                        .await?;
   958	                    let payloads =
   959	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
   960	                    // A cancel while earlier batches are actively moving
   961	                    // closes the send pipeline under backpressure, so this
   962	                    // queue fails with a data-plane error — prefer the
   963	                    // peer's framed reason (CANCELLED) the same way the
   964	                    // finish() drain does (otp-4b-3 codex F1). Not raced
   965	                    // against events like finish(): live `Need`s still
   966	                    // arrive here, and `recv_peer_fault` would consume them.
   967	                    if let Err(dp_err) = dp.queue(payloads).await {
   968	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
   969	                    }
   970	                }
   971	                None => {
   972	                    send_payload_records(tx, &source, plan_options, batch, &mut read_buf).await?;
   973	                }
   974	            }
   975	            continue;
   976	        }
   977	        if need_complete {
   978	            break;
   979	        }
   980	        match events.recv().await {
   981	            Some(event) => {
   982	                process_source_event(
   983	                    event,
   984	                    &mut pending,
   985	                    &mut need_complete,
   986	                    &mut needed_bytes,
   987	                    &mut needed_count,
   988	                    data_plane.as_ref(),
   989	                    tx,
   990	                    &mut pending_resize,
   991	                )
   992	                .await?;
   993	            }
   994	            None => {
   995	                return Err(eyre::Report::new(SessionFault::internal(
   996	                    "source receive half ended before NeedComplete",
   997	                )))
   998	            }
   999	        }
  1000	    }
  1001	
  1002	    // A resize proposed on the last batch may still be in flight. Resolve
  1003	    // it BEFORE finishing so the destination's armed slot is consumed by
  1004	    // the dialed socket — an armed-but-never-dialed credential would hang
  1005	    // its accept loop (which waits for every arm to be claimed). We do not
  1006	    // propose further here: exactly the one in-flight resize is drained.
  1007	    if let Some(dp) = &data_plane {
  1008	        if let Some(pending) = pending_resize.take() {
  1009	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1010	        }
  1011	    }
  1012	
  1013	    // Close the data plane BEFORE SourceDone so the destination's receive
  1014	    // pipeline sees each socket's END record and completes; SourceDone on
  1015	    // the control lane then lets the destination score and summarize.
  1016	    //
  1017	    // The drain is the byte-transfer phase's wall-time sink, so a
  1018	    // mid-transfer cancel almost always lands here. Race it against a
  1019	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
  1020	    // the served session frames `SessionError{CANCELLED}`, and the source
  1021	    // must surface THAT — not the data-plane transport break it also
  1022	    // causes. Two orderings, both covered:
  1023	    //   * fault arrives while the drain is still pending (e.g. a worker
  1024	    //     blocked reading a slow file, so the socket break never unblocks
  1025	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
  1026	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
  1027	    //     stops the in-flight workers.
  1028	    //   * the socket break makes `finish()` return `Err` first → prefer
  1029	    //     the framed reason if the control lane delivers one within the
  1030	    //     stall window (`prefer_peer_fault`).
  1031	    if let Some(dp) = data_plane.take() {
  1032	        tokio::select! {
  1033	            biased;
  1034	            fault = recv_peer_fault(&mut events) => {
  1035	                return Err(eyre::Report::new(fault));
  1036	            }
  1037	            res = dp.finish() => {
  1038	                if let Err(dp_err) = res {
  1039	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
  1040	                }
  1041	            }
  1042	        }
  1043	    }
  1044	
  1045	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
  1046	
  1047	    // CLOSING: the destination is the scorer; the next event must be
  1048	    // its summary (the receive half ends after forwarding it).
  1049	    match events.recv().await {
  1050	        Some(SourceEvent::Summary(summary)) => Ok(summary),
  1051	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
  1052	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1053	            format!("need for '{}' after NeedComplete", h.relative_path),
  1054	        ))),
  1055	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
  1056	            SessionFault::protocol_violation("duplicate NeedComplete"),
  1057	        )),
  1058	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
  1059	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
  1060	        )),
  1061	        None => Err(eyre::Report::new(SessionFault::internal(
  1062	            "source receive half ended before TransferSummary",
  1063	        ))),
  1064	    }
  1065	}
  1066	
  1067	/// Process every event ready right now (needs accumulating, resize acks
  1068	/// dialing their epoch-N socket) without blocking. Called between
  1069	/// manifest sends and at the top of the payload loop.
  1070	#[allow(clippy::too_many_arguments)]
  1071	async fn drain_ready_source_events(
  1072	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1073	    pending: &mut Vec<FileHeader>,
  1074	    need_complete: &mut bool,
  1075	    needed_bytes: &mut u64,
  1076	    needed_count: &mut usize,
  1077	    data_plane: Option<&data_plane::SourceDataPlane>,
  1078	    tx: &mut Box<dyn FrameTx>,
  1079	    pending_resize: &mut Option<data_plane::PendingResize>,
  1080	) -> Result<()> {
  1081	    while let Ok(event) = events.try_recv() {
  1082	        process_source_event(
  1083	            event,
  1084	            pending,
  1085	            need_complete,
  1086	            needed_bytes,
  1087	            needed_count,
  1088	            data_plane,
  1089	            tx,
  1090	            pending_resize,
  1091	        )
  1092	        .await?;
  1093	    }
  1094	    Ok(())
  1095	}

exec
/usr/bin/zsh -lc 'git show e6a0b3b''^:crates/blit-core/src/transfer_session/data_plane.rs | nl -ba | sed -n '"'520,720p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   520	        )
   521	        .await
   522	    }));
   523	    Ok(SourceDataPlane {
   524	        payload_tx: Some(payload_tx),
   525	        control_tx,
   526	        pipeline: Some(pipeline),
   527	        dial,
   528	        source,
   529	        host: host.to_string(),
   530	        tcp_port: grant.tcp_port,
   531	        session_token: grant.session_token.clone(),
   532	        pool,
   533	    })
   534	}
   535	
   536	impl SourceDataPlane {
   537	    /// The live dial (the byte sender owns it). The driver reads
   538	    /// `live_streams()` for observability and calls `resize_settled` as
   539	    /// each proposal completes.
   540	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   541	        &self.dial
   542	    }
   543	
   544	    /// sf-2 shape correction: propose one ADD toward the stream count the
   545	    /// accumulated need list implies, if none is in flight and the shape
   546	    /// wants more than the current live count. Mints the resize
   547	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   548	    /// the record back on the matching ack.
   549	    pub(super) fn propose_resize(
   550	        &self,
   551	        needed_bytes: u64,
   552	        needed_count: usize,
   553	    ) -> Result<Option<PendingResize>> {
   554	        let desired =
   555	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   556	                as usize;
   557	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   558	            return Ok(None);
   559	        };
   560	        let sub_token = generate_sub_token()
   561	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   562	        Ok(Some(PendingResize {
   563	            epoch: proposal.epoch,
   564	            target_streams: proposal.target_streams as u32,
   565	            sub_token,
   566	        }))
   567	    }
   568	
   569	    /// Dial the epoch-N data socket for an accepted resize and hand it to
   570	    /// the running pipeline (`SinkControl::Add`). A dial failure is FATAL
   571	    /// (fail-fast): a same-build peer whose listener already accepted
   572	    /// epoch-0 failing an epoch-N dial is a transport fault worth
   573	    /// surfacing — and faulting the session aborts the peer's accept loop
   574	    /// via AbortOnDrop, so its armed slot never orphans. (Old push
   575	    /// recovers non-fatally via an arm TTL; the session trades that for
   576	    /// simplicity — noted in the finding doc.) If the pipeline is already
   577	    /// gone (transfer completing under the ADD), the just-dialed socket
   578	    /// is closed cleanly so the peer's worker sees its END, not a reset.
   579	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   580	        let mut handshake = self.session_token.clone();
   581	        handshake.extend_from_slice(sub_token);
   582	        let session = DataPlaneSession::connect(
   583	            &self.host,
   584	            self.tcp_port,
   585	            &handshake,
   586	            self.dial.chunk_bytes(),
   587	            self.dial.prefetch_count(),
   588	            false,
   589	            self.dial.tcp_buffer_bytes(),
   590	            Arc::clone(&self.pool),
   591	        )
   592	        .await
   593	        .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?;
   594	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   595	            session,
   596	            Arc::clone(&self.source),
   597	            PathBuf::new(),
   598	        ));
   599	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   600	            if let SinkControl::Add(sink) = returned.0 {
   601	                let _ = sink.finish().await;
   602	            }
   603	        }
   604	        Ok(())
   605	    }
   606	
   607	    /// Feed one planned batch into the send pipeline. The pipeline
   608	    /// prepares each payload (tar-shard/file) and writes it through the
   609	    /// data-plane record framing across the live socket(s).
   610	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   611	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   612	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   613	        })?;
   614	        for payload in payloads {
   615	            tx.send(payload).await.map_err(|_| {
   616	                dp_fault("data-plane send pipeline closed before all payloads sent")
   617	            })?;
   618	        }
   619	        Ok(())
   620	    }
   621	
   622	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   623	    /// socket's END record on drain), and return the bytes sent. Must be
   624	    /// awaited before `SourceDone` goes out so the destination's receive
   625	    /// pipeline sees END and completes.
   626	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   627	        // Drop the sender: workers observe the closed queue, drain what
   628	        // is left, then `finish()` (END record) and exit.
   629	        self.payload_tx = None;
   630	        let pipeline = self
   631	            .pipeline
   632	            .take()
   633	            .expect("SourceDataPlane::finish called once");
   634	        pipeline
   635	            .join()
   636	            .await
   637	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   638	    }
   639	}
   640	
   641	// ---------------------------------------------------------------------------
   642	// Need-list enforcement for the data-plane receive
   643	// ---------------------------------------------------------------------------
   644	
   645	/// Sink decorator that enforces the session's need-list contract on the
   646	/// data-plane receive, giving it the SAME strictness the in-stream
   647	/// carrier applies inline in the control loop (`outstanding.remove`).
   648	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   649	/// without this a peer could substitute an off-need-list path for a
   650	/// needed one (count-preserving), duplicate one, or send resume block
   651	/// records the non-resume session never negotiated (codex otp-4b-1 F1).
   652	/// Every written path must be a granted, not-yet-received need; resume
   653	/// block records are rejected outright. The shared [`OutstandingNeeds`]
   654	/// set makes completion `is_empty()` for both carriers.
   655	pub(super) struct NeedListSink {
   656	    inner: Arc<dyn TransferSink>,
   657	    outstanding: OutstandingNeeds,
   658	}
   659	
   660	impl NeedListSink {
   661	    pub(super) fn new(inner: Arc<dyn TransferSink>, outstanding: OutstandingNeeds) -> Self {
   662	        Self { inner, outstanding }
   663	    }
   664	
   665	    /// Remove `path` from the outstanding set, or fault: a path that is
   666	    /// not present is either off the need list or a duplicate delivery.
   667	    fn claim(&self, path: &str) -> Result<()> {
   668	        if self
   669	            .outstanding
   670	            .lock()
   671	            .expect("outstanding-needs lock poisoned")
   672	            .remove(path)
   673	        {
   674	            Ok(())
   675	        } else {
   676	            Err(eyre::Report::new(SessionFault::protocol_violation(
   677	                format!(
   678	                    "data-plane payload for '{path}' which is not an outstanding need \
   679	                 (off the need list, or a duplicate delivery)"
   680	                ),
   681	            )))
   682	        }
   683	    }
   684	}
   685	
   686	#[async_trait]
   687	impl TransferSink for NeedListSink {
   688	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   689	        match &payload {
   690	            PreparedPayload::File(header) => self.claim(&header.relative_path)?,
   691	            PreparedPayload::TarShard { headers, .. } => {
   692	                for header in headers {
   693	                    self.claim(&header.relative_path)?;
   694	                }
   695	            }
   696	            // The session did not negotiate resume (otp-7), so a block
   697	            // record on the data plane is a protocol violation, not a
   698	            // silently-applied patch.
   699	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   700	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   701	                    "resume block record on the data plane of a non-resume session",
   702	                )));
   703	            }
   704	        }
   705	        self.inner.write_payload(payload).await
   706	    }
   707	
   708	    async fn write_file_stream(
   709	        &self,
   710	        header: &FileHeader,
   711	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   712	    ) -> Result<SinkOutcome> {
   713	        self.claim(&header.relative_path)?;
   714	        self.inner.write_file_stream(header, reader).await
   715	    }
   716	
   717	    async fn finish(&self) -> Result<()> {
   718	        self.inner.finish().await
   719	    }
   720	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/tests/transfer_session_roles.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Role-parameterized session suite (otp-3).
     2	//!
     3	//! Every fixture runs under BOTH role assignments — the initiator as
     4	//! SOURCE (push-shaped) and the initiator as DESTINATION (pull-shaped)
     5	//! — over the in-process transport, and the outcomes must be
     6	//! IDENTICAL: same need-list set, same summary counts, same bytes on
     7	//! disk. This is the owner's invariance requirement
     8	//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
     9	//! executable form: there is no per-direction code to diverge, and
    10	//! this suite pins that the one code path really is
    11	//! initiator-indifferent.
    12	
    13	use std::collections::BTreeMap;
    14	use std::path::Path;
    15	use std::sync::Arc;
    16	use std::time::Duration;
    17	
    18	use blit_core::generated::transfer_frame::Frame;
    19	use blit_core::generated::{
    20	    session_error, ComparisonMode, FileHeader, ManifestComplete, NeedBatch, NeedComplete,
    21	    NeedEntry, SessionHello, SessionOpen, TransferFrame, TransferRole, TransferSummary,
    22	};
    23	use blit_core::remote::transfer::source::FsTransferSource;
    24	use blit_core::transfer_plan::PlanOptions;
    25	use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
    26	use blit_core::transfer_session::{
    27	    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    28	    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
    29	};
    30	
    31	const SUITE_TIMEOUT: Duration = Duration::from_secs(120);
    32	
    33	/// (relative path, content, mtime seconds). Fixture mtimes are fixed
    34	/// epochs so both role-assignment runs see byte-for-byte identical
    35	/// trees.
    36	type FileSpec = (&'static str, Vec<u8>, i64);
    37	
    38	fn write_tree(root: &Path, files: &[FileSpec]) {
    39	    for (rel, content, mtime) in files {
    40	        let path = root.join(rel);
    41	        if let Some(parent) = path.parent() {
    42	            std::fs::create_dir_all(parent).unwrap();
    43	        }
    44	        std::fs::write(&path, content).unwrap();
    45	        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
    46	    }
    47	}
    48	
    49	/// Every regular file under `root` as rel-path → bytes.
    50	fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    51	    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    52	        for entry in std::fs::read_dir(dir).unwrap() {
    53	            let entry = entry.unwrap();
    54	            let path = entry.path();
    55	            if path.is_dir() {
    56	                walk(root, &path, out);
    57	            } else {
    58	                let rel = path
    59	                    .strip_prefix(root)
    60	                    .unwrap()
    61	                    .to_string_lossy()
    62	                    .replace('\\', "/");
    63	                out.insert(rel, std::fs::read(&path).unwrap());
    64	            }
    65	        }
    66	    }
    67	    let mut out = BTreeMap::new();
    68	    if root.exists() {
    69	        walk(root, root, &mut out);
    70	    }
    71	    out
    72	}
    73	
    74	fn assert_trees_identical(src: &Path, dst: &Path) {
    75	    let src_tree = collect_tree(src);
    76	    let dst_tree = collect_tree(dst);
    77	    assert_eq!(
    78	        src_tree.keys().collect::<Vec<_>>(),
    79	        dst_tree.keys().collect::<Vec<_>>(),
    80	        "path sets differ between {src:?} and {dst:?}"
    81	    );
    82	    for (rel, bytes) in &src_tree {
    83	        assert_eq!(
    84	            bytes, &dst_tree[rel],
    85	            "content differs for '{rel}' between {src:?} and {dst:?}"
    86	        );
    87	    }
    88	}
    89	
    90	fn basic_open(initiator_role: TransferRole) -> SessionOpen {
    91	    SessionOpen {
    92	        initiator_role: initiator_role as i32,
    93	        compare_mode: ComparisonMode::SizeMtime as i32,
    94	        in_stream_bytes: true,
    95	        ..Default::default()
    96	    }
    97	}
    98	
    99	/// Drive one full session between `src_root` and `dst_root` with the
   100	/// given end acting as initiator. Data direction is FIXED
   101	/// (src_root → dst_root); the parameter only swaps which end opens
   102	/// the session — the thing the owner's invariant says must not
   103	/// matter.
   104	async fn run_session(
   105	    initiator_role: TransferRole,
   106	    src_root: &Path,
   107	    dst_root: &Path,
   108	    plan_options: PlanOptions,
   109	) -> (
   110	    eyre::Result<TransferSummary>,
   111	    eyre::Result<DestinationOutcome>,
   112	) {
   113	    let open = basic_open(initiator_role);
   114	    let (source_endpoint, dest_endpoint) = match initiator_role {
   115	        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
   116	        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
   117	        TransferRole::Unspecified => panic!("fixture must pick a role"),
   118	    };
   119	    let source_cfg = SourceSessionConfig {
   120	        hello: HelloConfig::default(),
   121	        endpoint: source_endpoint,
   122	        plan_options,
   123	        data_plane_host: None,
   124	    };
   125	    let dest_cfg = DestinationSessionConfig {
   126	        hello: HelloConfig::default(),
   127	        endpoint: dest_endpoint,
   128	        data_plane_host: None,
   129	    };
   130	    let (a, b) = in_process_pair();
   131	    let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
   132	    tokio::time::timeout(SUITE_TIMEOUT, async {
   133	        tokio::join!(
   134	            run_source(source_cfg, a, source),
   135	            run_destination(
   136	                dest_cfg,
   137	                b,
   138	                DestinationTarget::Fixed(dst_root.to_path_buf())
   139	            ),
   140	        )
   141	    })
   142	    .await
   143	    .expect("session run timed out")
   144	}
   145	
   146	/// Run the same fixture under both role assignments (fresh trees per
   147	/// run) and pin the invariance property: identical need sets,
   148	/// identical summaries, byte-identical destinations.
   149	async fn assert_invariant_across_roles(
   150	    src_files: &[FileSpec],
   151	    dst_files: &[FileSpec],
   152	    plan_options: PlanOptions,
   153	) -> (TransferSummary, Vec<String>) {
   154	    let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
   155	    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
   156	        let tmp = tempfile::tempdir().unwrap();
   157	        let src_root = tmp.path().join("src");
   158	        let dst_root = tmp.path().join("dst");
   159	        std::fs::create_dir_all(&src_root).unwrap();
   160	        std::fs::create_dir_all(&dst_root).unwrap();
   161	        write_tree(&src_root, src_files);
   162	        write_tree(&dst_root, dst_files);
   163	
   164	        let (source_result, dest_result) =
   165	            run_session(initiator_role, &src_root, &dst_root, plan_options).await;
   166	        let source_summary = source_result
   167	            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
   168	        let dest_outcome = dest_result.unwrap_or_else(|e| {
   169	            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
   170	        });
   171	
   172	        assert_eq!(
   173	            source_summary, dest_outcome.summary,
   174	            "both ends must hold the same summary (initiator {initiator_role:?})"
   175	        );
   176	        assert!(
   177	            source_summary.in_stream_carrier_used,
   178	            "otp-3 sessions ride the in-stream carrier"
   179	        );
   180	        assert_trees_identical(&src_root, &dst_root);
   181	
   182	        let mut needed = dest_outcome.needed_paths.clone();
   183	        needed.sort();
   184	        per_role.push((dest_outcome.summary, needed));
   185	    }
   186	
   187	    let (summary_a, needed_a) = per_role.remove(0);
   188	    let (summary_b, needed_b) = per_role.remove(0);
   189	    assert_eq!(
   190	        needed_a, needed_b,
   191	        "need-list set must be identical whichever end initiates"
   192	    );
   193	    assert_eq!(
   194	        summary_a, summary_b,
   195	        "summary must be identical whichever end initiates"
   196	    );
   197	    (summary_a, needed_a)
   198	}
   199	
   200	fn fault_of(err: &eyre::Report) -> &SessionFault {
   201	    err.downcast_ref::<SessionFault>()
   202	        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
   203	}
   204	
   205	// ---------------------------------------------------------------------------
   206	// Fixtures
   207	// ---------------------------------------------------------------------------
   208	
   209	/// Mixed small tree: nested dirs, an empty file, a name with spaces,
   210	/// and a file larger than the in-stream chunk so file records span
   211	/// multiple FileData frames.
   212	fn small_tree() -> Vec<FileSpec> {
   213	    vec![
   214	        ("a.txt", b"alpha".to_vec(), 1_600_000_001),
   215	        ("empty.bin", Vec::new(), 1_600_000_002),
   216	        ("dir one/b.log", vec![0xAB; 4096], 1_600_000_003),
   217	        (
   218	            "dir one/deeper/c.dat",
   219	            b"gamma-content".to_vec(),
   220	            1_600_000_004,

exec
/usr/bin/zsh -lc "rg -n \"DestinationSessionConfig|run_destination\\(|data_plane_host|SessionEndpoint::initiator|initiator_role: TransferRole::Destination|initiator_role: TransferRole::Source\" crates/blit-core/tests crates/blit-core/src crates/blit-daemon/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/transfer_session_roles.rs:27:    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
crates/blit-core/tests/transfer_session_roles.rs:115:        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
crates/blit-core/tests/transfer_session_roles.rs:116:        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
crates/blit-core/tests/transfer_session_roles.rs:123:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:125:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:128:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:135:            run_destination(
crates/blit-core/tests/transfer_session_roles.rs:344:        initiator_role: TransferRole::Source as i32,
crates/blit-core/tests/transfer_session_roles.rs:351:        endpoint: SessionEndpoint::initiator(open),
crates/blit-core/tests/transfer_session_roles.rs:353:        data_plane_host: Some("127.0.0.1".into()),
crates/blit-core/tests/transfer_session_roles.rs:355:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:358:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:365:            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
crates/blit-core/tests/transfer_session_roles.rs:418:        initiator_role: TransferRole::Destination as i32,
crates/blit-core/tests/transfer_session_roles.rs:427:        data_plane_host: None, // a responder never dials
crates/blit-core/tests/transfer_session_roles.rs:429:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:431:        endpoint: SessionEndpoint::initiator(open), // dials + receives
crates/blit-core/tests/transfer_session_roles.rs:432:        data_plane_host: Some("127.0.0.1".into()),
crates/blit-core/tests/transfer_session_roles.rs:439:            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
crates/blit-core/tests/transfer_session_roles.rs:509:            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
crates/blit-core/tests/transfer_session_roles.rs:510:            _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
crates/blit-core/tests/transfer_session_roles.rs:519:            data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:521:        let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:527:            data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:534:                run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
crates/blit-core/tests/transfer_session_roles.rs:573:        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs:575:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:577:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:583:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:589:        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
crates/blit-core/tests/transfer_session_roles.rs:617:        endpoint: SessionEndpoint::initiator(open),
crates/blit-core/tests/transfer_session_roles.rs:619:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:621:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:624:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:630:        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
crates/blit-core/tests/transfer_session_roles.rs:673:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:676:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:679:    let dest = tokio::spawn(run_destination(
crates/blit-core/tests/transfer_session_roles.rs:741:        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs:743:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:795:        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs:797:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:854:        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs:856:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:906:    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs:909:        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs:912:    let dest = tokio::spawn(run_destination(
crates/blit-core/src/transfer_session/mod.rs:127:    pub data_plane_host: Option<String>,
crates/blit-core/src/transfer_session/mod.rs:130:pub struct DestinationSessionConfig {
crates/blit-core/src/transfer_session/mod.rs:139:    /// with [`SourceSessionConfig::data_plane_host`].
crates/blit-core/src/transfer_session/mod.rs:140:    pub data_plane_host: Option<String>,
crates/blit-core/src/transfer_session/mod.rs:687:        cfg.data_plane_host,
crates/blit-core/src/transfer_session/mod.rs:702:    data_plane_host: Option<String>,
crates/blit-core/src/transfer_session/mod.rs:731:        data_plane_host.as_deref(),
crates/blit-core/src/transfer_session/mod.rs:852:    data_plane_host: Option<&str>,
crates/blit-core/src/transfer_session/mod.rs:890:                let host = data_plane_host.ok_or_else(|| {
crates/blit-core/src/transfer_session/mod.rs:1449:pub async fn run_destination(
crates/blit-core/src/transfer_session/mod.rs:1450:    cfg: DestinationSessionConfig,
crates/blit-core/src/transfer_session/mod.rs:1512:        cfg.data_plane_host.as_deref(),
crates/blit-core/src/transfer_session/mod.rs:1525:    data_plane_host: Option<&str>,
crates/blit-core/src/transfer_session/mod.rs:1527:    match destination_session(transport, negotiated, dst_root, data_plane_host).await {
crates/blit-core/src/transfer_session/mod.rs:1656:    data_plane_host: Option<&str>,
crates/blit-core/src/transfer_session/mod.rs:1705:    // (pull) dials the grant it received on `data_plane_host`. Byte
crates/blit-core/src/transfer_session/mod.rs:1735:        None => match (&negotiated.accept.data_plane, data_plane_host) {
crates/blit-core/src/remote/transfer/session_client.rs:33:    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
crates/blit-core/src/remote/transfer/session_client.rs:80:        initiator_role: TransferRole::Source as i32,
crates/blit-core/src/remote/transfer/session_client.rs:106:        endpoint: SessionEndpoint::initiator(open),
crates/blit-core/src/remote/transfer/session_client.rs:110:        data_plane_host: Some(endpoint.host.clone()),
crates/blit-core/src/remote/transfer/session_client.rs:165:        initiator_role: TransferRole::Destination as i32,
crates/blit-core/src/remote/transfer/session_client.rs:186:    let cfg = DestinationSessionConfig {
crates/blit-core/src/remote/transfer/session_client.rs:188:        endpoint: SessionEndpoint::initiator(open),
crates/blit-core/src/remote/transfer/session_client.rs:191:        data_plane_host: Some(endpoint.host.clone()),
crates/blit-core/src/remote/transfer/session_client.rs:193:    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await

exec
/usr/bin/zsh -lc "rg -n \"DATA_PLANE_ACCEPT_TIMEOUT|DATA_PLANE_TOKEN_TIMEOUT|dial_data_plane|connect_timeout|write_all\" crates/blit-core/src/remote/transfer -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/session_client.rs:219:        .connect_timeout(Duration::from_secs(30));
crates/blit-core/src/remote/transfer/mod.rs:41:pub use socket::{configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT};
crates/blit-core/src/remote/transfer/stall_guard.rs:31://!   already bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` /
crates/blit-core/src/remote/transfer/stall_guard.rs:32://!   `DATA_PLANE_TOKEN_TIMEOUT` pair (`remote::transfer::socket`);
crates/blit-core/src/remote/transfer/stall_guard.rs:66:///   the shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
crates/blit-core/src/remote/transfer/stall_guard.rs:131:/// kernel send buffer and `write_all` blocks indefinitely (until OS-level
crates/blit-core/src/remote/transfer/stall_guard.rs:174:                // caller (write_all loop) will keep polling; if real
crates/blit-core/src/remote/transfer/stall_guard.rs:239:            tx.write_all(b"hello").await.unwrap();
crates/blit-core/src/remote/transfer/stall_guard.rs:260:                tx.write_all(b"x").await.unwrap();
crates/blit-core/src/remote/transfer/stall_guard.rs:292:        let _ = guarded.write_all(&[0u8; 64]).await;
crates/blit-core/src/remote/transfer/stall_guard.rs:297:            .write_all(&[0u8; 16])
crates/blit-core/src/remote/transfer/stall_guard.rs:316:            .write_all(b"hello world")
crates/blit-core/src/remote/transfer/stall_guard.rs:342:                .write_all(b"x")
crates/blit-core/src/remote/transfer/socket.rs:13://! design-3 added [`dial_data_plane`]: the client-side dial (bounded
crates/blit-core/src/remote/transfer/socket.rs:27:/// and [`DATA_PLANE_TOKEN_TIMEOUT`] — replacing three per-file
crates/blit-core/src/remote/transfer/socket.rs:35:pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/socket.rs:41:pub const DATA_PLANE_TOKEN_TIMEOUT: Duration = Duration::from_secs(15);
crates/blit-core/src/remote/transfer/socket.rs:109:/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
crates/blit-core/src/remote/transfer/socket.rs:116:/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
crates/blit-core/src/remote/transfer/socket.rs:123:pub async fn dial_data_plane(
crates/blit-core/src/remote/transfer/socket.rs:128:    dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:132:        DATA_PLANE_ACCEPT_TIMEOUT,
crates/blit-core/src/remote/transfer/socket.rs:133:        DATA_PLANE_TOKEN_TIMEOUT,
crates/blit-core/src/remote/transfer/socket.rs:138:/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
crates/blit-core/src/remote/transfer/socket.rs:140:async fn dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:144:    connect_timeout: Duration,
crates/blit-core/src/remote/transfer/socket.rs:147:    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
crates/blit-core/src/remote/transfer/socket.rs:152:                format!("connect did not complete within {connect_timeout:?}"),
crates/blit-core/src/remote/transfer/socket.rs:155:                "data-plane connect to {addr} timed out after {connect_timeout:?} — the \
crates/blit-core/src/remote/transfer/socket.rs:163:    match tokio::time::timeout(token_timeout, stream.write_all(handshake)).await {
crates/blit-core/src/remote/transfer/socket.rs:273:            dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:308:        // write_all must block on a peer that never reads. The accepted
crates/blit-core/src/remote/transfer/socket.rs:313:            dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:347:        let result = dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/pipeline.rs:1016:                let _ = writer.write_all(&bytes).await;
crates/blit-core/src/remote/transfer/data_plane.rs:55:/// (15+ minutes). All existing `self.stream.write_all/.flush` call
crates/blit-core/src/remote/transfer/data_plane.rs:149:        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
crates/blit-core/src/remote/transfer/data_plane.rs:238:            .write_all(&[DATA_PLANE_RECORD_END])
crates/blit-core/src/remote/transfer/data_plane.rs:284:            .write_all(&[DATA_PLANE_RECORD_FILE])
crates/blit-core/src/remote/transfer/data_plane.rs:288:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:292:            .write_all(path_bytes)
crates/blit-core/src/remote/transfer/data_plane.rs:297:            .write_all(&header.size.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:305:            .write_all(&header.mtime_seconds.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:309:            .write_all(&header.permissions.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:371:            // join first polls it and stops when write_all completes —
crates/blit-core/src/remote/transfer/data_plane.rs:384:                    let result = stream.write_all(write_slice).await;
crates/blit-core/src/remote/transfer/data_plane.rs:428:                .write_all(&buf_a.as_slice()[..bytes_a])
crates/blit-core/src/remote/transfer/data_plane.rs:462:            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
crates/blit-core/src/remote/transfer/data_plane.rs:466:            .write_all(&(headers.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:479:                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:483:                .write_all(rel_bytes)
crates/blit-core/src/remote/transfer/data_plane.rs:487:                .write_all(&header.size.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:491:                .write_all(&header.mtime_seconds.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:495:                .write_all(&header.permissions.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:501:            .write_all(&(data.len() as u64).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:515:                .write_all(chunk)
crates/blit-core/src/remote/transfer/data_plane.rs:556:            .write_all(&[DATA_PLANE_RECORD_BLOCK])
crates/blit-core/src/remote/transfer/data_plane.rs:560:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:564:            .write_all(path_bytes)
crates/blit-core/src/remote/transfer/data_plane.rs:568:            .write_all(&offset.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:572:            .write_all(&(content.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:576:            .write_all(content)
crates/blit-core/src/remote/transfer/data_plane.rs:612:            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
crates/blit-core/src/remote/transfer/data_plane.rs:616:            .write_all(&(path_bytes.len() as u32).to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:620:            .write_all(path_bytes)
crates/blit-core/src/remote/transfer/data_plane.rs:624:            .write_all(&total_size.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:628:            .write_all(&mtime_seconds.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:632:            .write_all(&permissions.to_be_bytes())
crates/blit-core/src/remote/transfer/data_plane.rs:705:            dst.write_all(&buf_a[..bytes_a]),
crates/blit-core/src/remote/transfer/data_plane.rs:710:        // AFTER `write_all` succeeds so a `bytes_completed`
crates/blit-core/src/remote/transfer/data_plane.rs:731:        dst.write_all(&buf_a[..bytes_a])
crates/blit-core/src/remote/transfer/sink.rs:673:    file.write_all(&bytes)

codex
The new receive side uses `JoinSet` directly; dropped session state should abort those workers just like the responder’s `AbortOnDrop` wrapper aborts its accept loop. I’m checking the lower-level data-plane session and socket helpers now, because the review hinges on whether both new sockets inherit the existing timeouts and stall guard semantics.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     4	use tokio::net::TcpStream;
     5	
     6	use crate::buffer::BufferPool;
     7	use crate::generated::FileHeader;
     8	
     9	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    10	use super::progress::{NoProbe, Probe};
    11	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    12	use crate::remote::transfer::source::TransferSource;
    13	use std::sync::Arc;
    14	
    15	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    16	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    17	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    18	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    19	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    20	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    21	
    22	/// ue-r2-2: length of the per-epoch resize credential a data socket
    23	/// echoes after the one-time token when resize was negotiated
    24	/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
    25	/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
    26	pub const SUB_TOKEN_LEN: usize = 16;
    27	
    28	/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
    29	/// as the daemon's one-time token (audit-3b): a missing system RNG is
    30	/// an error, never a weaker credential.
    31	pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
    32	    use rand::{rngs::SysRng, TryRng};
    33	    let mut buf = vec![0u8; SUB_TOKEN_LEN];
    34	    SysRng
    35	        .try_fill_bytes(&mut buf)
    36	        .map_err(|err| eyre::eyre!("system RNG unavailable: {err}"))?;
    37	    Ok(buf)
    38	}
    39	
    40	/// A single data-plane TCP stream and its send loop.
    41	///
    42	/// Generic over a [`Probe`] so the byte-copy hot path can carry
    43	/// per-stream telemetry under adaptive mode at **zero cost** when the
    44	/// probe is [`NoProbe`] (the default): the instrumented branches are
    45	/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
    46	/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
    47	/// bare type and get the `NoProbe` default; the adaptive controller
    48	/// constructs `DataPlaneSession<LiveProbe>` via
    49	/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
    50	///
    51	/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
    52	/// reader (TCP backpressure from a slow / wedged peer) trips after
    53	/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
    54	/// of pinning the worker for OS-level TCP retransmit exhaustion
    55	/// (15+ minutes). All existing `self.stream.write_all/.flush` call
    56	/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
    57	/// so no per-site change was needed.
    58	pub struct DataPlaneSession<P: Probe = NoProbe> {
    59	    stream: StallGuardWriter<TcpStream>,
    60	    pool: Arc<BufferPool>,
    61	    trace: bool,
    62	    chunk_bytes: usize,
    63	    payload_prefetch: usize,
    64	    bytes_sent: u64,
    65	    probe: P,
    66	}
    67	
    68	macro_rules! trace_client {
    69	    ($session:expr, $($arg:tt)*) => {
    70	        if $session.trace {
    71	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    72	        }
    73	    };
    74	}
    75	
    76	impl DataPlaneSession<NoProbe> {
    77	    /// Create a session from an existing stream with buffer pooling.
    78	    ///
    79	    /// Produces the un-instrumented `NoProbe` variant — the default for
    80	    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    81	    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    82	    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    83	    /// observable write progress instead of pinning the worker for
    84	    /// OS-level TCP retransmit exhaustion. The production call sites
    85	    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    86	    /// resume path) inherit the guard without code changes.
    87	    pub async fn from_stream(
    88	        stream: TcpStream,
    89	        trace: bool,
    90	        chunk_bytes: usize,
    91	        payload_prefetch: usize,
    92	        pool: Arc<BufferPool>,
    93	    ) -> Self {
    94	        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
    95	            .await
    96	    }
    97	
    98	    /// Connect to a data plane endpoint with buffer pooling.
    99	    #[allow(clippy::too_many_arguments)]
   100	    pub async fn connect(
   101	        host: &str,
   102	        port: u32,
   103	        token: &[u8],
   104	        chunk_bytes: usize,
   105	        payload_prefetch: usize,
   106	        trace: bool,
   107	        tcp_buffer_size: Option<usize>,
   108	        pool: Arc<BufferPool>,
   109	    ) -> Result<Self> {
   110	        Self::connect_with_probe(
   111	            host,
   112	            port,
   113	            token,
   114	            chunk_bytes,
   115	            payload_prefetch,
   116	            trace,
   117	            tcp_buffer_size,
   118	            pool,
   119	            NoProbe,
   120	        )
   121	        .await
   122	    }
   123	}
   124	
   125	impl<P: Probe> DataPlaneSession<P> {
   126	    /// `connect` with an explicit probe (ue-r2-1e: the dial tuner
   127	    /// attaches `LiveProbe` telemetry to the push data plane; the
   128	    /// probe-free path monomorphizes to `NoProbe` and reads no clock).
   129	    #[allow(clippy::too_many_arguments)]
   130	    pub async fn connect_with_probe(
   131	        host: &str,
   132	        port: u32,
   133	        token: &[u8],
   134	        chunk_bytes: usize,
   135	        payload_prefetch: usize,
   136	        trace: bool,
   137	        tcp_buffer_size: Option<usize>,
   138	        pool: Arc<BufferPool>,
   139	        probe: P,
   140	    ) -> Result<Self> {
   141	        let addr = format!("{}:{}", host, port);
   142	        if trace {
   143	            eprintln!("[data-plane-client] connecting to {}", addr);
   144	        }
   145	        // design-3: bounded dial (connect + w1-2 socket policy +
   146	        // negotiation-token write) via the shared data-plane helper —
   147	        // one owner for every client-side data-plane dial, both
   148	        // directions.
   149	        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
   150	            .await
   151	            .context("dialing push data plane")?;
   152	
   153	        Ok(
   154	            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
   155	                .await,
   156	        )
   157	    }
   158	}
   159	
   160	impl<P: Probe> DataPlaneSession<P> {
   161	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   162	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   163	    /// the adaptive controller calls this with a `LiveProbe` to enable
   164	    /// per-stream telemetry.
   165	    pub async fn from_stream_with_probe(
   166	        stream: TcpStream,
   167	        trace: bool,
   168	        chunk_bytes: usize,
   169	        payload_prefetch: usize,
   170	        pool: Arc<BufferPool>,
   171	        probe: P,
   172	    ) -> Self {
   173	        let payload_prefetch = payload_prefetch.max(1);
   174	        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   175	        Self {
   176	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   177	            pool,
   178	            trace,
   179	            chunk_bytes,
   180	            payload_prefetch,
   181	            bytes_sent: 0,
   182	            probe,
   183	        }
   184	    }
   185	
   186	    pub async fn send_payloads(
   187	        &mut self,
   188	        source: Arc<dyn TransferSource>,
   189	        payloads: Vec<TransferPayload>,
   190	    ) -> Result<()> {
   191	        self.send_payloads_with_progress(source, payloads, None)
   192	            .await
   193	    }
   194	
   195	    pub async fn send_payloads_with_progress(
   196	        &mut self,
   197	        source: Arc<dyn TransferSource>,
   198	        payloads: Vec<TransferPayload>,
   199	        progress: Option<&super::progress::RemoteTransferProgress>,
   200	    ) -> Result<()> {
   201	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   202	        while let Some(prepared) = stream.next().await {
   203	            match prepared? {
   204	                PreparedPayload::File(header) => {
   205	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   206	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   207	                    }
   208	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   209	                    if let Some(progress) = progress {
   210	                        progress.report_payload(0, header.size);
   211	                        progress.report_file_complete(header.relative_path.clone());
   212	                    }
   213	                }
   214	                PreparedPayload::TarShard { headers, data } => {
   215	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   216	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   217	                        return Err(err.wrap_err("sending tar shard"));
   218	                    }
   219	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   220	                    if let Some(progress) = progress {
   221	                        for header in &headers {
   222	                            progress.report_payload(0, header.size);
   223	                            progress.report_file_complete(header.relative_path.clone());
   224	                        }
   225	                    }
   226	                }
   227	                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   228	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   229	                }
   230	            }
   231	        }
   232	
   233	        Ok(())
   234	    }
   235	
   236	    pub async fn finish(&mut self) -> Result<()> {
   237	        self.stream
   238	            .write_all(&[DATA_PLANE_RECORD_END])
   239	            .await
   240	            .context("writing transfer terminator")?;
   241	        self.stream
   242	            .flush()
   243	            .await
   244	            .context("flushing data plane stream")
   245	    }
   246	
   247	    pub fn bytes_sent(&self) -> u64 {
   248	        self.bytes_sent
   249	    }
   250	
   251	    pub async fn send_file(
   252	        &mut self,
   253	        source: Arc<dyn TransferSource>,
   254	        header: &FileHeader,
   255	    ) -> Result<()> {
   256	        let rel = &header.relative_path;
   257	        let mut file = source
   258	            .open_file(header)
   259	            .await
   260	            .with_context(|| format!("opening {}", rel))?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '260,460p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   260	            .with_context(|| format!("opening {}", rel))?;
   261	        self.send_file_from_reader(header, &mut file).await
   262	    }
   263	
   264	    /// Send a file payload whose bytes come from an arbitrary async
   265	    /// reader (not a local file). Used by `DataPlaneSink` for the
   266	    /// remote→remote relay case, where bytes arrive from an inbound
   267	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   268	    ///
   269	    /// Same wire format and double-buffered loop as `send_file`.
   270	    pub async fn send_file_from_reader(
   271	        &mut self,
   272	        header: &FileHeader,
   273	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   274	    ) -> Result<()> {
   275	        let rel = &header.relative_path;
   276	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   277	
   278	        let path_bytes = rel.as_bytes();
   279	        if path_bytes.len() > u32::MAX as usize {
   280	            bail!("relative path too long for transfer: {}", rel);
   281	        }
   282	
   283	        self.stream
   284	            .write_all(&[DATA_PLANE_RECORD_FILE])
   285	            .await
   286	            .context("writing data-plane record tag")?;
   287	        self.stream
   288	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   289	            .await
   290	            .context("writing path length")?;
   291	        self.stream
   292	            .write_all(path_bytes)
   293	            .await
   294	            .context("writing path bytes")?;
   295	
   296	        self.stream
   297	            .write_all(&header.size.to_be_bytes())
   298	            .await
   299	            .context("writing file size")?;
   300	        // Wire-format extension (2026-05-01): include mtime + permissions
   301	        // inline so push and pull data plane records carry the same
   302	        // information. Lets the receive pipeline apply metadata via
   303	        // FsTransferSink without consulting an out-of-band manifest cache.
   304	        self.stream
   305	            .write_all(&header.mtime_seconds.to_be_bytes())
   306	            .await
   307	            .context("writing mtime")?;
   308	        self.stream
   309	            .write_all(&header.permissions.to_be_bytes())
   310	            .await
   311	            .context("writing permissions")?;
   312	
   313	        // Double-buffered I/O: overlaps source reads with network writes
   314	        self.send_file_double_buffered(reader, header, rel).await?;
   315	
   316	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   317	
   318	        Ok(())
   319	    }
   320	
   321	    /// Double-buffered file sending: overlaps disk reads with network writes.
   322	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   323	    ///
   324	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   325	    /// This hides disk latency behind network latency for improved throughput.
   326	    async fn send_file_double_buffered(
   327	        &mut self,
   328	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   329	        header: &FileHeader,
   330	        rel: &str,
   331	    ) -> Result<()> {
   332	        let mut remaining = header.size;
   333	        if remaining == 0 {
   334	            return Ok(());
   335	        }
   336	
   337	        // Acquire two buffers for double-buffering
   338	        let mut buf_a = self.pool.acquire().await;
   339	        let mut buf_b = self.pool.acquire().await;
   340	
   341	        // Initial read into buf_a
   342	        let mut bytes_a = file
   343	            .read(buf_a.as_mut_slice())
   344	            .await
   345	            .with_context(|| format!("reading {}", rel))?;
   346	
   347	        if bytes_a == 0 {
   348	            bail!(
   349	                "unexpected EOF while reading {} ({} bytes remaining)",
   350	                rel,
   351	                remaining
   352	            );
   353	        }
   354	        // Clamp to the declared size before subtracting. A source that
   355	        // returns more bytes than `header.size` — a file that grew after
   356	        // the manifest was computed, or a lying `TransferSource` — would
   357	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   358	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   359	        // framed stream. We send exactly `header.size` and ignore excess.
   360	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   361	        remaining -= bytes_a as u64;
   362	
   363	        // Main loop: write buf_a while reading into buf_b
   364	        while remaining > 0 {
   365	            // Per-stream telemetry: time ONLY the socket write as the
   366	            // backpressure signal. ue-r2-1e (carried ue-r2-1a review
   367	            // finding): the old code timed the whole overlapped
   368	            // write+read join, so a slow disk READ inflated
   369	            // "write blocked" and would bias the dial tuner
   370	            // conservative. The async block's clock starts when the
   371	            // join first polls it and stops when write_all completes —
   372	            // the concurrent read neither extends nor shortens it.
   373	            // Gated on the compile-time `P::ACTIVE` constant so
   374	            // `DataPlaneSession<NoProbe>` reads no clock.
   375	            let write_slice = &buf_a.as_slice()[..bytes_a];
   376	            let stream = &mut self.stream;
   377	            let (write_outcome, read_result) = tokio::join!(
   378	                async {
   379	                    let started = if P::ACTIVE {
   380	                        Some(std::time::Instant::now())
   381	                    } else {
   382	                        None
   383	                    };
   384	                    let result = stream.write_all(write_slice).await;
   385	                    (result, started.map(|t| t.elapsed()))
   386	                },
   387	                file.read(buf_b.as_mut_slice())
   388	            );
   389	
   390	            let (write_result, write_elapsed) = write_outcome;
   391	            write_result.with_context(|| format!("sending {}", rel))?;
   392	            if let Some(elapsed) = write_elapsed {
   393	                self.probe.note_write_blocked(elapsed.as_nanos() as u64);
   394	            }
   395	            self.probe.record_bytes(bytes_a as u64);
   396	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   397	
   398	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   399	
   400	            if bytes_b == 0 && remaining > 0 {
   401	                bail!(
   402	                    "unexpected EOF while reading {} ({} bytes remaining)",
   403	                    rel,
   404	                    remaining
   405	                );
   406	            }
   407	            // Same clamp as the initial read: never subtract more than
   408	            // `remaining`, so an over-returning reader can neither
   409	            // underflow the counter nor send undeclared bytes.
   410	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   411	            remaining -= bytes_b as u64;
   412	
   413	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   414	            std::mem::swap(&mut buf_a, &mut buf_b);
   415	            bytes_a = bytes_b;
   416	        }
   417	
   418	        // Final write: send the last chunk in buf_a. This is a pure
   419	        // write (no overlapped read), so the timing is cleanly
   420	        // attributable to socket-write backpressure.
   421	        if bytes_a > 0 {
   422	            let tail_start = if P::ACTIVE {
   423	                Some(std::time::Instant::now())
   424	            } else {
   425	                None
   426	            };
   427	            self.stream
   428	                .write_all(&buf_a.as_slice()[..bytes_a])
   429	                .await
   430	                .with_context(|| format!("sending {}", rel))?;
   431	            if P::ACTIVE {
   432	                if let Some(t) = tail_start {
   433	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   434	                }
   435	            }
   436	            self.probe.record_bytes(bytes_a as u64);
   437	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   438	        }
   439	
   440	        // Buffers return to pool automatically on drop
   441	        Ok(())
   442	    }
   443	
   444	    pub async fn send_prepared_tar_shard(
   445	        &mut self,
   446	        headers: Vec<FileHeader>,
   447	        data: &[u8],
   448	    ) -> Result<()> {
   449	        let shard_len = headers.len();
   450	        let preview = headers
   451	            .first()
   452	            .map(|h| h.relative_path.as_str())
   453	            .unwrap_or("<empty>");
   454	        trace_client!(
   455	            self,
   456	            "sending tar shard with {} file(s), {} bytes (first='{}')",
   457	            shard_len,
   458	            data.len(),
   459	            preview
   460	        );

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/socket.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Shared socket policy for data-plane TCP streams (w1-2).
     2	//!
     3	//! Every data-plane socket — the push client connect, the pull client
     4	//! connect, and all daemon accept paths (push epoch-0/resize,
     5	//! pull_sync epoch-0/resize/resume) — routes through
     6	//! [`configure_data_socket`], the single owner of the
     7	//! NODELAY/keepalive/tuned-buffer policy. Before this module the
     8	//! policy existed on push sockets only; the pull direction ran with
     9	//! Nagle enabled and the tuner's `tcp_buffer_bytes` was computed and
    10	//! discarded (design map §1.1, finding
    11	//! boundaries-pull-direction-bypasses-socket-policy).
    12	//!
    13	//! design-3 added [`dial_data_plane`]: the client-side dial (bounded
    14	//! connect + policy + bounded handshake write) lives here too, so
    15	//! both data-plane connect sites share one owner and neither can
    16	//! regress to an unbounded `TcpStream::connect`.
    17	
    18	use std::io;
    19	use std::time::Duration;
    20	
    21	use eyre::Context as _;
    22	use socket2::{SockRef, TcpKeepalive};
    23	use tokio::io::AsyncWriteExt;
    24	use tokio::net::TcpStream;
    25	
    26	/// Bounded wait for a data-plane accept (w1-4: one shared pair — this
    27	/// and [`DATA_PLANE_TOKEN_TIMEOUT`] — replacing three per-file
    28	/// declarations of the same two values). R46-F7 lineage: pre-fix the
    29	/// daemon called `listener.accept().await` with no timeout — a peer
    30	/// that opened the control connection but never opened the data
    31	/// connection (or hung mid-handshake) would pin the daemon's stream
    32	/// task indefinitely, holding the listener and the queued work. 30 s
    33	/// gives a generous margin for slow networks while still bounding the
    34	/// worst case.
    35	pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
    36	/// Bounded wait for the handshake-token bytes after a TCP accept.
    37	/// R46-F7: pre-fix `read_exact(&mut token_buf).await` had no timeout —
    38	/// a peer that opened the socket and stalled would hold the stream
    39	/// worker forever. 15 s is enough for a healthy peer to send a few
    40	/// dozen bytes; anything slower is a stuck or hostile peer.
    41	pub const DATA_PLANE_TOKEN_TIMEOUT: Duration = Duration::from_secs(15);
    42	
    43	/// Idle time before the first keepalive probe (w1-3). Before this the
    44	/// sockets ran `SO_KEEPALIVE` with OS-default timing (~2 h idle on
    45	/// every supported platform) — useless on transfer timescales, while
    46	/// the comments claimed it prevented idle stream timeouts. With
    47	/// 60 s + 5 probes at 10 s, a vanished peer on an idle data socket
    48	/// (an armed resize slot, a stream waiting for work while siblings
    49	/// transfer) is detected in ~2 minutes. The complementary case — a
    50	/// stalled peer with data in flight — is StallGuard's 30 s, not
    51	/// keepalive's.
    52	pub const TCP_KEEPALIVE_IDLE: Duration = Duration::from_secs(60);
    53	/// Interval between keepalive probes once idle has elapsed.
    54	pub const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
    55	/// Unanswered probes before the connection is declared dead.
    56	pub const TCP_KEEPALIVE_RETRIES: u32 = 5;
    57	
    58	/// Apply the data-plane socket policy to a connected or accepted
    59	/// stream, in place (no `into_std`/`from_std` round trip):
    60	///
    61	/// - `TCP_NODELAY` on — **hard error**. Nagle on a data-plane socket
    62	///   silently serializes small records behind ACKs; a socket we cannot
    63	///   configure is a socket we do not use.
    64	/// - `SO_KEEPALIVE` on with explicit timing
    65	///   ([`TCP_KEEPALIVE_IDLE`]/[`TCP_KEEPALIVE_INTERVAL`]/
    66	///   [`TCP_KEEPALIVE_RETRIES`]) — best-effort, logged. Detects a
    67	///   vanished peer on an idle data socket within ~2 minutes instead of
    68	///   the OS-default ~2 hours; the kernel can refuse on exotic socket
    69	///   types (POST_REVIEW_FIXES §1.1 lineage — failure is loud, never
    70	///   fatal).
    71	/// - Send/receive buffers sized to `tcp_buffer_size` when `Some` —
    72	///   best-effort, logged. The knobs are advisory (the kernel can
    73	///   clamp); a failure here should be visible to operators chasing a
    74	///   sysctl/rlimit mismatch, never fatal. `None` = kernel default —
    75	///   the value is a connect-time snapshot of
    76	///   [`TransferDial::tcp_buffer_bytes`](crate::engine::TransferDial::tcp_buffer_bytes)
    77	///   where a dial is in scope (epoch-0 sockets therefore run kernel
    78	///   defaults; resize-ADD sockets get the ramped size), and `None`
    79	///   where none is (the pull client and the daemon push receiver hold
    80	///   no dial).
    81	///
    82	/// Errors only if `TCP_NODELAY` cannot be set (or the fd/socket
    83	/// handle is unusable, which the same call surfaces).
    84	pub fn configure_data_socket(stream: &TcpStream, tcp_buffer_size: Option<usize>) -> io::Result<()> {
    85	    let socket = SockRef::from(stream);
    86	    socket.set_tcp_nodelay(true)?;
    87	    // `set_tcp_keepalive` also flips SO_KEEPALIVE on, so this is the
    88	    // whole keepalive story in one call.
    89	    let keepalive = TcpKeepalive::new()
    90	        .with_time(TCP_KEEPALIVE_IDLE)
    91	        .with_interval(TCP_KEEPALIVE_INTERVAL)
    92	        .with_retries(TCP_KEEPALIVE_RETRIES);
    93	    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
    94	        log::warn!("set TCP keepalive on data-plane socket: {}", e);
    95	    }
    96	    if let Some(size) = tcp_buffer_size {
    97	        if let Err(e) = socket.set_send_buffer_size(size) {
    98	            log::warn!("set TCP send buffer to {} bytes: {}", size, e);
    99	        }
   100	        if let Err(e) = socket.set_recv_buffer_size(size) {
   101	            log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
   102	        }
   103	    }
   104	    Ok(())
   105	}
   106	
   107	/// design-3: dial a data-plane endpoint with the shared bounds — the
   108	/// client-side mirror of the daemon's bounded accept. Connect is
   109	/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
   110	/// every control-plane connect at the same 30 s but never reached the
   111	/// TCP data plane: a firewalled or black-holed data port — the daemon
   112	/// advertises a fresh ephemeral port per transfer, and asymmetric
   113	/// firewalls that pass the control port but block ephemerals are
   114	/// common — hung for the kernel SYN timeout, 60–127 s, with no
   115	/// message). The handshake-token write is bounded by
   116	/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
   117	/// token read. Applies [`configure_data_socket`] in between.
   118	///
   119	/// On timeout the error chain carries an `io::ErrorKind::TimedOut`
   120	/// source so `remote::retry::is_retryable` classifies it as a
   121	/// transient transport failure (`--retry` re-dials instead of giving
   122	/// up on a deterministic-looking error).
   123	pub async fn dial_data_plane(
   124	    addr: &str,
   125	    handshake: &[u8],
   126	    tcp_buffer_size: Option<usize>,
   127	) -> eyre::Result<TcpStream> {
   128	    dial_data_plane_with_timeouts(
   129	        addr,
   130	        handshake,
   131	        tcp_buffer_size,
   132	        DATA_PLANE_ACCEPT_TIMEOUT,
   133	        DATA_PLANE_TOKEN_TIMEOUT,
   134	    )
   135	    .await
   136	}
   137	
   138	/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
   139	/// the bounded-failure shape without waiting out the production 30 s.
   140	async fn dial_data_plane_with_timeouts(
   141	    addr: &str,
   142	    handshake: &[u8],
   143	    tcp_buffer_size: Option<usize>,
   144	    connect_timeout: Duration,
   145	    token_timeout: Duration,
   146	) -> eyre::Result<TcpStream> {
   147	    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
   148	        Ok(connected) => connected.with_context(|| format!("connecting data plane {addr}"))?,
   149	        Err(_) => {
   150	            return Err(eyre::Report::new(io::Error::new(
   151	                io::ErrorKind::TimedOut,
   152	                format!("connect did not complete within {connect_timeout:?}"),
   153	            ))
   154	            .wrap_err(format!(
   155	                "data-plane connect to {addr} timed out after {connect_timeout:?} — the \
   156	                 port is likely unreachable (the daemon advertises a fresh ephemeral \
   157	                 data port per transfer; a firewall that passes the control port but \
   158	                 blocks ephemeral ports produces exactly this failure)"
   159	            )));
   160	        }
   161	    };
   162	    configure_data_socket(&stream, tcp_buffer_size).context("setting TCP_NODELAY")?;
   163	    match tokio::time::timeout(token_timeout, stream.write_all(handshake)).await {
   164	        Ok(written) => {
   165	            written.with_context(|| format!("writing data-plane handshake token to {addr}"))?
   166	        }
   167	        Err(_) => {
   168	            return Err(eyre::Report::new(io::Error::new(
   169	                io::ErrorKind::TimedOut,
   170	                format!("handshake write did not complete within {token_timeout:?}"),
   171	            ))
   172	            .wrap_err(format!(
   173	                "data-plane handshake to {addr} stalled for {token_timeout:?} — the peer \
   174	                 accepted the connection but is not reading"
   175	            )));
   176	        }
   177	    }
   178	    Ok(stream)
   179	}
   180	
   181	#[cfg(test)]
   182	mod tests {
   183	    use super::*;
   184	    use tokio::net::TcpListener;
   185	
   186	    async fn loopback_pair() -> (TcpStream, TcpStream) {
   187	        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
   188	        let addr = listener.local_addr().expect("addr");
   189	        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
   190	        let (server, _) = accepted.expect("accept");
   191	        (client.expect("connect"), server)
   192	    }
   193	
   194	    /// The full policy lands on the socket: nodelay and keepalive read
   195	    /// back true, and both buffer directions honor (at least) the
   196	    /// requested size — kernels may round up (Linux doubles), never
   197	    /// silently ignore a size this small.
   198	    #[tokio::test]
   199	    async fn applies_nodelay_keepalive_and_buffers() {
   200	        let (client, _server) = loopback_pair().await;
   201	        let requested = 256 * 1024;
   202	        configure_data_socket(&client, Some(requested)).expect("configure");
   203	
   204	        let sock = SockRef::from(&client);
   205	        assert!(
   206	            sock.tcp_nodelay().expect("read nodelay"),
   207	            "TCP_NODELAY must be on"
   208	        );
   209	        assert!(
   210	            sock.keepalive().expect("read keepalive"),
   211	            "SO_KEEPALIVE must be on"
   212	        );
   213	        assert!(
   214	            sock.send_buffer_size().expect("read sndbuf") >= requested,
   215	            "send buffer must be at least the requested size"
   216	        );
   217	        assert!(
   218	            sock.recv_buffer_size().expect("read rcvbuf") >= requested,
   219	            "recv buffer must be at least the requested size"
   220	        );

exec
/usr/bin/zsh -lc "rg -n \"pub struct AbortOnDrop|impl<T> AbortOnDrop|struct AbortOnDrop|AbortOnDrop\" crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/transfer_session/mod.rs:45:use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
crates/blit-core/src/transfer_session/mod.rs:720:    // AbortOnDrop: an early error return below must abort the receive
crates/blit-core/src/transfer_session/mod.rs:722:    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
crates/blit-core/src/transfer_session/mod.rs:1060:    //     `finish()` future drops the data plane, and its `AbortOnDrop`
crates/blit-core/src/transfer_session/mod.rs:1708:    // the in-stream control loop applies inline; AbortOnDrop (inside the
crates/blit-core/src/transfer_session/data_plane.rs:59:    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
crates/blit-core/src/transfer_session/data_plane.rs:157:    task: AbortOnDrop<Result<ReceiveTotals>>,
crates/blit-core/src/transfer_session/data_plane.rs:196:        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, arm_rx)));
crates/blit-core/src/transfer_session/data_plane.rs:232:        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
crates/blit-core/src/transfer_session/data_plane.rs:538:    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
crates/blit-core/src/transfer_session/data_plane.rs:540:    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
crates/blit-core/src/transfer_session/data_plane.rs:618:    // Bounded by AbortOnDrop: a fault on the control lane that drops the
crates/blit-core/src/transfer_session/data_plane.rs:620:    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/transfer_session/data_plane.rs:700:    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/transfer_session/data_plane.rs:771:    /// via AbortOnDrop, so its armed slot never orphans. (Old push
crates/blit-core/src/remote/pull.rs:21:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/pull.rs:617:        // R32-F2: AbortOnDrop so an outer cancellation aborts the
crates/blit-core/src/remote/pull.rs:623:        let manifest_send_task = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/pull.rs:660:        // R32-F2: wrap the data-plane handle in AbortOnDrop so an
crates/blit-core/src/remote/pull.rs:663:        let mut data_plane_handle: Option<AbortOnDrop<Result<DataPlaneResult>>> = None;
crates/blit-core/src/remote/pull.rs:871:                    data_plane_handle = Some(AbortOnDrop::new(handle));
crates/blit-core/src/remote/pull.rs:1100:        // `.join()` keeps the AbortOnDrop wrapper alive across the
crates/blit-core/src/remote/pull.rs:1729:    // AbortOnDrop set became a JoinSet, which aborts every remaining
crates/blit-core/src/remote/pull.rs:2040:    //! `AbortOnDrop`'s own contract (drop-without-consume aborts,
crates/blit-core/src/remote/pull.rs:2172:        // `pull_sync_with_spec`'s AbortOnDrop data-plane handle does
crates/blit-core/src/remote/pull.rs:2202:        let guard = super::AbortOnDrop::new(tokio::spawn(receive_data_plane_streams_owned(
crates/blit-core/src/remote/push/client/mod.rs:41:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/push/client/mod.rs:64:/// w4-1: takes `AbortOnDrop` (not a bare `JoinHandle`) and drains via
crates/blit-core/src/remote/push/client/mod.rs:67:async fn drain_pipeline_outcome(handle: AbortOnDrop<Result<SinkOutcome>>) -> Result<SinkOutcome> {
crates/blit-core/src/remote/push/client/mod.rs:87:async fn drain_pipeline_error(handle: AbortOnDrop<Result<SinkOutcome>>) -> eyre::Report {
crates/blit-core/src/remote/push/client/mod.rs:114:    /// w4-1: `AbortOnDrop`, not a bare `JoinHandle` — if `push()`
crates/blit-core/src/remote/push/client/mod.rs:119:    pipeline_handle: Option<AbortOnDrop<Result<SinkOutcome>>>,
crates/blit-core/src/remote/push/client/mod.rs:297:        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/push/client/mod.rs:1602:    //! needs the `pipeline_handle` field wired through `AbortOnDrop`.
crates/blit-core/src/remote/push/client/mod.rs:1618:        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/push/client/mod.rs:1673:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1696:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1721:        let handle = AbortOnDrop::new(tokio::spawn(async move { Ok(cloned) }));
crates/blit-core/src/remote/push/client/mod.rs:1733:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1752:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1773:        let handle = AbortOnDrop::new(tokio::spawn(async { Ok(SinkOutcome::default()) }));
crates/blit-core/src/remote/push/client/helpers.rs:17:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/push/client/helpers.rs:256:/// w4-1: the returned task is wrapped in `AbortOnDrop` so a caller
crates/blit-core/src/remote/push/client/helpers.rs:265:    AbortOnDrop<()>,
crates/blit-core/src/remote/push/client/helpers.rs:268:    let task = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/mod.rs:17:pub use abort_on_drop::AbortOnDrop;
crates/blit-core/src/remote/transfer/pipeline.rs:176:                    // cancellation) is the AbortOnDrop family, w4-1.
crates/blit-core/src/remote/transfer/pipeline.rs:2019:    /// AbortOnDrop family tracked under w4-1.
crates/blit-core/src/remote/transfer/abort_on_drop.rs:25:pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
crates/blit-core/src/remote/transfer/abort_on_drop.rs:27:impl<T> AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:46:            .expect("AbortOnDrop already consumed (programming error)");
crates/blit-core/src/remote/transfer/abort_on_drop.rs:56:impl<T> Drop for AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:66:    //! Regression tests for the `AbortOnDrop` wrapper that bounds
crates/blit-core/src/remote/transfer/abort_on_drop.rs:73:    use super::AbortOnDrop;
crates/blit-core/src/remote/transfer/abort_on_drop.rs:89:        // wrapping in AbortOnDrop and dropping immediately must
crates/blit-core/src/remote/transfer/abort_on_drop.rs:94:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:110:            "task ran to completion despite AbortOnDrop being dropped"
crates/blit-core/src/remote/transfer/abort_on_drop.rs:122:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:138:        let guard = AbortOnDrop::new(tokio::spawn(async {}));
crates/blit-core/src/remote/transfer/abort_on_drop.rs:162:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:186:             AbortOnDrop is leaking the handle out before the await again"

exec
/usr/bin/zsh -lc "rg -n \"JoinSet\" ~/.cargo/registry/src -g 'join_set.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:3://! This module provides the [`JoinSet`] type, a collection which stores a set
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:5://! tasks as they complete. See the documentation for the [`JoinSet`] type for
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:19:/// A `JoinSet` can be used to await the completion of some or all of the tasks
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:25:/// When the `JoinSet` is dropped, all tasks in the `JoinSet` are immediately aborted.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:32:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:36:/// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:56:/// While a task is tracked in a `JoinSet`, that task's ID is unique relative
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:58:/// `JoinSet` is equivalent to holding a [`JoinHandle`] to it. See the [task ID]
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:64:pub struct JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:68:/// A variant of [`task::Builder`] that spawns tasks on a [`JoinSet`] rather
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:76:    joinset: &'a mut JoinSet<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:80:impl<T> JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:81:    /// Create a new `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:88:    /// Returns the number of tasks currently in the `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:93:    /// Returns whether the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:99:impl<T: 'static> JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:101:    /// spawning it on this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:106:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:110:    ///     let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:129:    /// Spawn the provided task on the `JoinSet`, returning an [`AbortHandle`]
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:134:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:152:    /// `JoinSet` returning an [`AbortHandle`] that can be used to remotely
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:157:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:171:    /// and store it in this `JoinSet`, returning an [`AbortHandle`] that can
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:176:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:195:    /// this `JoinSet`, returning an [`AbortHandle`] that can be used to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:215:    /// it in this `JoinSet`, returning an [`AbortHandle`] that can be
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:225:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:229:    ///     let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:264:    /// provided runtime and store it in this `JoinSet`, returning an
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:295:    /// removed from this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:312:    /// removed from this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:377:    /// `JoinSet` will be empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:386:    /// Awaits the completion of all tasks in this `JoinSet`, returning a vector of their results.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:390:    /// a loop. If any tasks on the `JoinSet` fail with an [`JoinError`], then this call
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:391:    /// to `join_all` will panic and all remaining tasks on the `JoinSet` are
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:400:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:405:    /// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:422:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:427:    /// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:459:    /// Aborts all tasks on this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:461:    /// This does not remove the tasks from the `JoinSet`. To wait for the tasks to complete
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:462:    /// cancellation, you should call `join_next` in a loop until the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:467:    /// Removes all tasks from this `JoinSet` without aborting them.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:469:    /// The tasks removed by this call will continue to run in the background even if the `JoinSet`
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:480:    /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:488:    ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:490:    ///  * `Poll::Ready(Some(Ok(value)))` if one of the tasks in this `JoinSet` has completed.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:492:    ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:494:    ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:534:    /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:542:    ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:544:    ///  * `Poll::Ready(Some(Ok((id, value))))` if one of the tasks in this `JoinSet` has completed.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:547:    ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:549:    ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:591:impl<T> Drop for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:597:impl<T> fmt::Debug for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:599:        f.debug_struct("JoinSet").field("len", &self.len()).finish()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:603:impl<T> Default for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:609:/// Collect an iterator of futures into a [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:611:/// This is equivalent to calling [`JoinSet::spawn`] on each element of the iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:615:/// The main example from [`JoinSet`]'s documentation can also be written using [`collect`]:
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:618:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:622:/// let mut set: JoinSet<_> = (0..10).map(|i| async move { i }).collect();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:637:impl<T, F> std::iter::FromIterator<F> for JoinSet<T>
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:652:/// Extend a [`JoinSet`] with futures from an iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:654:/// This is equivalent to calling [`JoinSet::spawn`] on each element of the iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:661:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:665:///     let mut set: JoinSet<_> = (0..5).map(|i| async move { i }).collect();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:681:impl<T, F> std::iter::Extend<F> for JoinSet<T>
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:709:    /// [`JoinSet`], returning an [`AbortHandle`] that can be used to remotely
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:732:    /// builder's settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:752:    /// settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:762:    /// [`JoinSet`]: crate::task::JoinSet
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:776:    /// [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:782:    /// [`JoinSet`]: crate::task::JoinSet
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:797:    /// with this builder's settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.3/src/task/join_set.rs:820:    /// settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:3://! This module provides the [`JoinSet`] type, a collection which stores a set
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:5://! tasks as they complete. See the documentation for the [`JoinSet`] type for
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:19:/// A `JoinSet` can be used to await the completion of some or all of the tasks
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:25:/// When the `JoinSet` is dropped, all tasks in the `JoinSet` are immediately aborted.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:32:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:36:/// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:56:/// While a task is tracked in a `JoinSet`, that task's ID is unique relative
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:58:/// `JoinSet` is equivalent to holding a [`JoinHandle`] to it. See the [task ID]
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:64:pub struct JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:68:/// A variant of [`task::Builder`] that spawns tasks on a [`JoinSet`] rather
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:76:    joinset: &'a mut JoinSet<T>,
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:80:impl<T> JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:81:    /// Create a new `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:88:    /// Returns the number of tasks currently in the `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:93:    /// Returns whether the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:99:impl<T: 'static> JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:101:    /// spawning it on this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:106:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:110:    ///     let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:129:    /// Spawn the provided task on the `JoinSet`, returning an [`AbortHandle`]
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:134:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:152:    /// `JoinSet` returning an [`AbortHandle`] that can be used to remotely
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:157:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:171:    /// and store it in this `JoinSet`, returning an [`AbortHandle`] that can
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:176:    /// `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:195:    /// this `JoinSet`, returning an [`AbortHandle`] that can be used to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:215:    /// it in this `JoinSet`, returning an [`AbortHandle`] that can be
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:225:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:229:    ///     let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:264:    /// provided runtime and store it in this `JoinSet`, returning an
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:295:    /// removed from this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:312:    /// removed from this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:377:    /// `JoinSet` will be empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:386:    /// Awaits the completion of all tasks in this `JoinSet`, returning a vector of their results.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:390:    /// a loop. If any tasks on the `JoinSet` fail with an [`JoinError`], then this call
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:391:    /// to `join_all` will panic and all remaining tasks on the `JoinSet` are
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:400:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:405:    /// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:422:    /// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:427:    /// let mut set = JoinSet::new();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:459:    /// Aborts all tasks on this `JoinSet`.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:461:    /// This does not remove the tasks from the `JoinSet`. To wait for the tasks to complete
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:462:    /// cancellation, you should call `join_next` in a loop until the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:467:    /// Removes all tasks from this `JoinSet` without aborting them.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:469:    /// The tasks removed by this call will continue to run in the background even if the `JoinSet`
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:480:    /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:488:    ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:490:    ///  * `Poll::Ready(Some(Ok(value)))` if one of the tasks in this `JoinSet` has completed.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:492:    ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:494:    ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:534:    /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:542:    ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:544:    ///  * `Poll::Ready(Some(Ok((id, value))))` if one of the tasks in this `JoinSet` has completed.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:547:    ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:549:    ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:591:impl<T> Drop for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:597:impl<T> fmt::Debug for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:599:        f.debug_struct("JoinSet").field("len", &self.len()).finish()
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:603:impl<T> Default for JoinSet<T> {
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:609:/// Collect an iterator of futures into a [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:611:/// This is equivalent to calling [`JoinSet::spawn`] on each element of the iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:615:/// The main example from [`JoinSet`]'s documentation can also be written using [`collect`]:
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:618:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:622:/// let mut set: JoinSet<_> = (0..10).map(|i| async move { i }).collect();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:637:impl<T, F> std::iter::FromIterator<F> for JoinSet<T>
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:652:/// Extend a [`JoinSet`] with futures from an iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:654:/// This is equivalent to calling [`JoinSet::spawn`] on each element of the iterator.
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:661:/// use tokio::task::JoinSet;
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:665:///     let mut set: JoinSet<_> = (0..5).map(|i| async move { i }).collect();
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:681:impl<T, F> std::iter::Extend<F> for JoinSet<T>
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:709:    /// [`JoinSet`], returning an [`AbortHandle`] that can be used to remotely
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:732:    /// builder's settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:752:    /// settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:762:    /// [`JoinSet`]: crate::task::JoinSet
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:776:    /// [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:782:    /// [`JoinSet`]: crate::task::JoinSet
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:797:    /// with this builder's settings, and store it in the [`JoinSet`].
/home/michael/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.52.2/src/task/join_set.rs:820:    /// settings, and store it in the [`JoinSet`].

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/abort_on_drop.rs | sed -n '1,90p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! RAII wrapper that aborts the underlying tokio task when dropped
     2	//! without awaiting (R32-F2; hoisted to a shared location under
     3	//! `w4-1` so every spawn family — not just `pull.rs` — can use it).
     4	//!
     5	//! `JoinHandle::drop` detaches; it does NOT cancel the spawned task.
     6	//! That's a real bug wherever a spawned task's lifetime is meant to
     7	//! be bounded by a calling future: when the outer future is dropped
     8	//! (e.g. CLI Ctrl-C cancels the gRPC stream from the daemon's
     9	//! `delegated_pull` handler, or an early `?` return exits a handler
    10	//! while a data-plane task is still running), a bare `JoinHandle`
    11	//! left running would otherwise keep reading sockets and writing
    12	//! files with no owner.
    13	//!
    14	//! Usage: wrap every `tokio::spawn` whose lifetime should be bounded
    15	//! by the calling future. Await with `.join().await` — that holds
    16	//! `self` across the await so a parent-future cancellation during
    17	//! the await still triggers `abort()` via Drop. Do NOT add an
    18	//! `into_inner()` accessor: returning the bare `JoinHandle` and then
    19	//! awaiting it re-introduces the cancellation gap (R34-F2 — the bare
    20	//! handle is dropped on parent-future cancel and detaches the task
    21	//! instead of aborting it).
    22	
    23	use tokio::task::JoinHandle;
    24	
    25	pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
    26	
    27	impl<T> AbortOnDrop<T> {
    28	    pub fn new(handle: JoinHandle<T>) -> Self {
    29	        Self(Some(handle))
    30	    }
    31	
    32	    /// Await the spawned task while keeping `self` alive across the
    33	    /// await. If the surrounding future is cancelled during the
    34	    /// await, `self` is dropped and our `Drop` impl fires `abort()`.
    35	    /// Compare to a hypothetical `into_inner().await` pattern, which
    36	    /// would release the guard before awaiting — that's the
    37	    /// cancellation-gap bug R34-F2 fixed.
    38	    pub async fn join(mut self) -> std::result::Result<T, tokio::task::JoinError> {
    39	        // Borrow the JoinHandle out of the Option, but DON'T move it
    40	        // out of `self`. `self` lives across this await; if the
    41	        // surrounding future is cancelled here, `self` drops and
    42	        // `Drop::drop` aborts the still-owned handle.
    43	        let handle = self
    44	            .0
    45	            .as_mut()
    46	            .expect("AbortOnDrop already consumed (programming error)");
    47	        let result = handle.await;
    48	        // Task completed (success or panic). Clear the slot so the
    49	        // trailing Drop after this returns is a no-op rather than
    50	        // calling abort() on an already-finished handle.
    51	        self.0 = None;
    52	        result
    53	    }
    54	}
    55	
    56	impl<T> Drop for AbortOnDrop<T> {
    57	    fn drop(&mut self) {
    58	        if let Some(handle) = self.0.take() {
    59	            handle.abort();
    60	        }
    61	    }
    62	}
    63	
    64	#[cfg(test)]
    65	mod tests {
    66	    //! Regression tests for the `AbortOnDrop` wrapper that bounds
    67	    //! spawned tasks across the workspace (R32-F2, hoisted under
    68	    //! `w4-1`). Without this, dropping the `JoinHandle` would detach
    69	    //! the spawned task — meaning a cancelled parent future (CLI
    70	    //! Ctrl-C, an early `?` return, etc.) couldn't actually stop a
    71	    //! running background task.
    72	
    73	    use super::AbortOnDrop;
    74	    use std::sync::atomic::{AtomicBool, Ordering};
    75	    use std::sync::Arc;
    76	    use std::time::Duration;
    77	
    78	    // Paused virtual time (w4-1 codex review): the relocated test
    79	    // waited only 150ms real time against the task's 500ms natural
    80	    // completion, so it passed whether or not Drop aborted — vacuous
    81	    // since its pull.rs days. Under start_paused the auto-advancing
    82	    // clock deterministically runs a detached task's 500ms sleep
    83	    // BEFORE the test's 700ms wake, so a Drop impl that detaches
    84	    // instead of aborting now fails the assertion, with no wall-clock
    85	    // sensitivity.
    86	    #[tokio::test(start_paused = true)]
    87	    async fn drop_without_consume_aborts_running_task() {
    88	        // The task tries to set the "completed" flag after a delay;
    89	        // wrapping in AbortOnDrop and dropping immediately must
    90	        // prevent the flag from ever being set.

exec
/usr/bin/zsh -lc "rg -n \"run_responder|transfer\\(|SourceResponderTarget|DestinationTarget|session_error_frame|ActiveJobs|CancelJob|CANCELLED|BUILD_MISMATCH\" crates/blit-daemon/src/service crates/blit-core/src/transfer_session -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer_session_e2e.rs:66:    active_jobs: crate::active_jobs::ActiveJobs,
crates/blit-daemon/src/service/transfer_session_e2e.rs:287:/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
crates/blit-daemon/src/service/transfer_session_e2e.rs:290:/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
crates/blit-daemon/src/service/transfer_session_e2e.rs:320:    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
crates/blit-daemon/src/service/transfer_session_e2e.rs:337:    // The client must surface CANCELLED promptly (no hang).
crates/blit-daemon/src/service/transfer_session_e2e.rs:346:        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
crates/blit-core/src/transfer_session/mod.rs:12://! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
crates/blit-core/src/transfer_session/mod.rs:151:    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
crates/blit-core/src/transfer_session/mod.rs:277:/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
crates/blit-core/src/transfer_session/mod.rs:281:/// they are only meaningful for `BUILD_MISMATCH`.
crates/blit-core/src/transfer_session/mod.rs:282:pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
crates/blit-core/src/transfer_session/mod.rs:299:/// [`DestinationTarget::Fixed`] instead.
crates/blit-core/src/transfer_session/mod.rs:331:pub enum DestinationTarget {
crates/blit-core/src/transfer_session/mod.rs:337:/// [`DestinationTarget`]: `Fixed` is a source known up front (an
crates/blit-core/src/transfer_session/mod.rs:345:/// [`run_responder`] for the daemon-as-SOURCE (pull-equivalent, otp-5).
crates/blit-core/src/transfer_session/mod.rs:346:pub enum SourceResponderTarget {
crates/blit-core/src/transfer_session/mod.rs:352:/// played. `run_responder` dispatches on the initiator's declared role,
crates/blit-core/src/transfer_session/mod.rs:412:/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
crates/blit-core/src/transfer_session/mod.rs:458:/// reads the open then calls this) and `run_responder` (which reads the
crates/blit-core/src/transfer_session/mod.rs:680:        // `run_responder`, not here (otp-5).
crates/blit-core/src/transfer_session/mod.rs:698:/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
crates/blit-core/src/transfer_session/mod.rs:997:                    // peer's framed reason (CANCELLED) the same way the
crates/blit-core/src/transfer_session/mod.rs:1053:    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
crates/blit-core/src/transfer_session/mod.rs:1054:    // the served session frames `SessionError{CANCELLED}`, and the source
crates/blit-core/src/transfer_session/mod.rs:1280:/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
crates/blit-core/src/transfer_session/mod.rs:1444:/// `target` is [`DestinationTarget::Fixed`] when the root is known up
crates/blit-core/src/transfer_session/mod.rs:1446:/// [`DestinationTarget::Resolve`] when the root must be resolved from
crates/blit-core/src/transfer_session/mod.rs:1452:    target: DestinationTarget,
crates/blit-core/src/transfer_session/mod.rs:1477:        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
crates/blit-core/src/transfer_session/mod.rs:1478:        DestinationTarget::Fixed(_) => None,
crates/blit-core/src/transfer_session/mod.rs:1496:            DestinationTarget::Fixed(root) => root.clone(),
crates/blit-core/src/transfer_session/mod.rs:1500:            DestinationTarget::Resolve(_) => {
crates/blit-core/src/transfer_session/mod.rs:1519:/// [`run_responder`] (the daemon DESTINATION responder), so the receive
crates/blit-core/src/transfer_session/mod.rs:1549:pub async fn run_responder(
crates/blit-core/src/transfer_session/mod.rs:1552:    source_target: SourceResponderTarget,
crates/blit-core/src/transfer_session/mod.rs:1553:    dest_target: DestinationTarget,
crates/blit-core/src/transfer_session/mod.rs:1575:                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
crates/blit-core/src/transfer_session/mod.rs:1576:                DestinationTarget::Fixed(_) => None,
crates/blit-core/src/transfer_session/mod.rs:1589:                    DestinationTarget::Fixed(root) => root.clone(),
crates/blit-core/src/transfer_session/mod.rs:1590:                    DestinationTarget::Resolve(_) => {
crates/blit-core/src/transfer_session/mod.rs:1605:                SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
crates/blit-core/src/transfer_session/mod.rs:1606:                SourceResponderTarget::Fixed(_) => None,
crates/blit-core/src/transfer_session/mod.rs:1617:                SourceResponderTarget::Fixed(source) => source,
crates/blit-core/src/transfer_session/mod.rs:1618:                SourceResponderTarget::Resolve(_) => {
crates/blit-core/src/transfer_session/mod.rs:2238:    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
crates/blit-core/src/transfer_session/mod.rs:2245:        // The peer framed CANCELLED on the control lane before we ask.
crates/blit-core/src/transfer_session/mod.rs:2248:            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:2266:            "the framed CANCELLED must win over the data-plane break"
crates/blit-core/src/transfer_session/mod.rs:2271:    /// legitimate `Need` may be queued ahead of the peer's `CANCELLED`.
crates/blit-core/src/transfer_session/mod.rs:2272:    /// `prefer_peer_fault` must SKIP it and still surface CANCELLED — not
crates/blit-core/src/transfer_session/mod.rs:2286:            message: "transfer cancelled via CancelJob".into(),
crates/blit-daemon/src/service/transfer.rs:6://! serve BOTH roles: it runs `blit_core::transfer_session::run_responder`,
crates/blit-daemon/src/service/transfer.rs:37:    run_responder, DestinationTarget, HelloConfig, OpenResolver, ResolvedEndpoint, SessionFault,
crates/blit-daemon/src/service/transfer.rs:38:    SourceResponderTarget,
crates/blit-daemon/src/service/transfer.rs:97:/// the client's declared initiator role via [`run_responder`]: a SOURCE
crates/blit-daemon/src/service/transfer.rs:119:    let outcome = run_responder(
crates/blit-daemon/src/service/transfer.rs:122:        SourceResponderTarget::Resolve(source_resolver),
crates/blit-daemon/src/service/transfer.rs:123:        DestinationTarget::Resolve(dest_resolver),
crates/blit-daemon/src/service/transfer.rs:133:            // run_responder already emitted a SessionError frame to the
crates/blit-daemon/src/service/core.rs:11:use crate::active_jobs::{ActiveJobKind, ActiveJobs, CancelOutcome};
crates/blit-daemon/src/service/core.rs:17:    daemon_event, ActiveTransfer, CancelJobRequest, CancelJobResponse, ClearRecentRequest,
crates/blit-daemon/src/service/core.rs:78:    pub(crate) active_jobs: ActiveJobs,
crates/blit-daemon/src/service/core.rs:110:            active_jobs: ActiveJobs::new(),
crates/blit-daemon/src/service/core.rs:190:        // c-5b: emit via ActiveJobs so the event lands in the
crates/blit-daemon/src/service/core.rs:221:    active_jobs: &crate::active_jobs::ActiveJobs,
crates/blit-daemon/src/service/core.rs:307:/// ActiveJobs ring records. Pairs with `emit_transfer_started` on
crates/blit-daemon/src/service/core.rs:362:    async fn transfer(
crates/blit-daemon/src/service/core.rs:373:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:377:        // the row still supports CancelJob and appears in GetState, and
crates/blit-daemon/src/service/core.rs:395:            // SessionError{CANCELLED}, not a bare Status (codex F1).
crates/blit-daemon/src/service/core.rs:535:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:536:        // ActiveJobs row registered with empty module/path —
crates/blit-daemon/src/service/core.rs:631:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:692:        // ActiveJobs row mirrors the metrics gauge — both are
crates/blit-daemon/src/service/core.rs:721:        // failure, or `CancelJob(transfer_id)` regardless of
crates/blit-daemon/src/service/core.rs:773:            //   cancel_token.cancelled() → `CancelJob` RPC fired the
crates/blit-daemon/src/service/core.rs:780:            //   None         → cancelled (client OR CancelJob)
crates/blit-daemon/src/service/core.rs:790:            // a hangup / `CancelJob`. See that helper for the rationale.
crates/blit-daemon/src/service/core.rs:807:            // Map the select outcome onto the ActiveJobs ring
crates/blit-daemon/src/service/core.rs:818:            //   None        → client hangup or CancelJob.
crates/blit-daemon/src/service/core.rs:822:            //                  CancelJob; otherwise it was the
crates/blit-daemon/src/service/core.rs:828:                    (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1118:        request: Request<CancelJobRequest>,
crates/blit-daemon/src/service/core.rs:1119:    ) -> Result<Response<CancelJobResponse>, Status> {
crates/blit-daemon/src/service/core.rs:1127:                "CancelJobRequest.transfer_id must not be empty",
crates/blit-daemon/src/service/core.rs:1130:        // `ActiveJobs::cancel_authorized` is synchronous and short — the
crates/blit-daemon/src/service/core.rs:1135:            CancelOutcome::Cancelled => Ok(Response::new(CancelJobResponse {
crates/blit-daemon/src/service/core.rs:1157:    /// on `ActiveJobs` only ever references the recents ring + its own
crates/blit-daemon/src/service/core.rs:1320:/// `CancelJob` cancel, both of which resolve to `None` so the caller
crates/blit-daemon/src/service/core.rs:1324:/// transfer that completed at the same instant `CancelJob` fired its
crates/blit-daemon/src/service/core.rs:1325:/// token was mis-recorded as "cancelled via CancelJob" despite having
crates/blit-daemon/src/service/core.rs:1352:/// row's `CancelJob` token via [`resolve_transfer_outcome`].
crates/blit-daemon/src/service/core.rs:1358:/// unobservable work that `CancelJob` also refused to touch
crates/blit-daemon/src/service/core.rs:1372:/// Returns the `(ok, error_message)` pair the ActiveJobs ring records:
crates/blit-daemon/src/service/core.rs:1378:/// - cancel token fired → `(false, "cancelled via CancelJob")`, and the
crates/blit-daemon/src/service/core.rs:1405:        // token means the cause was CancelJob; otherwise the client
crates/blit-daemon/src/service/core.rs:1409:                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
crates/blit-daemon/src/service/core.rs:1411:            (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1419:/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
crates/blit-daemon/src/service/core.rs:1448:                .send(Ok(blit_core::transfer_session::session_error_frame(
crates/blit-daemon/src/service/core.rs:1450:                    "transfer cancelled via CancelJob",
crates/blit-daemon/src/service/core.rs:1453:            (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1460:/// `(ok, error_message)` pair the ActiveJobs guard expects.
crates/blit-daemon/src/service/core.rs:1485:    /// instant `CancelJob` fired gets mis-recorded as cancelled.
crates/blit-daemon/src/service/core.rs:1503:    /// `CancelJob` cancel — the fix must not make transfers
crates/blit-daemon/src/service/core.rs:1511:            ready(()),         // CancelJob fired
crates/blit-daemon/src/service/core.rs:1518:    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
crates/blit-daemon/src/service/core.rs:1519:    /// must reach the client as a framed `SessionError{CANCELLED}` on
crates/blit-daemon/src/service/core.rs:1537:        assert_eq!(msg.as_deref(), Some("cancelled via CancelJob"));
crates/blit-daemon/src/service/core.rs:1548:                "cancel must emit a framed CANCELLED SessionError"
crates/blit-daemon/src/service/core.rs:1550:            other => panic!("expected a CANCELLED error frame, got {other:?}"),
crates/blit-daemon/src/service/core.rs:1597:    /// handler as `(false, "cancelled via CancelJob")` and deliver a
crates/blit-daemon/src/service/core.rs:1611:        assert_eq!(err.as_deref(), Some("cancelled via CancelJob"));
crates/blit-daemon/src/service/core.rs:1820:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1835:        // D-2026-07-04-3: CancelJob dispatch fires the row token for
crates/blit-daemon/src/service/core.rs:1847:                .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1855:                "{}: CancelJob must fire the row token",
crates/blit-daemon/src/service/core.rs:1878:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1882:            .expect_err("a policy-gated kind must reject CancelJob");
crates/blit-daemon/src/service/core.rs:1886:            "token must NOT be fired when CancelJob is unsupported"
crates/blit-daemon/src/service/core.rs:1894:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1906:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:2225:        // ActiveJobs::register emits no events directly — only the
crates/blit-daemon/src/service/core.rs:2299:    async fn progress_event_cannot_arrive_after_terminal_for_same_transfer() {
crates/blit-daemon/src/service/core.rs:2442:    async fn event_matches_filter_matches_only_target_transfer() {
crates/blit-daemon/src/service/pull_sync.rs:70:    // Populate the ActiveJobs row now that we know the
crates/blit-daemon/src/service/pull_sync.rs:249:    let files_to_send = files_needing_transfer(&diff);
crates/blit-daemon/src/service/push/control.rs:112:                // Populate the ActiveJobs row now that we know
crates/blit-daemon/src/service/push/control.rs:815:    //! running with no owner — unreachable by `CancelJob`. This pins
crates/blit-daemon/src/service/delegated_pull.rs:277:    let _guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/delegated_pull.rs:304:    // TCP SYN timeout (60-180s on Linux), pinning the ActiveJobs row and

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer.rs | sed -n '260,560p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
 succeeded in 0ms:
     1	//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
     2	//!
     3	//! otp-1 (D-2026-07-05-4) landed the wire surface (the RPC, the frame
     4	//! set, `docs/TRANSFER_SESSION.md`) with the handler refusing
     5	//! UNIMPLEMENTED. otp-4a landed the behavior; otp-5a makes the daemon
     6	//! serve BOTH roles: it runs `blit_core::transfer_session::run_responder`,
     7	//! which dispatches on the client's declared initiator role — a SOURCE
     8	//! initiator makes the daemon the DESTINATION (push-equivalent), a
     9	//! DESTINATION initiator makes it the SOURCE (pull-equivalent, streaming
    10	//! its module tree). The dispatcher in `core.rs::transfer` mirrors
    11	//! `push`: register a jobs row, race the session against cancel/hangup
    12	//! via `resolve_streaming_outcome`, return the response `ReceiverStream`.
    13	//!
    14	//! This module owns the two daemon-specific pieces the session driver
    15	//! in blit-core cannot: (1) the [`OpenResolver`] that maps a wire
    16	//! module/path to a local root and read-only decision (blit-core stays
    17	//! free of module config and `tonic::Status`), and (2) the transport
    18	//! assembly + outcome mapping.
    19	//!
    20	//! Carrier: the push-equivalent (daemon DESTINATION) rides the TCP data
    21	//! plane (otp-4b); the pull-equivalent (daemon SOURCE) is in-stream only
    22	//! until otp-5b adds the SOURCE-responder data plane. Progress-byte
    23	//! wiring (`with_byte_progress`) is not threaded yet — session rows
    24	//! report `bytes_completed=0`, matching today's push rows.
    25	
    26	use std::collections::HashMap;
    27	use std::sync::Arc;
    28	
    29	use tokio::sync::mpsc;
    30	use tokio::sync::Mutex;
    31	use tonic::{Status, Streaming};
    32	
    33	use blit_core::generated::session_error::Code;
    34	use blit_core::generated::{SessionOpen, TransferFrame};
    35	use blit_core::transfer_session::transport::grpc_daemon_transport;
    36	use blit_core::transfer_session::{
    37	    run_responder, DestinationTarget, HelloConfig, OpenResolver, ResolvedEndpoint, SessionFault,
    38	    SourceResponderTarget,
    39	};
    40	
    41	use super::util::{resolve_contained_path, resolve_module, resolve_relative_path};
    42	use crate::runtime::{ModuleConfig, RootExport};
    43	
    44	/// Map a resolver `tonic::Status` onto a `SessionError` code. blit-core
    45	/// is deliberately `Status`-free, so the daemon picks the wire code:
    46	/// an unknown module is `MODULE_UNKNOWN`, a bad or escaping path is a
    47	/// `PROTOCOL_VIOLATION` (the initiator sent an unusable request),
    48	/// anything else is `INTERNAL`.
    49	fn status_to_fault(status: Status) -> SessionFault {
    50	    let code = match status.code() {
    51	        tonic::Code::NotFound => Code::ModuleUnknown,
    52	        tonic::Code::InvalidArgument | tonic::Code::PermissionDenied => Code::ProtocolViolation,
    53	        _ => Code::Internal,
    54	    };
    55	    SessionFault::refusal(code, status.message().to_string())
    56	}
    57	
    58	/// Build the daemon's [`OpenResolver`]: given a received `SessionOpen`,
    59	/// resolve its module + path to an absolute local root and report the
    60	/// module's read-only flag. Mirrors the push Header sequence
    61	/// (`resolve_module` → path validation → F2 canonical containment via
    62	/// `resolve_contained_path`), refusing with a `SessionError` instead of
    63	/// a `tonic::Status`. The closure is `Fn` (callable once per session)
    64	/// and clones its captured handles per call so it stays `Send + Sync`.
    65	pub(crate) fn make_open_resolver(
    66	    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    67	    default_root: Option<RootExport>,
    68	) -> Box<OpenResolver> {
    69	    Box::new(move |open: &SessionOpen| {
    70	        let modules = Arc::clone(&modules);
    71	        let default_root = default_root.clone();
    72	        let module_name = open.module.clone();
    73	        let wire_path = open.path.clone();
    74	        Box::pin(async move {
    75	            let config = resolve_module(&modules, default_root.as_ref(), &module_name)
    76	                .await
    77	                .map_err(status_to_fault)?;
    78	            // Empty path targets the module root; a non-empty path is
    79	            // validated and contained against the module's canonical
    80	            // root (F2 symlink-escape protection — the same chokepoint
    81	            // the per-file write path uses).
    82	            let root = if wire_path.is_empty() {
    83	                config.path.clone()
    84	            } else {
    85	                let rel = resolve_relative_path(&wire_path).map_err(status_to_fault)?;
    86	                resolve_contained_path(&config, &rel).map_err(status_to_fault)?
    87	            };
    88	            Ok(ResolvedEndpoint {
    89	                root,
    90	                read_only: config.read_only,
    91	            })
    92	        })
    93	    })
    94	}
    95	
    96	/// Run one daemon-side transfer session to completion, dispatching on
    97	/// the client's declared initiator role via [`run_responder`]: a SOURCE
    98	/// initiator makes the daemon the DESTINATION (push-equivalent, otp-4);
    99	/// a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent,
   100	/// otp-5). Returns `Ok(())` on a clean transfer or `Err(Status)`
   101	/// carrying the session fault's message for the jobs record. The session
   102	/// communicates its own refusals to the peer as `SessionError` *frames*
   103	/// (via the response stream); this `Status` is for the daemon's outcome
   104	/// record and `resolve_streaming_outcome`'s terminal handling, not the
   105	/// primary error channel.
   106	pub(crate) async fn run_transfer_session(
   107	    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
   108	    default_root: Option<RootExport>,
   109	    inbound: Streaming<TransferFrame>,
   110	    tx: mpsc::Sender<Result<TransferFrame, Status>>,
   111	) -> Result<(), Status> {
   112	    let transport = grpc_daemon_transport(tx, inbound);
   113	    // The same module→root resolver serves both roles; only the one the
   114	    // initiator's declared role selects is consulted. Two clones so each
   115	    // target owns its resolver (the closure clones its captured handles
   116	    // per call, so this is cheap).
   117	    let source_resolver = make_open_resolver(Arc::clone(&modules), default_root.clone());
   118	    let dest_resolver = make_open_resolver(modules, default_root);
   119	    let outcome = run_responder(
   120	        HelloConfig::default(),
   121	        transport,
   122	        SourceResponderTarget::Resolve(source_resolver),
   123	        DestinationTarget::Resolve(dest_resolver),
   124	    )
   125	    .await;
   126	    match outcome {
   127	        // Either role completing cleanly is a successful transfer; the
   128	        // daemon record does not distinguish push- from pull-equivalent
   129	        // (the jobs kind stays Push until the taxonomy is revisited at
   130	        // cutover — see the dispatcher).
   131	        Ok(_) => Ok(()),
   132	        Err(report) => {
   133	            // run_responder already emitted a SessionError frame to the
   134	            // peer; surface the reason for the record.
   135	            let msg = report
   136	                .downcast_ref::<SessionFault>()
   137	                .map(|f| f.message.clone())
   138	                .unwrap_or_else(|| format!("{report:#}"));
   139	            Err(Status::internal(msg))
   140	        }
   141	    }
   142	}
   143	
   144	#[cfg(test)]
   145	mod tests {
   146	    use super::*;
   147	
   148	    #[test]
   149	    fn status_to_fault_maps_not_found_to_module_unknown() {
   150	        let fault = status_to_fault(Status::not_found("module 'x' not found"));
   151	        assert_eq!(fault.code, Code::ModuleUnknown);
   152	        assert!(fault.message.contains("not found"));
   153	    }
   154	
   155	    #[test]
   156	    fn status_to_fault_maps_permission_denied_to_protocol_violation() {
   157	        let fault = status_to_fault(Status::permission_denied("path containment: escape"));
   158	        assert_eq!(fault.code, Code::ProtocolViolation);
   159	    }
   160	
   161	    #[test]
   162	    fn status_to_fault_maps_invalid_argument_to_protocol_violation() {
   163	        let fault = status_to_fault(Status::invalid_argument("path not allowed"));
   164	        assert_eq!(fault.code, Code::ProtocolViolation);
   165	    }
   166	}

exec
/usr/bin/zsh -lc "git show --numstat --format='%H%n%s%n%b' e6a0b3b" in /home/michael/dev/Blit
 succeeded in 0ms:
e6a0b3b234d4f175082cb8c7b6a74cfb5940a61c
otp-5b-1: single-stream SOURCE-responder data plane (transport/role decoupling)
The TCP data plane was keyed to ROLE (DESTINATION binds+accepts+receives;
SOURCE dials+sends) — a coincidence that holds only for push. otp-5b splits
the two axes: the RESPONDER binds+accepts and the INITIATOR dials (NAT
reality), while byte direction stays set by role. This slice (5b-1) adds the
pull pair single-stream: a SOURCE responder accepts+sends, a DESTINATION
initiator dials+receives. Resize (the accept-based epoch-N socket) is 5b-2.

blit-core:
- responder_finish binds a data plane for EITHER role when !in_stream_bytes
  (was DESTINATION-only); the bound listener travels in
  Negotiated.responder_data_plane and is consumed by whichever role's driver
  runs. receiver_capacity in the accept stays DESTINATION-only (the byte
  RECEIVER advertises; a DESTINATION initiator advertises in its own
  SessionOpen.receiver_capacity).
- data_plane::accept_source_data_plane: accept the granted epoch-0 socket(s)
  off the bound listener, wrap each via DataPlaneSession::from_stream ->
  DataPlaneSink, and drive the SAME elastic send pipeline dial_source builds.
  Single-stream (resizable=false), so propose_resize returns None and no
  DataPlaneResize flows.
- data_plane::dial_destination_data_plane + InitiatorReceivePlaneRun: dial the
  granted epoch-0 socket(s) and drain each via execute_receive_pipeline (the
  same NeedListSink strictness). DestRecvPlane enum tags the DESTINATION
  receive by connection role (Responder accept / Initiator dial); finish()
  reports the settled stream count either way.
- source_send_half selects accept-vs-dial by whether it holds a bound listener
  or a received grant; destination_session selects dial-vs-accept the mirror
  way. A Resize on the single-stream pull data plane is a protocol violation.
- DestinationSessionConfig gains data_plane_host (the initiator dials the
  responder's host), threaded through drive_destination/destination_session,
  symmetric with SourceSessionConfig.

client: run_pull_session defaults to the TCP data plane (in_stream_bytes from
PullSessionOptions, default false) with data_plane_host = endpoint host;
PullSessionOptions gains the in_stream_bytes fallback knob.

Tests (1519 -> 1521):
- roles suite: pull_data_plane_single_stream_lands_bytes — DESTINATION
  initiator / SOURCE responder over a real loopback TCP data plane; byte-
  identical, data_plane_streams == Some(1), !in_stream_carrier_used.
- e2e: pull_session_lands_bytes_over_the_data_plane (default TCP; the old
  in-stream-only pull test is split so the in-stream path stays covered via
  pull_session_lands_bytes_over_in_stream_carrier). The existing A/B vs old
  pull_sync now runs over the data plane (converge-up bar).
Guard: reverting the SOURCE-responder bind (grant no data plane) makes both
new data-plane tests fall back to in-stream and FAIL !in_stream_carrier_used;
restored, all pass.

Finding: .review/findings/otp-5b-source-responder-data-plane.md.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>


153	0	.review/findings/otp-5b-source-responder-data-plane.md
23	11	crates/blit-core/src/remote/transfer/session_client.rs
204	7	crates/blit-core/src/transfer_session/data_plane.rs
154	69	crates/blit-core/src/transfer_session/mod.rs
82	0	crates/blit-core/tests/transfer_session_roles.rs
49	10	crates/blit-daemon/src/service/transfer_session_e2e.rs

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '340,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   340	}
   341	
   342	#[tonic::async_trait]
   343	impl Blit for BlitService {
   344	    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
   345	    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
   346	    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
   347	    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
   348	    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
   349	    type SubscribeStream =
   350	        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DaemonEvent, Status>> + Send>>;
   351	    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
   352	
   353	    /// ONE_TRANSFER_PATH otp-4a: the daemon serves the unified session
   354	    /// by running `run_destination` as the Responder — the byte
   355	    /// RECEIVER of a client-initiated SOURCE push. Mirrors `push`:
   356	    /// register a jobs row, race the session against cancel/hangup, and
   357	    /// return the response stream immediately (the session runs in the
   358	    /// spawned task, feeding the `ReceiverStream`). Session refusals
   359	    /// travel to the peer as `SessionError` frames; the daemon-specific
   360	    /// module resolution + transport assembly live in `super::transfer`.
   361	    /// Contract: docs/TRANSFER_SESSION.md.
   362	    async fn transfer(
   363	        &self,
   364	        request: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
   365	    ) -> Result<Response<Self::TransferStream>, Status> {
   366	        let peer = peer_addr_string(&request);
   367	        let modules = Arc::clone(&self.modules);
   368	        let default_root = self.default_root.clone();
   369	        let (tx, rx) = mpsc::channel(32);
   370	        let inbound = request.into_inner();
   371	        let metrics = Arc::clone(&self.metrics);
   372	        metrics.inc_push();
   373	        let guard = Arc::clone(&metrics).enter_transfer();
   374	        // Jobs row: registered with an empty endpoint (the module/path
   375	        // arrive in the SessionOpen, mid-handshake inside the session).
   376	        // Populating the row's endpoint from the open is a follow-up —
   377	        // the row still supports CancelJob and appears in GetState, and
   378	        // reuses ActiveJobKind::Push (daemon-receive = push-equivalent)
   379	        // until the kind taxonomy is revisited at cutover.
   380	        let job = self.active_jobs.register(
   381	            ActiveJobKind::Push,
   382	            peer.clone(),
   383	            String::new(),
   384	            String::new(),
   385	        );
   386	        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
   387	        let started = std::time::Instant::now();
   388	        let events_tx = self.events_tx();
   389	
   390	        tokio::spawn(async move {
   391	            let guard = guard;
   392	            let job = job;
   393	            let cancel_token = job.cancellation_token().clone();
   394	            // Session variant: cancel surfaces as a framed
   395	            // SessionError{CANCELLED}, not a bare Status (codex F1).
   396	            let (ok, err_msg) = resolve_transfer_session_outcome(
   397	                super::transfer::run_transfer_session(modules, default_root, inbound, tx.clone()),
   398	                &tx,
   399	                &cancel_token,
   400	                &metrics,
   401	            )
   402	            .await;
   403	            job.record_outcome(ok, err_msg.clone());
   404	            let finished_event = build_transfer_finished_event(&job, ok, err_msg.as_deref());
   405	            drop(job);
   406	            drop(guard);
   407	            let _ = events_tx.send(finished_event);
   408	            metrics.log_completion("transfer", started.elapsed(), ok);
   409	        });
   410	
   411	        Ok(Response::new(ReceiverStream::new(rx)))
   412	    }
   413	
   414	    async fn subscribe(
   415	        &self,
   416	        request: Request<SubscribeRequest>,
   417	    ) -> Result<Response<Self::SubscribeStream>, Status> {
   418	        let req = request.into_inner();
   419	        let transfer_id_filter = req.transfer_id_filter;
   420	        // c-5b: atomically register a broadcast Receiver AND
   421	        // snapshot the per-row event ring (if replay_recent &&
   422	        // filter is non-empty AND the row exists). Both happen
   423	        // under the table lock so no event can be observed
   424	        // both via replay and via broadcast — see emit_event /
   425	        // subscribe_with_ring rustdoc for the full ordering.
   426	        let (mut broadcast_rx, replay) = self.active_jobs.subscribe_with_ring(
   427	            &self.events_tx,
   428	            &transfer_id_filter,
   429	            req.replay_recent,
   430	        );

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '1410,1465p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1410	                .await;
  1411	            (false, Some("cancelled via CancelJob".to_string()))
  1412	        }
  1413	        None => (false, Some("client cancelled".to_string())),
  1414	    }
  1415	}
  1416	
  1417	/// Session variant of [`resolve_streaming_outcome`] for the `Transfer`
  1418	/// RPC: identical hangup / completion / fault handling, but on
  1419	/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
  1420	/// response stream instead of a bare `Status::cancelled` (otp-4a codex
  1421	/// F1). The session speaks `TransferFrame`s, so the client reads the
  1422	/// framed error — and the aborted session future can't send it itself
  1423	/// once the select drops it, so the dispatcher does. A session that
  1424	/// faults on its own already framed the reason; the trailing `Status`
  1425	/// on that branch is belt-and-braces for a pre-frame transport break.
  1426	async fn resolve_transfer_session_outcome<H>(
  1427	    handler: H,
  1428	    tx: &mpsc::Sender<Result<blit_core::generated::TransferFrame, Status>>,
  1429	    cancel_token: &CancellationToken,
  1430	    metrics: &TransferMetrics,
  1431	) -> (bool, Option<String>)
  1432	where
  1433	    H: std::future::Future<Output = Result<(), Status>>,
  1434	{
  1435	    let outcome =
  1436	        resolve_transfer_outcome(handler, tx.closed(), cancel_token.cancelled(), false).await;
  1437	    match outcome {
  1438	        Some(result) => {
  1439	            let (ok, err_msg) = outcome_from_status(&result);
  1440	            if let Err(status) = result {
  1441	                metrics.inc_error();
  1442	                let _ = tx.send(Err(status)).await;
  1443	            }
  1444	            (ok, err_msg)
  1445	        }
  1446	        None if cancel_token.is_cancelled() => {
  1447	            let _ = tx
  1448	                .send(Ok(blit_core::transfer_session::session_error_frame(
  1449	                    blit_core::generated::session_error::Code::Cancelled,
  1450	                    "transfer cancelled via CancelJob",
  1451	                )))
  1452	                .await;
  1453	            (false, Some("cancelled via CancelJob".to_string()))
  1454	        }
  1455	        None => (false, Some("client cancelled".to_string())),
  1456	    }
  1457	}
  1458	
  1459	/// Translate a handler's `Result<_, Status>` into the
  1460	/// `(ok, error_message)` pair the ActiveJobs guard expects.
  1461	/// Used inside [`resolve_streaming_outcome`] for the `push` /
  1462	/// `pull_sync` dispatchers. `delegated_pull` has its own shape
  1463	/// (handler returns `bool` inside a select) and inlines the
  1464	/// equivalent mapping there.
  1465	fn outcome_from_status<T>(result: &Result<T, Status>) -> (bool, Option<String>) {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/endpoint.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::path::{Path, PathBuf};
     2	
     3	use eyre::{bail, eyre, Result};
     4	
     5	/// Canonical remote path variants.
     6	#[derive(Debug, Clone)]
     7	pub enum RemotePath {
     8	    /// Addressed via `server:/module/...`
     9	    Module { module: String, rel_path: PathBuf },
    10	    /// Addressed via `server://...` (default root/export)
    11	    Root { rel_path: PathBuf },
    12	    /// Discovery form (`server` or `server:port`)
    13	    Discovery,
    14	}
    15	
    16	/// Parsed representation of a canonical remote endpoint.
    17	#[derive(Debug, Clone)]
    18	pub struct RemoteEndpoint {
    19	    pub host: String,
    20	    pub port: u16,
    21	    pub path: RemotePath,
    22	}
    23	
    24	impl RemoteEndpoint {
    25	    /// The one statement of blit's default daemon port (w7-6): the
    26	    /// daemon's bind default, `blit scan`'s display elision, and the
    27	    /// TUI's local-row fallback all reference this constant.
    28	    pub const DEFAULT_PORT: u16 = 9031;
    29	
    30	    pub fn parse(raw: &str) -> Result<Self> {
    31	        let trimmed = raw.trim();
    32	        if trimmed.is_empty() {
    33	            bail!("remote location cannot be empty");
    34	        }
    35	
    36	        match check_local_path(trimmed) {
    37	            LocalPathCheck::IsLocal => {
    38	                bail!("input appears to be a local path");
    39	            }
    40	            LocalPathCheck::RemoteWithBackslashes => {
    41	                bail!(
    42	                    "remote paths must use forward slashes, not backslashes.\n\
    43	                     Example: server:/module/path or server://path\n\
    44	                     Got: {}",
    45	                    trimmed
    46	                );
    47	            }
    48	            LocalPathCheck::NotLocal => {}
    49	        }
    50	
    51	        if let Some(idx) = trimmed.find("://") {
    52	            // Root export (server://path)
    53	            let host_port = &trimmed[..idx];
    54	            let remainder = &trimmed[idx + 3..];
    55	            let (host, port) = parse_host_port(host_port)?;
    56	            let rel = normalize_relative_path_buf(remainder);
    57	            return Ok(Self {
    58	                host,
    59	                port,
    60	                path: RemotePath::Root { rel_path: rel },
    61	            });
    62	        }
    63	
    64	        if let Some(idx) = trimmed.find(":/") {
    65	            // Module export (server:/module/...)
    66	            let host_port = &trimmed[..idx];
    67	            let remainder = &trimmed[idx + 2..];
    68	            let (host, port) = parse_host_port(host_port)?;
    69	
    70	            let slash_idx = remainder.find('/').ok_or_else(|| {
    71	                eyre!(
    72	                    "module path must end with '/' (e.g., server:/module/ or server:/module/path)"
    73	                )
    74	            })?;
    75	
    76	            let module = &remainder[..slash_idx];
    77	            if module.is_empty() {
    78	                bail!("module name cannot be empty; expected server:/module/...");
    79	            }
    80	            let rest = &remainder[slash_idx + 1..];
    81	            let rel = normalize_relative_path_buf(rest);
    82	
    83	            return Ok(Self {
    84	                host,
    85	                port,
    86	                path: RemotePath::Module {
    87	                    module: module.to_string(),
    88	                    rel_path: rel,
    89	                },
    90	            });
    91	        }
    92	
    93	        // Discovery (server or server:port)
    94	        let (host, port) = parse_host_port(trimmed)?;
    95	        Ok(Self {
    96	            host,
    97	            port,
    98	            path: RemotePath::Discovery,
    99	        })
   100	    }
   101	
   102	    pub fn control_plane_uri(&self) -> String {
   103	        // R58-F10: IPv6 literals must be bracketed in the URI's
   104	        // authority component. The host field is stored bracket-
   105	        // less (the parser strips them), so we re-bracket here.
   106	        // A colon-containing host can only be IPv6 in our schema —
   107	        // hostnames and IPv4 addresses never contain colons. Bare
   108	        // `2001:db8::1:9031` is parsed by HTTP libraries as host
   109	        // `2001` with garbage trailing, which is the bug.
   110	        if self.host.contains(':') {
   111	            format!("http://[{}]:{}", self.host, self.port)
   112	        } else {
   113	            format!("http://{}:{}", self.host, self.port)
   114	        }
   115	    }
   116	
   117	    /// Render just the `host[:port]` portion of this endpoint
   118	    /// — drops the module / path suffix. Used by user-facing
   119	    /// hints that reference the host as an argument
   120	    /// (e.g. `blit jobs cancel <host> <id>`), where stitching
   121	    /// the full canonical path back together would be wrong.
   122	    /// Handles bracketed IPv6 via [`display_host`]; preserves
   123	    /// non-default ports.
   124	    pub fn host_port_display(&self) -> String {
   125	        let host = display_host(&self.host);
   126	        if self.port == Self::DEFAULT_PORT {
   127	            host
   128	        } else {
   129	            format!("{}:{}", host, self.port)
   130	        }
   131	    }
   132	
   133	    /// Display helper used by CLI for canonical formatting.
   134	    pub fn display(&self) -> String {
   135	        let host = display_host(&self.host);
   136	        let host_port = if self.port == Self::DEFAULT_PORT {
   137	            host
   138	        } else {
   139	            format!("{}:{}", host, self.port)
   140	        };
   141	
   142	        match &self.path {
   143	            RemotePath::Module { module, rel_path } => {
   144	                let mut path = format!("{}:/{}", host_port, module);
   145	                if rel_path.as_os_str().is_empty() {
   146	                    path.push('/');
   147	                } else {
   148	                    path.push('/');
   149	                    path.push_str(&rel_path_to_string(rel_path));
   150	                }
   151	                path
   152	            }
   153	            RemotePath::Root { rel_path } => {
   154	                if rel_path.as_os_str().is_empty() {
   155	                    format!("{host_port}://")
   156	                } else {
   157	                    format!("{}://{}", host_port, rel_path_to_string(rel_path))
   158	                }
   159	            }
   160	            RemotePath::Discovery => host_port,
   161	        }
   162	    }
   163	}
   164	
   165	fn parse_host_port(authority: &str) -> Result<(String, u16)> {
   166	    if authority.is_empty() {
   167	        bail!("remote location missing host");
   168	    }
   169	
   170	    if let Some(stripped) = authority.strip_prefix('[') {
   171	        // IPv6 literal
   172	        let closing = stripped
   173	            .find(']')
   174	            .ok_or_else(|| eyre!("unterminated IPv6 address: {}", authority))?;
   175	        let host = &stripped[..closing];
   176	        let remainder = &stripped[closing + 1..];
   177	        let port = if let Some(port_str) = remainder.strip_prefix(':') {
   178	            parse_port(port_str)?
   179	        } else if remainder.is_empty() {
   180	            RemoteEndpoint::DEFAULT_PORT
   181	        } else {
   182	            bail!("invalid host specification: {}", authority);
   183	        };
   184	        return Ok((host.to_string(), port));
   185	    }
   186	
   187	    if let Some((host, port)) = authority.rsplit_once(':') {
   188	        if host.is_empty() {
   189	            bail!("remote location missing host before ':'");
   190	        }
   191	        Ok((host.to_string(), parse_port(port)?))
   192	    } else {
   193	        Ok((authority.to_string(), RemoteEndpoint::DEFAULT_PORT))
   194	    }
   195	}
   196	
   197	fn parse_port(raw: &str) -> Result<u16> {
   198	    if raw.is_empty() {
   199	        return Ok(RemoteEndpoint::DEFAULT_PORT);
   200	    }
   201	    raw.parse::<u16>()
   202	        .map_err(|_| eyre!("invalid port '{}'", raw))
   203	}
   204	
   205	fn normalize_relative_path_buf(raw: &str) -> PathBuf {
   206	    if raw.is_empty() {
   207	        PathBuf::new()
   208	    } else {
   209	        let trimmed = raw.trim_start_matches('/');
   210	        if trimmed.is_empty() {
   211	            PathBuf::new()
   212	        } else {
   213	            PathBuf::from(trimmed)
   214	        }
   215	    }
   216	}
   217	
   218	fn rel_path_to_string(path: &Path) -> String {
   219	    // Delegate to the canonical POSIX-form helper.
   220	    crate::path_posix::relative_path_to_posix(path)
   221	}
   222	
   223	fn display_host(host: &str) -> String {
   224	    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
   225	        format!("[{}]", host)
   226	    } else {
   227	        host.to_string()
   228	    }
   229	}
   230	
   231	/// Result of checking if input looks like a local path
   232	enum LocalPathCheck {
   233	    /// Definitely a local path
   234	    IsLocal,
   235	    /// Looks like a remote path with backslashes - user error
   236	    RemoteWithBackslashes,
   237	    /// Not a local path
   238	    NotLocal,
   239	}
   240	
   241	fn check_local_path(input: &str) -> LocalPathCheck {
   242	    if input.is_empty() {
   243	        return LocalPathCheck::NotLocal;
   244	    }
   245	
   246	    let first = input.chars().next().expect("non-empty input checked above");
   247	    if matches!(first, '.' | '/' | '\\' | '~') {
   248	        return LocalPathCheck::IsLocal;
   249	    }
   250	
   251	    // UNC paths: \\server\share or //server/share
   252	    if input.starts_with("//") || input.starts_with("\\\\") {
   253	        return LocalPathCheck::IsLocal;
   254	    }
   255	
   256	    // Windows drive letter: C:\path or C:/path - check BEFORE backslash handling
   257	    if input.len() >= 3 {
   258	        let mut chars = input.chars();
   259	        let drive = chars.next().expect("len >= 3 checked above");
   260	        if drive.is_ascii_alphabetic() {

exec
/usr/bin/zsh -lc "rg -n \"struct RemoteEndpoint|host:|control_plane_uri|localhost|127\\.0\\.0\\.1|endpoint.host|RemoteEndpoint\" crates/blit-core/src/remote crates/blit-daemon/src/service/transfer_session_e2e.rs -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer_session_e2e.rs:47:use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
crates/blit-daemon/src/service/transfer_session_e2e.rs:61:    endpoint: RemoteEndpoint,
crates/blit-daemon/src/service/transfer_session_e2e.rs:87:        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
crates/blit-daemon/src/service/transfer_session_e2e.rs:104:        let endpoint = RemoteEndpoint {
crates/blit-daemon/src/service/transfer_session_e2e.rs:105:            host: "127.0.0.1".into(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:123:    fn endpoint_for_missing_module(&self) -> RemoteEndpoint {
crates/blit-daemon/src/service/transfer_session_e2e.rs:124:        RemoteEndpoint {
crates/blit-daemon/src/service/transfer_session_e2e.rs:125:            host: self.endpoint.host.clone(),
crates/blit-core/src/remote/mod.rs:9:pub use endpoint::{RemoteEndpoint, RemotePath};
crates/blit-core/src/remote/pull.rs:18:use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
crates/blit-core/src/remote/pull.rs:172:    endpoint: RemoteEndpoint,
crates/blit-core/src/remote/pull.rs:177:    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
crates/blit-core/src/remote/pull.rs:178:        let uri = endpoint.control_plane_uri();
crates/blit-core/src/remote/pull.rs:251:        endpoint: &RemoteEndpoint,
crates/blit-core/src/remote/pull.rs:310:        let host = self.endpoint.host.clone();
crates/blit-core/src/remote/pull.rs:432:        endpoint: &RemoteEndpoint,
crates/blit-core/src/remote/pull.rs:542:    /// handle (host:port for the gRPC connection); the spec is
crates/blit-core/src/remote/pull.rs:1673:    host: String,
crates/blit-core/src/remote/pull.rs:1738:        host: String,
crates/blit-core/src/remote/pull.rs:1879:async fn connect_pull_stream(host: &str, port: u32, handshake: &[u8]) -> Result<TcpStream> {
crates/blit-core/src/remote/pull.rs:2062:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
crates/blit-core/src/remote/pull.rs:2094:                "127.0.0.1".to_string(),
crates/blit-core/src/remote/pull.rs:2120:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
crates/blit-core/src/remote/pull.rs:2132:        let stream = super::connect_pull_stream("127.0.0.1", port, TOKEN)
crates/blit-core/src/remote/pull.rs:2175:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
crates/blit-core/src/remote/pull.rs:2203:            "127.0.0.1".to_string(),
crates/blit-core/src/remote/pull.rs:2253:                "127.0.0.1".to_string(),
crates/blit-core/src/remote/pull.rs:2302:    fn endpoint_with_path(path: RemotePath) -> RemoteEndpoint {
crates/blit-core/src/remote/pull.rs:2303:        RemoteEndpoint {
crates/blit-core/src/remote/pull.rs:2304:            host: "localhost".to_string(),
crates/blit-core/src/remote/pull.rs:2310:    fn module_endpoint(module: &str, rel: &str) -> RemoteEndpoint {
crates/blit-core/src/remote/endpoint.rs:18:pub struct RemoteEndpoint {
crates/blit-core/src/remote/endpoint.rs:19:    pub host: String,
crates/blit-core/src/remote/endpoint.rs:24:impl RemoteEndpoint {
crates/blit-core/src/remote/endpoint.rs:102:    pub fn control_plane_uri(&self) -> String {
crates/blit-core/src/remote/endpoint.rs:180:            RemoteEndpoint::DEFAULT_PORT
crates/blit-core/src/remote/endpoint.rs:193:        Ok((authority.to_string(), RemoteEndpoint::DEFAULT_PORT))
crates/blit-core/src/remote/endpoint.rs:199:        return Ok(RemoteEndpoint::DEFAULT_PORT);
crates/blit-core/src/remote/endpoint.rs:223:fn display_host(host: &str) -> String {
crates/blit-core/src/remote/endpoint.rs:272:        // Pattern like "host:\path" or "host:\\path" suggests user meant remote
crates/blit-core/src/remote/endpoint.rs:303:        let ep = RemoteEndpoint::parse("example.com:/media/").unwrap();
crates/blit-core/src/remote/endpoint.rs:305:        assert_eq!(ep.port, RemoteEndpoint::DEFAULT_PORT);
crates/blit-core/src/remote/endpoint.rs:320:        let ep = RemoteEndpoint::parse("example.com:9000:/data/projects/foo").unwrap();
crates/blit-core/src/remote/endpoint.rs:336:        let ep = RemoteEndpoint::parse("example.com://backups").unwrap();
crates/blit-core/src/remote/endpoint.rs:347:        let ep = RemoteEndpoint::parse("example.com").unwrap();
crates/blit-core/src/remote/endpoint.rs:353:        let ep = RemoteEndpoint::parse("example.com:9130").unwrap();
crates/blit-core/src/remote/endpoint.rs:360:        let ep = RemoteEndpoint::parse("[2001:db8::1]:/share/").unwrap();
crates/blit-core/src/remote/endpoint.rs:376:        assert!(RemoteEndpoint::parse("example.com:/module").is_err());
crates/blit-core/src/remote/endpoint.rs:381:        let result = RemoteEndpoint::parse(r"server:\module\path");
crates/blit-core/src/remote/endpoint.rs:393:        let result = RemoteEndpoint::parse(r"server:\\");
crates/blit-core/src/remote/endpoint.rs:405:        let ep = RemoteEndpoint::parse("server:/m/path").expect("parse");
crates/blit-core/src/remote/endpoint.rs:411:        let ep = RemoteEndpoint::parse("server:9444:/m/path").expect("parse");
crates/blit-core/src/remote/endpoint.rs:417:        let ep = RemoteEndpoint::parse("[::1]:9444:/m/path").expect("parse");
crates/blit-core/src/remote/push/client/mod.rs:17:use crate::remote::endpoint::RemoteEndpoint;
crates/blit-core/src/remote/push/client/mod.rs:143:    host: String,
crates/blit-core/src/remote/push/client/mod.rs:157:        host: &str,
crates/blit-core/src/remote/push/client/mod.rs:258:                    host: host.to_string(),
crates/blit-core/src/remote/push/client/mod.rs:597:    endpoint: RemoteEndpoint,
crates/blit-core/src/remote/push/client/mod.rs:602:    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
crates/blit-core/src/remote/push/client/mod.rs:603:        let uri = endpoint.control_plane_uri();
crates/blit-core/src/remote/push/client/mod.rs:1020:                                                &self.endpoint.host,
crates/blit-core/src/remote/push/client/helpers.rs:310:    endpoint: &crate::remote::endpoint::RemoteEndpoint,
crates/blit-core/src/remote/transfer/session_client.rs:28:use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
crates/blit-core/src/remote/transfer/session_client.rs:69:    endpoint: &RemoteEndpoint,
crates/blit-core/src/remote/transfer/session_client.rs:110:        data_plane_host: Some(endpoint.host.clone()),
crates/blit-core/src/remote/transfer/session_client.rs:156:    endpoint: &RemoteEndpoint,
crates/blit-core/src/remote/transfer/session_client.rs:191:        data_plane_host: Some(endpoint.host.clone()),
crates/blit-core/src/remote/transfer/session_client.rs:199:fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
crates/blit-core/src/remote/transfer/session_client.rs:215:async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
crates/blit-core/src/remote/transfer/session_client.rs:216:    let uri = endpoint.control_plane_uri();
crates/blit-core/src/remote/transfer/socket.rs:187:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
crates/blit-core/src/remote/transfer/socket.rs:269:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
crates/blit-core/src/remote/transfer/socket.rs:304:        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
crates/blit-core/src/remote/transfer/pipeline.rs:1002:            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
crates/blit-core/src/remote/transfer/data_plane.rs:101:        host: &str,
crates/blit-core/src/remote/transfer/data_plane.rs:131:        host: &str,
crates/blit-core/src/remote/transfer/data_plane.rs:890:        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
crates/blit-core/src/remote/transfer/tcp_info.rs:73:        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer_session_e2e.rs | sed -n '1,120p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
     2	//! unified `Transfer` session and a real client initiates it as SOURCE
     3	//! over gRPC. otp-4b makes the default carrier the **TCP data plane**
     4	//! (the responder grants it in `SessionAccept`, the client dials +
     5	//! authenticates + sends payloads over sockets); the in-stream carrier
     6	//! stays live as the requested fallback. These tests pin the
     7	//! push-equivalent behavior over both carriers:
     8	//!
     9	//! - a session lands bytes byte-identically and scores them correctly,
    10	//!   over the data plane and over the in-stream fallback;
    11	//! - **A/B parity**: the same fixture through OLD push and the NEW
    12	//!   session (data plane) yields byte-identical destination trees +
    13	//!   equal shared summary counters (the converge-up bar);
    14	//! - responder refusals (read-only module, unknown module) arrive as
    15	//!   `SessionError` frames, surfaced to the client as faults;
    16	//! - the unified SizeMtime semantic: a same-size destination file that
    17	//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
    18	//!   converged behavior — see the finding doc's compare decision).
    19	//!
    20	//! otp-5a/5b add the pull-equivalent (roles flipped): the client initiates
    21	//! as DESTINATION and the daemon streams its module tree as the SOURCE
    22	//! Responder. otp-5b makes the default carrier the TCP data plane too — the
    23	//! daemon (SOURCE responder) binds+grants+accepts sockets while sending and
    24	//! the client (DESTINATION initiator) dials + receives — with the in-stream
    25	//! carrier as the requested fallback. Those tests pin a byte-identical
    26	//! landing over both carriers + A/B parity vs old `pull_sync`, proving the
    27	//! one served RPC handles both directions by the declared role, not a
    28	//! second code path.
    29	//!
    30	//! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
    31	//! `BlitService` on loopback + a real client. Only in-crate tests can
    32	//! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
    33	//! blit-daemon.
    34	
    35	use std::collections::{BTreeMap, HashMap};
    36	use std::path::{Path, PathBuf};
    37	use std::sync::Arc;
    38	
    39	use blit_core::fs_enum::FileFilter;
    40	use blit_core::generated::blit_server::BlitServer;
    41	use blit_core::generated::{session_error, MirrorMode};
    42	use blit_core::remote::pull::PullSyncOptions;
    43	use blit_core::remote::transfer::session_client::{
    44	    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
    45	};
    46	use blit_core::remote::transfer::source::FsTransferSource;
    47	use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
    48	use blit_core::transfer_session::SessionFault;
    49	use tokio::sync::oneshot;
    50	
    51	use crate::runtime::ModuleConfig;
    52	use crate::service::BlitService;
    53	
    54	// ---------------------------------------------------------------------------
    55	// Harness
    56	// ---------------------------------------------------------------------------
    57	
    58	/// A running in-process daemon exposing module "test" over a writable
    59	/// (or read-only) temp dir, and the loopback endpoint targeting it.
    60	struct Daemon {
    61	    endpoint: RemoteEndpoint,
    62	    shutdown: Option<oneshot::Sender<()>>,
    63	    server: Option<tokio::task::JoinHandle<()>>,
    64	    _dest: tempfile::TempDir,
    65	    dest_root: PathBuf,
    66	    active_jobs: crate::active_jobs::ActiveJobs,
    67	}
    68	
    69	impl Daemon {
    70	    async fn start(read_only: bool) -> Self {
    71	        let dest = tempfile::tempdir().expect("dest dir");
    72	        let canonical = dest.path().canonicalize().expect("canonical dest");
    73	        let mut modules = HashMap::new();
    74	        modules.insert(
    75	            "test".to_string(),
    76	            ModuleConfig {
    77	                name: "test".into(),
    78	                path: canonical.clone(),
    79	                canonical_root: canonical.clone(),
    80	                read_only,
    81	                _comment: None,
    82	                delegation_allowed: true,
    83	            },
    84	        );
    85	        let service = BlitService::with_modules(modules, false);
    86	        let active_jobs = service.active_jobs.clone();
    87	        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
    88	            .await
    89	            .expect("bind loopback listener");
    90	        let port = listener.local_addr().expect("listener addr").port();
    91	        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    92	        let server = tokio::spawn(async move {
    93	            blit_core::remote::grpc_server::production_server_builder()
    94	                .add_service(BlitServer::new(service))
    95	                .serve_with_incoming_shutdown(
    96	                    tokio_stream::wrappers::TcpListenerStream::new(listener),
    97	                    async {
    98	                        let _ = shutdown_rx.await;
    99	                    },
   100	                )
   101	                .await
   102	                .expect("in-process daemon serves");
   103	        });
   104	        let endpoint = RemoteEndpoint {
   105	            host: "127.0.0.1".into(),
   106	            port,
   107	            path: RemotePath::Module {
   108	                module: "test".into(),
   109	                rel_path: PathBuf::new(),
   110	            },
   111	        };
   112	        Daemon {
   113	            endpoint,
   114	            shutdown: Some(shutdown_tx),
   115	            server: Some(server),
   116	            _dest: dest,
   117	            dest_root: canonical,
   118	            active_jobs,
   119	        }
   120	    }

exec
/usr/bin/zsh -lc 'git diff --check e6a0b3b''^ e6a0b3b' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'git diff --name-only e6a0b3b''^ e6a0b3b' in /home/michael/dev/Blit
 succeeded in 0ms:
.review/findings/otp-5b-source-responder-data-plane.md
crates/blit-core/src/remote/transfer/session_client.rs
crates/blit-core/src/transfer_session/data_plane.rs
crates/blit-core/src/transfer_session/mod.rs
crates/blit-core/tests/transfer_session_roles.rs
crates/blit-daemon/src/service/transfer_session_e2e.rs

exec
/usr/bin/zsh -lc 'git diff --unified=80 e6a0b3b''^ e6a0b3b -- crates/blit-core/src/transfer_session/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 0ecb287..21ca670 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -53,160 +53,168 @@ pub const CONTRACT_VERSION: u32 = 1;
 
 /// Payload chunk size on the in-stream carrier. Same unit the gRPC
 /// control plane uses today; the data plane (otp-4) has its own.
 const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
 
 /// Manifest entries buffered per destination diff batch. Mirrors the
 /// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
 /// per-entry check is 2+ blocking syscalls, so it runs chunked on the
 /// blocking pool instead of inline per entry.
 const DEST_DIFF_CHUNK: usize = 128;
 
 /// Buffer of the in-memory pipe that feeds wire file-record bytes
 /// into `FsTransferSink::write_file_stream`. Bounds destination-side
 /// buffering per file record.
 const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
 
 /// This build's session identity: `<crate version>+<git sha>[.dirty]`
 /// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
 /// "unknown" when git was unavailable at compile time.
 pub fn session_build_id() -> &'static str {
     concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
 }
 
 /// The identity this end presents in `SessionHello`. Defaults to the
 /// real compile-time identity; tests inject mismatches.
 #[derive(Debug, Clone)]
 pub struct HelloConfig {
     pub build_id: String,
     pub contract_version: u32,
 }
 
 impl Default for HelloConfig {
     fn default() -> Self {
         Self {
             build_id: session_build_id().to_string(),
             contract_version: CONTRACT_VERSION,
         }
     }
 }
 
 /// Which handshake part this end plays. Orthogonal to role: all four
 /// initiator/role combinations run the same state machine (contract
 /// §Invariants 3).
 pub enum SessionEndpoint {
     /// This end opened the transport; it sends `SessionOpen`.
     /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
     Initiator { open: Box<SessionOpen> },
     /// This end answers `SessionOpen` with `SessionAccept`. Daemon
     /// module/path/read-only validation attaches here at otp-4.
     Responder,
 }
 
 impl SessionEndpoint {
     /// Convenience constructor so callers don't spell the `Box`.
     pub fn initiator(open: SessionOpen) -> Self {
         SessionEndpoint::Initiator {
             open: Box::new(open),
         }
     }
 }
 
 pub struct SourceSessionConfig {
     pub hello: HelloConfig,
     pub endpoint: SessionEndpoint,
     /// Engine planner knobs (tar/large/raw thresholds). Local to the
     /// source end — strategy selection is planner-owned and never
     /// crosses the wire (contract §Transport selection).
     pub plan_options: PlanOptions,
     /// Host to dial the granted TCP data plane on (otp-4b). The
     /// initiator connected the control plane to this host; the data
     /// plane rides the same host on the granted port (contract
     /// §Transport: the initiator always dials). `None` disables the
     /// data plane at this end — a grant then faults, since the responder
     /// is waiting to accept sockets that would never arrive.
     pub data_plane_host: Option<String>,
 }
 
 pub struct DestinationSessionConfig {
     pub hello: HelloConfig,
     pub endpoint: SessionEndpoint,
+    /// Host to dial the granted TCP data plane on when this end is the
+    /// **initiator** (pull-equivalent, otp-5b): the DESTINATION initiator
+    /// dials the SOURCE responder's granted sockets on the same host it
+    /// reached the control plane on (contract §Transport: the initiator
+    /// always dials). `None` — or a DESTINATION responder, which binds
+    /// rather than dials — falls back to the in-stream carrier. Symmetric
+    /// with [`SourceSessionConfig::data_plane_host`].
+    pub data_plane_host: Option<String>,
 }
 
 /// A session-terminating fault: either end refusing, aborting, or
 /// catching the peer in a protocol violation. Carried as the error
 /// payload of the drivers' `eyre::Report`s — downcast to inspect the
 /// wire code.
 #[derive(Debug, Clone)]
 pub struct SessionFault {
     pub code: session_error::Code,
     pub message: String,
     /// Both build ids on BUILD_MISMATCH so the operator sees exactly
     /// which end is stale (contract §Errors).
     pub local_build_id: String,
     pub peer_build_id: String,
     /// True when the peer already knows about this fault — it sent
     /// the `SessionError` frame itself, or this end already emitted
     /// one. Drivers must not send another.
     pub peer_notified: bool,
 }
 
 impl SessionFault {
     fn new(code: session_error::Code, message: impl Into<String>) -> Self {
         Self {
             code,
             message: message.into(),
             local_build_id: String::new(),
             peer_build_id: String::new(),
             peer_notified: false,
         }
     }
 
     fn protocol_violation(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::ProtocolViolation, message)
     }
 
     fn internal(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::Internal, message)
     }
 
     fn read_only(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::ReadOnly, message)
     }
 
     /// Public constructor for a caller-side refusal (e.g. the daemon's
     /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
     /// code). blit-core stays free of `tonic::Status`, so the caller
     /// picks the wire code.
     pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
         Self::new(code, message)
     }
 
     fn from_wire(err: SessionError) -> Self {
         Self {
             code: session_error::Code::try_from(err.code)
                 .unwrap_or(session_error::Code::SessionErrorUnspecified),
             message: err.message,
             // The peer reports its view: its "local" is our peer.
             local_build_id: err.peer_build_id,
             peer_build_id: err.local_build_id,
             peer_notified: true,
         }
     }
 
     fn to_wire(&self) -> SessionError {
         SessionError {
             code: self.code as i32,
             message: self.message.clone(),
             local_build_id: self.local_build_id.clone(),
             peer_build_id: self.peer_build_id.clone(),
         }
     }
 }
 
 impl fmt::Display for SessionFault {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "session {}: {}", self.code.as_str_name(), self.message)
     }
 }
 
 impl std::error::Error for SessionFault {}
@@ -425,170 +433,172 @@ async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) ->
         }
     };
 
     if peer_hello.build_id != hello.build_id
         || peer_hello.contract_version != hello.contract_version
     {
         let fault = SessionFault {
             code: session_error::Code::BuildMismatch,
             message: format!(
                 "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
                 hello.build_id, hello.contract_version,
                 peer_hello.build_id, peer_hello.contract_version,
             ),
             local_build_id: hello.build_id.clone(),
             peer_build_id: peer_hello.build_id.clone(),
             peer_notified: false,
         };
         return Err(notify_and_wrap(transport, fault).await);
     }
     Ok(())
 }
 
 /// The responder half of establish AFTER the `SessionOpen` is read:
 /// complement check, `validate_open`, endpoint resolution, data-plane
 /// prepare, and `SessionAccept`. Factored out so both `establish` (which
 /// reads the open then calls this) and `run_responder` (which reads the
 /// open, dispatches on the declared role, then calls this with the
 /// resolved local role) share one implementation. Sends the refusal
 /// `SessionError` itself; returned faults are `peer_notified`.
 async fn responder_finish(
     transport: &mut FrameTransport,
     open: SessionOpen,
     local_role: TransferRole,
     validate_open: &OpenValidator,
     resolve_open: Option<&OpenResolver>,
 ) -> Result<Negotiated> {
     // The initiator declares ITS role; this responder end must
     // hold the complement.
     let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
     if declared != complement(local_role) {
         return Err(notify_and_wrap(
             transport,
             SessionFault::protocol_violation(format!(
                 "initiator declared role {} but this responder is {}",
                 declared.as_str_name(),
                 local_role.as_str_name()
             )),
         )
         .await);
     }
     if let Err(fault) = validate_open(&open) {
         // Refusal is a SessionError instead of SessionAccept,
         // never a silent close (contract §Phase state machine).
         return Err(notify_and_wrap(transport, fault).await);
     }
     // Responder endpoint resolution (otp-4): map the wire
     // module/path to a local root and enforce read-only, both
     // BEFORE SessionAccept so a refusal replaces the accept
     // (never follows it). The resolver is caller-supplied
     // (daemon module lookup); a fixed-root responder passes
     // None and resolves nothing here.
     let resolved_root = match resolve_open {
         Some(resolve) => match resolve(&open).await {
             Ok(resolved) => {
                 // A read-only module is fatal only for a
                 // DESTINATION (it would write); a SOURCE
                 // responder (otp-5, daemon-send) reads happily.
                 if local_role == TransferRole::Destination && resolved.read_only {
                     return Err(notify_and_wrap(
                         transport,
                         SessionFault::read_only("destination module is read-only".to_string()),
                     )
                     .await);
                 }
                 Some(resolved.root)
             }
             Err(fault) => return Err(notify_and_wrap(transport, fault).await),
         },
         None => None,
     };
-    // Data plane (otp-4b): a DESTINATION responder binds a TCP
-    // listener and grants it, unless the initiator requested the
-    // in-stream carrier or the bind fails (grant-less accept ⇒
-    // in-stream fallback). A SOURCE responder (otp-5, daemon-send)
-    // grants no data plane in otp-5a — the transport/role decoupling
-    // that lets a SOURCE responder bind+grant lands at otp-5b.
-    let responder_data_plane = if local_role == TransferRole::Destination && !open.in_stream_bytes {
-        data_plane::prepare_responder_data_plane().await
-    } else {
+    // Data plane (otp-4b/5b): a responder binds a TCP listener and grants
+    // it, unless the initiator requested the in-stream carrier or the bind
+    // fails (grant-less accept ⇒ in-stream fallback). This is role-agnostic
+    // (otp-5b): the RESPONDER binds+accepts and the INITIATOR dials, while
+    // byte direction is set by role — a DESTINATION responder accepts+
+    // receives (push, otp-4b), a SOURCE responder accepts+sends (pull,
+    // otp-5b). The bound listener travels in `Negotiated.responder_data_plane`
+    // and is consumed by whichever role's driver runs.
+    let responder_data_plane = if open.in_stream_bytes {
         None
+    } else {
+        data_plane::prepare_responder_data_plane().await
     };
     let accept = SessionAccept {
         // The byte RECEIVER advertises capacity at session
         // open (D-2026-06-20-1/-2); consumed by the dial when
         // the data plane lands (otp-4b).
         receiver_capacity: if local_role == TransferRole::Destination {
             Some(crate::engine::local_receiver_capacity())
         } else {
             None
         },
         // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
         data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
     };
     transport.send(frame(Frame::Accept(accept.clone()))).await?;
     Ok(Negotiated {
         open,
         accept,
         resolved_root,
         responder_data_plane,
     })
 }
 
 /// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
 /// scoping requirement). Sends the refusal `SessionError` itself when
 /// it detects the fault locally; returned faults are `peer_notified`.
 async fn establish(
     transport: &mut FrameTransport,
     hello: &HelloConfig,
     endpoint: &SessionEndpoint,
     local_role: TransferRole,
     validate_open: &OpenValidator,
     // Consulted only on the Responder branch, after the received open
     // passes `validate_open` and before SessionAccept. `None` = the
     // caller supplies the root itself (Initiator, or fixed-root test).
     resolve_open: Option<&OpenResolver>,
 ) -> Result<Negotiated> {
     exchange_hello(transport, hello).await?;
 
     match endpoint {
         SessionEndpoint::Initiator { open } => {
             let open = open.as_ref().clone();
             transport.send(frame(Frame::Open(open.clone()))).await?;
             let accept = match expect_frame(transport).await? {
                 Frame::Accept(a) => a,
                 other => {
                     return Err(notify_and_wrap(
                         transport,
                         SessionFault::protocol_violation(format!(
                             "expected SessionAccept, got {}",
                             frame_name(&Some(other))
                         )),
                     )
                     .await)
                 }
             };
             Ok(Negotiated {
                 open,
                 accept,
                 resolved_root: None,
                 responder_data_plane: None,
             })
         }
         SessionEndpoint::Responder => {
             let open = match expect_frame(transport).await? {
                 Frame::Open(o) => o,
                 other => {
                     return Err(notify_and_wrap(
                         transport,
                         SessionFault::protocol_violation(format!(
                             "expected SessionOpen, got {}",
                             frame_name(&Some(other))
                         )),
                     )
                     .await)
                 }
             };
             responder_finish(transport, open, local_role, validate_open, resolve_open).await
         }
     }
 }
@@ -598,355 +608,379 @@ async fn establish(
 async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
     match transport.recv().await? {
         Some(TransferFrame {
             frame: Some(Frame::Error(err)),
         }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
         Some(TransferFrame { frame: Some(f) }) => Ok(f),
         Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
             SessionFault::protocol_violation("frame with empty oneof"),
         )),
         None => Err(eyre::Report::new(SessionFault::internal(
             "peer closed during session establish",
         ))),
     }
 }
 
 /// Send the fault to the peer (best effort), mark it notified, and
 /// wrap it for return.
 async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
     let _ = transport.send(error_frame(&fault)).await;
     fault.peer_notified = true;
     eyre::Report::new(fault)
 }
 
 // ---------------------------------------------------------------------------
 // SOURCE driver
 // ---------------------------------------------------------------------------
 
 /// Events the source's receive half forwards to its send half. The
 /// channel is unbounded but bounded by construction: every `Need`
 /// consumes a distinct sent-manifest entry (unknown or repeated paths
 /// fault the session), so the queue never exceeds the source's own
 /// manifest size — the contract's bounded-buffering rule holds.
 enum SourceEvent {
     Need(FileHeader),
     NeedComplete,
     /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
     /// send half dials the epoch-N socket on `accepted`.
     ResizeAck(DataPlaneResizeAck),
     Summary(TransferSummary),
     Fault(SessionFault),
 }
 
 /// Run the SOURCE role of one transfer session over `transport`.
 /// Returns the destination-computed `TransferSummary` (contract: the
 /// end that wrote the bytes is the end that attests to them).
 pub async fn run_source(
     cfg: SourceSessionConfig,
     transport: FrameTransport,
     source: Arc<dyn TransferSource>,
 ) -> Result<TransferSummary> {
     let mut transport = transport;
     if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
         // Own-config coherence: a source initiator declares SOURCE.
         let declared = TransferRole::try_from(open.initiator_role);
         if declared != Ok(TransferRole::Source) {
             eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
         }
         if let Err(fault) = source_open_validator(open) {
             eyre::bail!("run_source initiator config unsupported: {fault}");
         }
     }
 
     let negotiated = establish(
         &mut transport,
         &cfg.hello,
         &cfg.endpoint,
         TransferRole::Source,
         &source_open_validator,
         // run_source only ever resolves nothing: a SOURCE *initiator*
         // owns its own root, and a SOURCE *responder* driven directly
         // (the in-process role suite) is handed a Fixed source. The
         // daemon SOURCE responder resolves module→root inside
         // `run_responder`, not here (otp-5).
         None,
     )
     .await?;
 
     drive_source(
         cfg.plan_options,
         cfg.data_plane_host,
-        &negotiated,
+        negotiated,
         transport,
         source,
     )
     .await
 }
 
 /// The SOURCE session body after establish: spawn the receive half,
 /// run the send half, and map a fault to a peer-notified report. Shared
 /// by [`run_source`] (initiator or direct-responder) and
 /// [`run_responder`] (the daemon SOURCE responder), so the send/receive
 /// choreography is single-sourced.
 async fn drive_source(
     plan_options: PlanOptions,
     data_plane_host: Option<String>,
-    negotiated: &Negotiated,
+    mut negotiated: Negotiated,
     transport: FrameTransport,
     source: Arc<dyn TransferSource>,
 ) -> Result<TransferSummary> {
+    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
+    // its send sockets on; a SOURCE initiator (push) has none and dials the
+    // grant it received instead. Take it here so the send half owns it.
+    let responder_data_plane = negotiated.responder_data_plane.take();
     let (mut tx, rx) = transport.split();
     let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
     // Set by the send half the moment ManifestComplete goes out. On
     // an ordered transport, a NeedComplete arriving while this is
     // still false is provably premature — the peer cannot have
     // received what we have not sent (contract: NeedComplete only
     // after ManifestComplete received + all entries diffed).
     let manifest_sent = Arc::new(AtomicBool::new(false));
     let (event_tx, event_rx) = mpsc::unbounded_channel();
     // AbortOnDrop: an early error return below must abort the receive
     // half instead of leaking it (same rationale as design-2 / w4-1).
     let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
         rx,
         Arc::clone(&sent),
         Arc::clone(&manifest_sent),
         event_tx,
     )));
 
     match source_send_half(
         plan_options,
         data_plane_host.as_deref(),
-        negotiated,
+        &negotiated,
+        responder_data_plane,
         &mut tx,
         source,
         sent,
         &manifest_sent,
         event_rx,
     )
     .await
     {
         Ok(summary) => Ok(summary),
         Err(report) => {
             let mut fault = fault_from_report(report);
             if !fault.peer_notified {
                 let _ = tx.send(error_frame(&fault)).await;
                 fault.peer_notified = true;
             }
             Err(eyre::Report::new(fault))
         }
     }
 }
 
 /// Receive half of the source driver: drains the transport for the
 /// whole session so destination sends can never deadlock against a
 /// blocked source send, and routes the destination lane to the send
 /// half. Terminates on summary, error, close, or violation.
 async fn source_recv_half(
     mut rx: Box<dyn FrameRx>,
     sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
     manifest_sent: Arc<AtomicBool>,
     events: mpsc::UnboundedSender<SourceEvent>,
 ) {
     loop {
         let received = match rx.recv().await {
             Ok(Some(f)) => f,
             Ok(None) => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::internal(
                     "peer closed before TransferSummary",
                 )));
                 return;
             }
             Err(err) => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
                     "transport receive failed: {err:#}"
                 ))));
                 return;
             }
         };
         match received.frame {
             Some(Frame::NeedBatch(batch)) => {
                 for entry in batch.entries {
                     if entry.resume {
                         let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                             format!(
                                 "resume-flagged need for '{}' in a session opened without resume",
                                 entry.relative_path
                             ),
                         )));
                         return;
                     }
                     let header = sent
                         .lock()
                         .expect("sent-manifest lock poisoned")
                         .remove(&entry.relative_path);
                     match header {
                         Some(h) => {
                             let _ = events.send(SourceEvent::Need(h));
                         }
                         None => {
                             let _ = events.send(SourceEvent::Fault(
                                 SessionFault::protocol_violation(format!(
                                     "need for unknown or already-needed path '{}'",
                                     entry.relative_path
                                 )),
                             ));
                             return;
                         }
                     }
                 }
             }
             Some(Frame::NeedComplete(_)) => {
                 if !manifest_sent.load(Ordering::Acquire) {
                     // Fail fast at arrival time (otp-3 codex F2): the
                     // event queue would otherwise let an early
                     // NeedComplete be processed late and pass as
                     // legitimate.
                     let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                         "NeedComplete before the source's ManifestComplete",
                     )));
                     return;
                 }
                 let _ = events.send(SourceEvent::NeedComplete);
             }
             Some(Frame::ResizeAck(ack)) => {
                 // The destination's response to a shape-resize proposal
                 // (otp-4b-2). Forward it to the send half, which owns the
                 // dial and dials the epoch-N socket on `accepted`.
                 let _ = events.send(SourceEvent::ResizeAck(ack));
             }
             Some(Frame::Summary(summary)) => {
                 let _ = events.send(SourceEvent::Summary(summary));
                 return;
             }
             Some(Frame::Error(err)) => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
                 return;
             }
             other => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                     format!("{} on the source's receive lane", frame_name(&other)),
                 )));
                 return;
             }
         }
     }
 }
 
 #[allow(clippy::too_many_arguments)]
 async fn source_send_half(
     plan_options: PlanOptions,
     data_plane_host: Option<&str>,
     negotiated: &Negotiated,
+    responder_data_plane: Option<data_plane::ResponderDataPlane>,
     tx: &mut Box<dyn FrameTx>,
     source: Arc<dyn TransferSource>,
     sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
     manifest_sent: &AtomicBool,
     mut events: mpsc::UnboundedReceiver<SourceEvent>,
 ) -> Result<TransferSummary> {
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut need_complete = false;
 
-    // Data plane (otp-4b): dial the granted TCP sockets up front —
-    // BEFORE streaming the manifest — so the destination's accept loop
-    // (armed the moment it sent SessionAccept) sees the connections
-    // promptly rather than waiting out its bounded-accept timeout while
-    // a long manifest streams. The sockets sit idle (keepalive covers
-    // that) until payloads are queued below. `None` = the in-stream
-    // carrier (fallback), which needs no early setup.
-    let mut data_plane = match &negotiated.accept.data_plane {
-        Some(grant) => {
-            let host = data_plane_host.ok_or_else(|| {
-                eyre::Report::new(SessionFault::internal(
-                    "responder granted a TCP data plane but this initiator has no host to dial",
-                ))
-            })?;
-            Some(
-                data_plane::dial_source_data_plane(
-                    host,
-                    grant,
-                    negotiated.accept.receiver_capacity.as_ref(),
-                    Arc::clone(&source),
-                )
-                .await?,
+    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
+    // streaming the manifest — so the peer sees the connections promptly
+    // rather than waiting out a bounded-accept/connect timeout while a long
+    // manifest streams. Which end connects depends on connection role
+    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
+    // listener; a SOURCE **initiator** (push) dials the grant it received.
+    // Byte direction is the same either way (SOURCE sends), so both yield a
+    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
+    // in-stream carrier (fallback), which needs no early setup.
+    let mut data_plane = match responder_data_plane {
+        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
+        // initiator advertised its capacity in the open (byte RECEIVER
+        // advertises, wherever it initiates); the accept plane is single-
+        // stream (otp-5b-1).
+        Some(bound) => Some(
+            data_plane::accept_source_data_plane(
+                bound,
+                negotiated.open.receiver_capacity.as_ref(),
+                Arc::clone(&source),
             )
-        }
-        None => None,
+            .await?,
+        ),
+        // SOURCE initiator (push, otp-4b): dial the grant if the responder
+        // granted a data plane; else in-stream.
+        None => match &negotiated.accept.data_plane {
+            Some(grant) => {
+                let host = data_plane_host.ok_or_else(|| {
+                    eyre::Report::new(SessionFault::internal(
+                        "responder granted a TCP data plane but this initiator has no host to dial",
+                    ))
+                })?;
+                Some(
+                    data_plane::dial_source_data_plane(
+                        host,
+                        grant,
+                        negotiated.accept.receiver_capacity.as_ref(),
+                        Arc::clone(&source),
+                    )
+                    .await?,
+                )
+            }
+            None => None,
+        },
     };
 
     // sf-2 shape correction (otp-4b-2): running totals of the need list,
     // fed to the shape table so the SOURCE grows the data-plane stream
     // count as the workload's shape becomes known. Append-only (a need is
     // counted once, when it arrives), and the in-flight resize record the
     // ack is matched against (at most one — the dial enforces it).
     let mut needed_bytes: u64 = 0;
     let mut needed_count: usize = 0;
     let mut pending_resize: Option<data_plane::PendingResize> = None;
 
     // Streaming manifest: entries go out as enumeration produces them
     // (immediate start in every direction — plan §Design 2). The open
     // carries no source path: the source end owns its local endpoint.
     let _ = &negotiated.open;
     let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
     let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
     while let Some(header) = header_rx.recv().await {
         sent.lock()
             .expect("sent-manifest lock poisoned")
             .insert(header.relative_path.clone(), header.clone());
         tx.send(frame(Frame::ManifestEntry(header))).await?;
         // Faults detected by the receive half abort the stream now,
         // not after the full scan; needs just accumulate. (Resize acks
         // cannot arrive yet — none is proposed before the payload phase.)
         drain_ready_source_events(
             &mut events,
             &mut pending,
             &mut need_complete,
             &mut needed_bytes,
             &mut needed_count,
             data_plane.as_ref(),
             tx,
             &mut pending_resize,
         )
         .await?;
     }
     let scanned = scan_handle
         .await
         .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
     let scan_complete = unreadable
         .lock()
         .expect("unreadable list lock poisoned")
         .is_empty();
     log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
     tx.send(frame(Frame::ManifestComplete(ManifestComplete {
         scan_complete,
     })))
     .await?;
     manifest_sent.store(true, Ordering::Release);
 
     // Payload phase. The byte carrier is either the TCP data plane
     // (dialed above) or the in-stream record grammar (fallback). Needs
     // accumulated while a batch was being sent become the next planner
     // batch (contract §Transport selection); payloads only flow after
     // ManifestComplete.
     // The in-stream carrier reuses one read buffer across records; the
     // data plane owns its own pooled buffers, so skip that allocation.
     let mut read_buf = if data_plane.is_none() {
         vec![0u8; IN_STREAM_CHUNK]
     } else {
         Vec::new()
     };
     loop {
         drain_ready_source_events(
             &mut events,
             &mut pending,
             &mut need_complete,
             &mut needed_bytes,
             &mut needed_count,
             data_plane.as_ref(),
             tx,
             &mut pending_resize,
         )
         .await?;
         if !pending.is_empty() {
             let batch = std::mem::take(&mut pending);
             match &mut data_plane {
                 Some(dp) => {
                     // sf-2: correct the stream count toward the shape the
@@ -1394,494 +1428,545 @@ pub struct DestinationOutcome {
     /// role suite pins these identical across role assignments — the
     /// executable form of the owner's invariance requirement.
     pub needed_paths: Vec<String>,
     /// The settled data-plane stream count this end observed (epoch-0 +
     /// accepted resizes), or `None` for the in-stream carrier. The sf-2
     /// pin (otp-4b-2) reads it to assert shape correction grew the
     /// stream set past the zero-knowledge single-stream grant.
     pub data_plane_streams: Option<usize>,
 }
 
 /// Run the DESTINATION role of one transfer session over `transport`,
 /// writing under the root named by `target`. Diffs the streamed
 /// manifest against its own filesystem (the destination is the one
 /// diff owner — plan §Design 3), returns the summary it computed and
 /// sent.
 ///
 /// `target` is [`DestinationTarget::Fixed`] when the root is known up
 /// front (an Initiator's own local root, or a test), or
 /// [`DestinationTarget::Resolve`] when the root must be resolved from
 /// the received `SessionOpen` mid-handshake (the daemon Responder,
 /// where the wire module name selects the root).
 pub async fn run_destination(
     cfg: DestinationSessionConfig,
     transport: FrameTransport,
     target: DestinationTarget,
 ) -> Result<DestinationOutcome> {
     let mut transport = transport;
     let endpoint = match cfg.endpoint {
         SessionEndpoint::Initiator { mut open } => {
             let declared = TransferRole::try_from(open.initiator_role);
             if declared != Ok(TransferRole::Destination) {
                 eyre::bail!(
                     "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
                 );
             }
             if let Err(fault) = destination_open_validator(&open) {
                 eyre::bail!("run_destination initiator config unsupported: {fault}");
             }
             // Dial contract: the byte receiver advertises capacity in
             // its open when it is the initiator (contract §Invariants 5).
             if open.receiver_capacity.is_none() {
                 open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
             }
             SessionEndpoint::Initiator { open }
         }
         SessionEndpoint::Responder => SessionEndpoint::Responder,
     };
 
     let resolve_open: Option<&OpenResolver> = match &target {
         DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
         DestinationTarget::Fixed(_) => None,
     };
 
     let negotiated = establish(
         &mut transport,
         &cfg.hello,
         &endpoint,
         TransferRole::Destination,
         &destination_open_validator,
         resolve_open,
     )
     .await?;
 
     // The resolver's root (Responder + Resolve) wins; otherwise the
     // caller-supplied Fixed root.
     let dst_root = match negotiated.resolved_root.clone() {
         Some(root) => root,
         None => match &target {
             DestinationTarget::Fixed(root) => root.clone(),
             // Unreachable: a Resolve target always yields a root on the
             // Responder branch, and establish only skips resolution on
             // the Initiator branch (which pairs with a Fixed root).
             DestinationTarget::Resolve(_) => {
                 return Err(eyre::Report::new(SessionFault::internal(
                     "resolver target produced no destination root",
                 )));
             }
         },
     };
 
-    drive_destination(&mut transport, negotiated, &dst_root).await
+    drive_destination(
+        &mut transport,
+        negotiated,
+        &dst_root,
+        cfg.data_plane_host.as_deref(),
+    )
+    .await
 }
 
 /// The DESTINATION session body: run the diff/receive loop and map a
 /// fault to a peer-notified report. Shared by [`run_destination`] and
 /// [`run_responder`] (the daemon DESTINATION responder), so the receive
 /// choreography is single-sourced.
 async fn drive_destination(
     transport: &mut FrameTransport,
     negotiated: Negotiated,
     dst_root: &Path,
+    data_plane_host: Option<&str>,
 ) -> Result<DestinationOutcome> {
-    match destination_session(transport, negotiated, dst_root).await {
+    match destination_session(transport, negotiated, dst_root, data_plane_host).await {
         Ok(outcome) => Ok(outcome),
         Err(report) => {
             let mut fault = fault_from_report(report);
             if !fault.peer_notified {
                 let _ = transport.send(error_frame(&fault)).await;
                 fault.peer_notified = true;
             }
             Err(eyre::Report::new(fault))
         }
     }
 }
 
 /// Serve one transfer session as the RESPONDER, dispatching on the
 /// initiator's declared role — the daemon's single serving entry
 /// (contract §Invariants 3: one handshake, roles not directions). A
 /// client that declares SOURCE makes this end the DESTINATION
 /// (push-equivalent, otp-4); a client that declares DESTINATION makes
 /// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
 /// the endpoint resolution for each role; only the one the initiator
 /// selects is used. Returns a [`ResponderOutcome`] tagged with the role
 /// that ran.
 pub async fn run_responder(
     hello: HelloConfig,
     transport: FrameTransport,
     source_target: SourceResponderTarget,
     dest_target: DestinationTarget,
 ) -> Result<ResponderOutcome> {
     let mut transport = transport;
     exchange_hello(&mut transport, &hello).await?;
     let open = match expect_frame(&mut transport).await? {
         Frame::Open(o) => o,
         other => {
             return Err(notify_and_wrap(
                 &mut transport,
                 SessionFault::protocol_violation(format!(
                     "expected SessionOpen, got {}",
                     frame_name(&Some(other))
                 )),
             )
             .await)
         }
     };
     let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
     match declared {
         // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
         TransferRole::Source => {
             let resolve = match &dest_target {
                 DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
                 DestinationTarget::Fixed(_) => None,
             };
             let negotiated = responder_finish(
                 &mut transport,
                 open,
                 TransferRole::Destination,
                 &destination_open_validator,
                 resolve,
             )
             .await?;
             let dst_root = match negotiated.resolved_root.clone() {
                 Some(root) => root,
                 None => match &dest_target {
                     DestinationTarget::Fixed(root) => root.clone(),
                     DestinationTarget::Resolve(_) => {
                         return Err(eyre::Report::new(SessionFault::internal(
                             "resolver target produced no destination root",
                         )));
                     }
                 },
             };
-            let outcome = drive_destination(&mut transport, negotiated, &dst_root).await?;
+            // A DESTINATION responder (push) binds+accepts its receive
+            // sockets — it never dials, so it needs no data-plane host.
+            let outcome = drive_destination(&mut transport, negotiated, &dst_root, None).await?;
             Ok(ResponderOutcome::Destination(outcome))
         }
         // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
         TransferRole::Destination => {
             let resolve = match &source_target {
                 SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
                 SourceResponderTarget::Fixed(_) => None,
             };
             let negotiated = responder_finish(
                 &mut transport,
                 open,
                 TransferRole::Source,
                 &source_open_validator,
                 resolve,
             )
             .await?;
             let source: Arc<dyn TransferSource> = match source_target {
                 SourceResponderTarget::Fixed(source) => source,
                 SourceResponderTarget::Resolve(_) => {
                     // A Resolve target always yields a root on the
                     // Responder branch (establish only skips resolution
                     // on the Initiator branch, which uses Fixed).
                     let root = negotiated.resolved_root.clone().ok_or_else(|| {
                         eyre::Report::new(SessionFault::internal(
                             "resolver target produced no source root",
                         ))
                     })?;
                     Arc::new(FsTransferSource::new(root))
                 }
             };
             // The SOURCE owns its planner knobs; a daemon-served source
-            // has no client-supplied ones (§Transport selection). otp-5a
-            // is in-stream only, so there is no data-plane host to dial.
+            // has no client-supplied ones (§Transport selection). A SOURCE
+            // responder binds+accepts its send sockets (otp-5b) — it never
+            // dials, so it needs no data-plane host.
             let summary =
-                drive_source(PlanOptions::default(), None, &negotiated, transport, source).await?;
+                drive_source(PlanOptions::default(), None, negotiated, transport, source).await?;
             Ok(ResponderOutcome::Source(summary))
         }
         TransferRole::Unspecified => Err(notify_and_wrap(
             &mut transport,
             SessionFault::protocol_violation(
                 "initiator declared no role (TRANSFER_ROLE_UNSPECIFIED)",
             ),
         )
         .await),
     }
 }
 
 fn violation(message: String) -> eyre::Report {
     eyre::Report::new(SessionFault::protocol_violation(message))
 }
 
 async fn destination_session(
     transport: &mut FrameTransport,
     negotiated: Negotiated,
     dst_root: &Path,
+    data_plane_host: Option<&str>,
 ) -> Result<DestinationOutcome> {
     let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
         .unwrap_or(ComparisonMode::Unspecified);
     let compare_opts = CompareOptions {
         mode: compare_mode.into(),
         ignore_existing: negotiated.open.ignore_existing,
         include_deletions: false, // mirror lands at otp-6
     };
     // src_root is only consumed by local File payloads, which never
     // occur on a session destination (payload bytes arrive as records
     // and go through the stream/tar write paths). `Arc` so the data-plane
     // receive task (otp-4b) can share the one sink across sockets.
     let sink = Arc::new(FsTransferSink::new(
         PathBuf::new(),
         dst_root.to_path_buf(),
         FsSinkConfig {
             preserve_times: true,
             dry_run: false,
             checksum: None,
             resume: false,
             compare_mode,
         },
     ));
     // Same canonical-containment chokepoint the sink write paths use
     // (R46-F3), applied to diff stats so a hostile manifest path can't
     // make the destination stat outside its root.
     let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
 
     // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
     // `granted` is the ever-granted DEDUP set — control-loop-local,
     // insert-only, never removed, so a concurrent data-plane claim can
     // never re-open a grant (a duplicate manifest path is granted at
     // most once regardless of delivery timing). `outstanding` is the
     // not-yet-delivered COMPLETION set — inserted for each freshly
     // granted path before its NeedBatch, claimed by both carriers (the
     // in-stream arms inline, the data-plane NeedListSink as payloads
     // land), and empty at SourceDone. A count proxy was insufficient
     // (F1); merging the two into one set raced the data-plane claim
     // against the diff (fix-review F1).
     let mut granted: HashSet<String> = HashSet::new();
     let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
 
-    // Data plane (otp-4b): when the responder granted a TCP data plane,
-    // payload bytes arrive on sockets (not the control lane). Arm the
-    // accept+receive task NOW — concurrent with the diff loop below, and
-    // before the source dials — so the connections are accepted promptly.
-    // The NeedListSink gives the socket receive the same need-list
-    // strictness the in-stream control loop applies inline. AbortOnDrop
-    // bounds it to this future: a control-lane fault that returns from
-    // this fn aborts the receive task instead of leaking it.
-    // `resize_live` tracks the stream count this end has granted (epoch-0
-    // plus each accepted resize ADD); `resize_ceiling` is the receiver's
-    // advertised max_streams, the cumulative bound a resize may not cross.
-    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
-        match negotiated.responder_data_plane {
-            Some(rdp) => {
-                let initial = rdp.initial_streams() as usize;
-                let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
-                    Arc::clone(&sink) as Arc<dyn TransferSink>,
-                    Arc::clone(&outstanding),
-                ));
-                let run = rdp.spawn(recv_sink);
-                let ceiling = run.ceiling;
-                (Some(run), initial, ceiling)
+    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
+    // bytes arrive on sockets (not the control lane). Set it up NOW —
+    // concurrent with the diff loop below, and before the peer sends — so
+    // the connections are established promptly. Which end connects depends
+    // on connection role (otp-5b): a DESTINATION **responder** (push)
+    // accepts sockets off its bound listener; a DESTINATION **initiator**
+    // (pull) dials the grant it received on `data_plane_host`. Byte
+    // direction is the same either way (DESTINATION receives). The
+    // NeedListSink gives the socket receive the same need-list strictness
+    // the in-stream control loop applies inline; AbortOnDrop (inside the
+    // responder run) bounds the accept task to this future. `resize_live`
+    // tracks the stream count this end has granted (epoch-0 plus each
+    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
+    // max_streams — both meaningful only for the resize-armable responder
+    // path (push); the pull initiator path is single-stream (otp-5b-1).
+    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
+        Arc::clone(&sink) as Arc<dyn TransferSink>,
+        Arc::clone(&outstanding),
+    ));
+    let (mut data_plane_recv, mut resize_live, resize_ceiling) = match negotiated
+        .responder_data_plane
+    {
+        // DESTINATION responder (push, otp-4b): accept + receive.
+        Some(rdp) => {
+            let initial = rdp.initial_streams() as usize;
+            let run = rdp.spawn(recv_sink);
+            let ceiling = run.ceiling;
+            (
+                Some(data_plane::DestRecvPlane::Responder(run)),
+                initial,
+                ceiling,
+            )
+        }
+        // DESTINATION initiator (pull, otp-5b): dial + receive when the
+        // SOURCE responder granted a data plane and we have a host to
+        // dial; otherwise the in-stream carrier.
+        None => match (&negotiated.accept.data_plane, data_plane_host) {
+            (Some(grant), Some(host)) => {
+                let run = data_plane::dial_destination_data_plane(host, grant, recv_sink).await?;
+                // Single-stream (otp-5b-1): no resize is accepted, so
+                // the ceiling stays 0 and a Resize frame is a violation.
+                (
+                    Some(data_plane::DestRecvPlane::Initiator(run)),
+                    0usize,
+                    0usize,
+                )
             }
-            None => (None, 0usize, 0usize),
-        };
+            _ => (None, 0usize, 0usize),
+        },
+    };
 
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut needed_paths: Vec<String> = Vec::new();
     let mut manifest_complete = false;
     let mut files_written: u64 = 0;
     let mut bytes_written: u64 = 0;
 
     loop {
         let received = match transport.recv().await? {
             Some(f) => f,
             None => {
                 return Err(eyre::Report::new(SessionFault::internal(
                     "peer closed mid-session",
                 )))
             }
         };
         match received.frame {
             Some(Frame::ManifestEntry(header)) => {
                 if manifest_complete {
                     return Err(violation(format!(
                         "manifest entry '{}' after ManifestComplete",
                         header.relative_path
                     )));
                 }
                 pending.push(header);
                 if pending.len() >= DEST_DIFF_CHUNK {
                     let chunk = std::mem::take(&mut pending);
                     diff_chunk_and_send_needs(
                         transport,
                         chunk,
                         dst_root,
                         canonical_dst_root.as_deref(),
                         &compare_opts,
                         &mut granted,
                         &outstanding,
                         &mut needed_paths,
                     )
                     .await?;
                 }
             }
             Some(Frame::ManifestComplete(_complete)) => {
                 if manifest_complete {
                     return Err(violation("duplicate ManifestComplete".into()));
                 }
                 // (scan_complete gates mirror purges from otp-6 on;
                 // nothing consumes it in otp-3.)
                 let chunk = std::mem::take(&mut pending);
                 diff_chunk_and_send_needs(
                     transport,
                     chunk,
                     dst_root,
                     canonical_dst_root.as_deref(),
                     &compare_opts,
                     &mut granted,
                     &outstanding,
                     &mut needed_paths,
                 )
                 .await?;
                 // NeedComplete only after ManifestComplete received
                 // AND every entry diffed — both true here.
                 transport
                     .send(frame(Frame::NeedComplete(NeedComplete {})))
                     .await?;
                 manifest_complete = true;
             }
             Some(Frame::FileBegin(header)) => {
                 // Payload records ride the control lane only under the
                 // in-stream carrier; with a TCP data plane active they
                 // flow over the sockets, so one here is a violation.
                 if data_plane_recv.is_some() {
                     return Err(violation(format!(
                         "file record '{}' on the control lane while a TCP data plane is active",
                         header.relative_path
                     )));
                 }
                 if !manifest_complete {
                     return Err(violation(format!(
                         "payload record for '{}' before ManifestComplete",
                         header.relative_path
                     )));
                 }
                 if !outstanding
                     .lock()
                     .expect("outstanding-needs lock poisoned")
                     .remove(&header.relative_path)
                 {
                     return Err(violation(format!(
                         "payload for '{}' which is not on the need list",
                         header.relative_path
                     )));
                 }
                 let outcome = receive_file_record(transport, &sink, &header).await?;
                 files_written += outcome.files_written as u64;
                 bytes_written += outcome.bytes_written;
             }
             Some(Frame::TarShardHeader(shard)) => {
                 if data_plane_recv.is_some() {
                     return Err(violation(
                         "tar shard record on the control lane while a TCP data plane is active"
                             .into(),
                     ));
                 }
                 if !manifest_complete {
                     return Err(violation("tar shard record before ManifestComplete".into()));
                 }
                 {
                     let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
                     for h in &shard.files {
                         if !out.remove(&h.relative_path) {
                             return Err(violation(format!(
                                 "tar shard entry '{}' which is not on the need list",
                                 h.relative_path
                             )));
                         }
                     }
                 }
                 let outcome = receive_tar_record(transport, &sink, shard).await?;
                 files_written += outcome.files_written as u64;
                 bytes_written += outcome.bytes_written;
             }
             Some(Frame::Resize(resize)) => {
                 // sf-2 shape correction (otp-4b-2): the SOURCE proposes
                 // one ADD; arm the credential, grant it (bump `resize_live`),
                 // and ack so the SOURCE dials the epoch-N socket. Only ADD
                 // occurs on the session (REMOVE is a tuner concern, future
                 // work); anything else fails fast.
-                let run = data_plane_recv.as_ref().ok_or_else(|| {
-                    violation("DataPlaneResize on a session with no data plane".into())
-                })?;
+                let run = match data_plane_recv.as_ref() {
+                    Some(data_plane::DestRecvPlane::Responder(run)) => run,
+                    // The pull data plane is single-stream (otp-5b-1): the
+                    // SOURCE responder never proposes a resize, so one here
+                    // is a protocol violation (otp-5b-2 adds the accept-based
+                    // epoch-N socket + dial).
+                    Some(data_plane::DestRecvPlane::Initiator(_)) => {
+                        return Err(violation(
+                            "DataPlaneResize on the single-stream pull data plane (otp-5b-1)"
+                                .into(),
+                        ))
+                    }
+                    None => {
+                        return Err(violation(
+                            "DataPlaneResize on a session with no data plane".into(),
+                        ))
+                    }
+                };
                 let op = DataPlaneResizeOp::try_from(resize.op)
                     .unwrap_or(DataPlaneResizeOp::Unspecified);
                 if op != DataPlaneResizeOp::Add {
                     return Err(violation(format!(
                         "unsupported data-plane resize op {}",
                         op.as_str_name()
                     )));
                 }
                 if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
                     return Err(violation(
                         "DataPlaneResize sub_token must be 16 bytes".into(),
                     ));
                 }
                 // Cumulative ceiling bound (defense in depth — the
                 // source's dial already clamps to the same profile).
                 let accepted = resize_live < resize_ceiling && run.arm(resize.sub_token.clone());
                 if accepted {
                     resize_live += 1;
                 }
                 let effective = if accepted {
                     resize.target_stream_count
                 } else {
                     resize_live as u32
                 };
                 transport
                     .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
                         epoch: resize.epoch,
                         effective_stream_count: effective,
                         accepted,
                     })))
                     .await?;
             }
             Some(Frame::SourceDone(_)) => {
                 if !manifest_complete {
                     return Err(violation("SourceDone before ManifestComplete".into()));
                 }
                 // Completion, both carriers: the shared `outstanding`
                 // set must be empty (every granted need claimed exactly
                 // once). In-stream claims inline above; the data-plane
                 // NeedListSink claims as payloads land, so joining the
                 // receive task first drains the last of them (and
                 // surfaces any receive error / stall). Set membership —
                 // not a file count — is the contract (codex F1: a count
                 // proxy let a peer substitute or duplicate paths).
                 // `finish()` drops the arm sender (no more resizes), joins
                 // the accept loop, and reports the settled stream count.
                 let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
                     Some(run) => {
                         let totals = run.finish().await?;
                         files_written = totals.outcome.files_written as u64;
                         bytes_written = totals.outcome.bytes_written;
                         (false, Some(totals.streams))
                     }
                     None => (true, None),
                 };
                 let unfulfilled = outstanding
                     .lock()
                     .expect("outstanding-needs lock poisoned")
                     .len();
                 if unfulfilled != 0 {
                     return Err(violation(format!(
                         "SourceDone with {unfulfilled} needed file(s) never delivered"
                     )));
                 }
                 let summary = TransferSummary {
                     files_transferred: files_written,
                     bytes_transferred: bytes_written,
                     entries_deleted: 0, // mirror lands at otp-6
                     in_stream_carrier_used,
                     files_resumed: 0, // resume lands at otp-7
                 };
                 transport.send(frame(Frame::Summary(summary))).await?;
                 return Ok(DestinationOutcome {
                     summary,
                     needed_paths,
                     data_plane_streams,
                 });
             }
             Some(Frame::Error(err)) => {
                 return Err(eyre::Report::new(SessionFault::from_wire(err)));

exec
/usr/bin/zsh -lc 'git diff --unified=80 e6a0b3b''^ e6a0b3b -- crates/blit-core/src/transfer_session/data_plane.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/transfer_session/data_plane.rs b/crates/blit-core/src/transfer_session/data_plane.rs
index 521e4f7..d40e62a 100644
--- a/crates/blit-core/src/transfer_session/data_plane.rs
+++ b/crates/blit-core/src/transfer_session/data_plane.rs
@@ -1,126 +1,134 @@
 //! Session-side TCP data-plane orchestration (otp-4b).
 //!
 //! The unified session reuses blit-core's data-plane byte plumbing —
 //! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
 //! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
 //! its OWN choreography here. The push-specific bind/arm/accept loop
 //! (`blit-daemon` push service) and the multi-stream send driver
 //! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
 //! deletes at cutover (otp-10), so nothing in this file calls into them.
 //!
-//! The RESPONDER (whichever end is DESTINATION for otp-4/-5) binds a
-//! listener, mints the tokens, grants them in `SessionAccept`, and
-//! accepts + receives; the INITIATOR (SOURCE here) dials, authenticates,
-//! and sends. Because the grant is issued before any manifest is seen,
-//! the zero-knowledge `initial_stream_proposal` is 1 — the session data
-//! plane always starts single-stream (otp-4b-1).
+//! Two orthogonal axes (otp-5b): the **connection role** — the RESPONDER
+//! binds+accepts, the INITIATOR dials (NAT reality) — and the **byte
+//! role** — the SOURCE sends, the DESTINATION receives. otp-4b wired the
+//! push pair (DESTINATION responder accepts+receives; SOURCE initiator
+//! dials+sends); otp-5b adds the pull pair (SOURCE responder accepts+
+//! sends via [`accept_source_data_plane`]; DESTINATION initiator dials+
+//! receives via [`dial_destination_data_plane`]). The byte machinery is
+//! shared — send is `DataPlaneSession`/`DataPlaneSink`/the elastic
+//! pipeline, receive is `execute_receive_pipeline` — only socket
+//! acquisition differs per byte role. Because the grant is issued before
+//! any manifest is seen, the zero-knowledge `initial_stream_proposal` is
+//! 1 — the session data plane always starts single-stream (otp-4b-1); the
+//! pull data plane stays single-stream through otp-5b-1 (resize is
+//! otp-5b-2).
 //!
 //! otp-4b-2 adds mid-transfer growth: the SOURCE owns a [`TransferDial`]
 //! (bounded by the receiver's advertised capacity) and drives the sf-2
 //! shape correction — as the need list accumulates it re-runs the shape
 //! table and proposes `DataPlaneResize{ADD}` (one stream per epoch) on
 //! the control lane; the DESTINATION arms the credential, replies
 //! `DataPlaneResizeAck`, and accepts one more socket; the SOURCE dials
 //! the epoch-N socket and hands it to the running elastic pipeline via
 //! [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
 //! still future work — otp-4b-2 moves only the stream count.
 
 use std::collections::HashSet;
 use std::path::{Path, PathBuf};
 use std::sync::{Arc, Mutex as StdMutex};
 
 use async_trait::async_trait;
 use eyre::Result;
 use tokio::io::AsyncReadExt;
 use tokio::net::{TcpListener, TcpStream};
 use tokio::sync::mpsc;
 use tokio::task::JoinSet;
 
 use crate::buffer::BufferPool;
 use crate::engine::{initial_stream_proposal, local_receiver_capacity, TransferDial};
 use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
 use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
 use crate::remote::transfer::pipeline::execute_receive_pipeline;
 use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
 use crate::remote::transfer::socket::{
-    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
+    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
 };
 use crate::remote::transfer::source::TransferSource;
 use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
 use crate::remote::transfer::{
     execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
     SUB_TOKEN_LEN,
 };
 
 use super::SessionFault;
 
 /// The set of granted-but-not-yet-received needs, shared between the
 /// destination's control loop (which inserts each path before sending
 /// its `NeedBatch`) and the data-plane receive (which claims each path
 /// as its payload lands). Completion is an empty set — the same signal
 /// the in-stream carrier uses via its inline `outstanding.remove`.
 pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;
 
 fn dp_fault(msg: impl Into<String>) -> eyre::Report {
     eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
 }
 
 // ---------------------------------------------------------------------------
 // Responder (DESTINATION) — bind, grant, accept, receive
 // ---------------------------------------------------------------------------
 
 /// A bound data-plane listener plus the credentials the responder
 /// advertises in its `SessionAccept`. Held by the responder driver
 /// across the handshake so the accept loop can run after establish.
 pub(super) struct ResponderDataPlane {
     listener: TcpListener,
     session_token: Vec<u8>,
     epoch0_sub_token: Vec<u8>,
     initial_streams: u32,
     port: u16,
 }
 
 /// Bind a data-plane listener and mint credentials for the grant. Any
 /// failure (bind, addr, RNG) logs and returns `None` — the caller then
 /// issues a grant-less `SessionAccept` and the session falls back to the
 /// in-stream carrier (contract §Transport selection: a responder that
 /// cannot bind grants no data plane).
 pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
     let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
         Ok(listener) => listener,
         Err(err) => {
             log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
             return None;
         }
     };
     let port = match listener.local_addr() {
         Ok(addr) => addr.port(),
         Err(err) => {
             log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
             return None;
         }
     };
     // Two independent 16-byte credentials (contract §Transport: a socket
     // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
     // is the fallible-RNG minter — a missing system RNG is an error, not
     // a weaker credential.
     let session_token = match generate_sub_token() {
         Ok(token) => token,
         Err(err) => {
             log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
             return None;
         }
     };
     let epoch0_sub_token = match generate_sub_token() {
         Ok(token) => token,
         Err(err) => {
             log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
             return None;
         }
     };
     // The grant is issued before any manifest is seen, so the proposal
     // has zero knowledge: initial_streams == 1. All growth is via resize
     // (otp-4b-2). The ceiling is this end's own advertised max_streams.
     let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
     let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
     Some(ResponderDataPlane {
@@ -333,301 +341,490 @@ async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
             "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
         )))
         }
     };
     configure_data_socket(&socket, None)
         .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
     Ok(socket)
 }
 
 /// Read the fixed-length epoch-0 credential and verify it whole. A socket
 /// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
 /// arms exactly the sockets it dials, so a mismatch is fatal here).
 async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
     let mut socket = accept_raw(listener).await?;
     let mut buf = vec![0u8; expected.len()];
     let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
     match read {
         Ok(Ok(_)) => {}
         Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
         Err(_) => {
             return Err(dp_fault(format!(
                 "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
             )))
         }
     }
     // Constant-time comparison is not required: the tokens are 16 random
     // bytes read once per socket, single-session; a timing oracle buys
     // nothing against per-transfer secrets (same posture as the old push
     // acceptor's `token == expected_token`).
     if buf != expected {
         return Err(dp_fault(
             "data-plane socket presented an invalid credential",
         ));
     }
     Ok(socket)
 }
 
 /// Read a resize socket's `session_token ‖ sub_token(16)` credential
 /// (bounded), verify the session token, and match the sub-token against
 /// an armed credential — removing it so each arm is consumed once. Runs
 /// in the accept loop body (never a select arm), so a select cancel can
 /// never truncate a half-read socket.
 async fn authenticate_resize(
     socket: TcpStream,
     session_token: &[u8],
     armed: &mut Vec<Vec<u8>>,
 ) -> Result<TcpStream> {
     let mut socket = socket;
     let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
     let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
     match read {
         Ok(Ok(_)) => {}
         Ok(Err(err)) => {
             return Err(dp_fault(format!(
                 "reading resize data-plane credential: {err}"
             )))
         }
         Err(_) => {
             return Err(dp_fault(format!(
                 "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
             )))
         }
     }
     if buf[..session_token.len()] != *session_token {
         return Err(dp_fault(
             "resize data socket presented a wrong session token",
         ));
     }
     let sub = &buf[session_token.len()..];
     match armed.iter().position(|t| t.as_slice() == sub) {
         Some(idx) => {
             armed.swap_remove(idx);
             Ok(socket)
         }
         None => Err(dp_fault(
             "resize data socket presented an unarmed credential",
         )),
     }
 }
 
+// ---------------------------------------------------------------------------
+// Initiator (DESTINATION) — dial, receive (otp-5b-1)
+// ---------------------------------------------------------------------------
+
+/// Live handle to a DESTINATION **initiator** receive data plane
+/// (otp-5b-1, the pull direction): the initiator dials the granted
+/// epoch-0 socket(s) and drains each into the sink through the shared
+/// receive pipeline — the same byte machinery the DESTINATION responder
+/// uses, only the socket is dialed instead of accepted. Single-stream: no
+/// resize arming (otp-5b-2 adds the accept-based epoch-N socket + dial).
+/// [`Self::finish`] joins the workers for the aggregated write outcome +
+/// settled stream count.
+pub(super) struct InitiatorReceivePlaneRun {
+    receives: JoinSet<Result<SinkOutcome>>,
+    streams: usize,
+}
+
+/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
+/// socket. `host` is the responder's host (the initiator reached the
+/// control plane there; the data plane rides the same host on the granted
+/// port — contract §Transport: the initiator always dials). Each worker
+/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
+/// in-stream carrier applies inline).
+pub(super) async fn dial_destination_data_plane(
+    host: &str,
+    grant: &DataPlaneGrant,
+    sink: Arc<dyn TransferSink>,
+) -> Result<InitiatorReceivePlaneRun> {
+    let initial = grant.initial_streams.max(1) as usize;
+    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
+    let mut handshake = grant.session_token.clone();
+    handshake.extend_from_slice(&grant.epoch0_sub_token);
+    let addr = format!("{host}:{}", grant.tcp_port);
+
+    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
+    let mut streams = 0usize;
+    for _ in 0..initial {
+        // `dial_data_plane` connects, applies the data-socket policy, and
+        // writes the handshake credential — the same bounded dial the
+        // SOURCE initiator uses (design-3: one owner for every client-side
+        // data-plane dial, both directions).
+        let socket = dial_data_plane(&addr, &handshake, None)
+            .await
+            .map_err(|err| dp_fault(format!("dialing session data plane (receive): {err:#}")))?;
+        streams += 1;
+        spawn_receive(&mut receives, socket, &sink);
+    }
+    Ok(InitiatorReceivePlaneRun { receives, streams })
+}
+
+impl InitiatorReceivePlaneRun {
+    /// Join every receive worker for the aggregated write totals. A worker
+    /// error (receive failure / stall) surfaces here; each drains to its
+    /// socket's END record on a clean transfer.
+    async fn finish(mut self) -> Result<ReceiveTotals> {
+        let mut total = SinkOutcome::default();
+        while let Some(joined) = self.receives.join_next().await {
+            let outcome =
+                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
+            total.files_written += outcome.files_written;
+            total.bytes_written += outcome.bytes_written;
+        }
+        Ok(ReceiveTotals {
+            outcome: total,
+            streams: self.streams,
+        })
+    }
+}
+
+/// The DESTINATION end's receive data plane, tagged by connection role.
+/// Both drain socket bytes into the sink through the same receive
+/// pipeline; they differ only in how sockets are obtained (accept vs dial)
+/// and whether resize is armable (push only, otp-4b-2).
+pub(super) enum DestRecvPlane {
+    /// DESTINATION **responder** (push, otp-4b): accepts sockets, resize-
+    /// armable via the control loop.
+    Responder(ResponderDataPlaneRun),
+    /// DESTINATION **initiator** (pull, otp-5b-1): dialed single-stream
+    /// receive, no resize.
+    Initiator(InitiatorReceivePlaneRun),
+}
+
+impl DestRecvPlane {
+    /// Drain the data plane to completion and report the settled stream
+    /// count + write outcome (the DESTINATION is the scorer).
+    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
+        match self {
+            DestRecvPlane::Responder(run) => run.finish().await,
+            DestRecvPlane::Initiator(run) => run.finish().await,
+        }
+    }
+}
+
 // ---------------------------------------------------------------------------
 // Initiator (SOURCE) — dial, authenticate, send, resize
 // ---------------------------------------------------------------------------
 
 /// A resize the SOURCE has proposed and minted a credential for but not
 /// yet completed: the driver has sent (or will send) the matching
 /// `DataPlaneResize{ADD}` on the control lane and, on the peer's
 /// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
 /// flight (the dial's `pending_epoch` enforces it; this is the
 /// driver-side record the ack is matched against).
 pub(super) struct PendingResize {
     pub(super) epoch: u32,
     pub(super) target_streams: u32,
     pub(super) sub_token: Vec<u8>,
 }
 
 /// A running source-side data plane: the dialed socket(s) wrapped as an
 /// ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the sf-2
 /// shape correction). Planned payloads are fed via [`Self::queue`];
 /// closing via [`Self::finish`] drains the pipeline, emits each socket's
 /// END record, and returns the bytes this end sent.
 pub(super) struct SourceDataPlane {
     payload_tx: Option<mpsc::Sender<TransferPayload>>,
     control_tx: mpsc::UnboundedSender<SinkControl>,
     // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
     // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
     pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
     // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
     // advertised capacity (contract §Invariants 5). otp-4b-2 drives only
     // its shape-correction stream count; the cheap-dial tuner is future
     // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
     dial: Arc<TransferDial>,
     source: Arc<dyn TransferSource>,
     host: String,
     tcp_port: u32,
     session_token: Vec<u8>,
     pool: Arc<BufferPool>,
+    /// Whether this data plane grows mid-transfer via `DataPlaneResize`.
+    /// True for the SOURCE **initiator** (push, otp-4b-2: it dials each
+    /// epoch-N socket on the ack). False for the SOURCE **responder**
+    /// (pull, otp-5b-1): the accept-based epoch-N socket + ack→accept
+    /// choreography is otp-5b-2, so this slice stays single-stream and
+    /// `propose_resize` returns `None` regardless of the need list.
+    resizable: bool,
 }
 
 /// Dial the granted data plane and start the elastic send pipeline.
 /// `host` is the responder's host (the initiator connected the control
 /// plane to it; the data plane rides the same host on the granted port —
 /// contract §Transport: the initiator always dials). `receiver_capacity`
 /// is the DESTINATION's advertised profile from `SessionAccept`; it
 /// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
 /// never unlimited).
 pub(super) async fn dial_source_data_plane(
     host: &str,
     grant: &DataPlaneGrant,
     receiver_capacity: Option<&CapacityProfile>,
     source: Arc<dyn TransferSource>,
 ) -> Result<SourceDataPlane> {
     let initial = grant.initial_streams.max(1) as usize;
     // The byte sender's dial, bounded by the receiver's advertised
     // capacity. Seed the settled live count to the granted epoch-0
     // streams — every shape-resize proposal steps from here.
     let dial = TransferDial::conservative_within(receiver_capacity).shared();
     dial.set_negotiated_streams(initial);
 
     // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
     let mut handshake = grant.session_token.clone();
     handshake.extend_from_slice(&grant.epoch0_sub_token);
 
     // Provision the pool for the dial ceiling so resize-added sockets
     // draw buffers from the same pool without re-pooling (as old push
     // does — a shared pool sized for the maximum stream count).
     let pool = Arc::new(BufferPool::for_data_plane(
         dial.chunk_bytes(),
         dial.ceiling_max_streams().max(1),
     ));
     let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
     for _ in 0..initial {
         let session = DataPlaneSession::connect(
             host,
             grant.tcp_port,
             &handshake,
             dial.chunk_bytes(),
             dial.prefetch_count(),
             false,
             dial.tcp_buffer_bytes(),
             Arc::clone(&pool),
         )
         .await
         .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
         // The source-side sink never reads its dst_root (it only sends);
         // `root()` is consulted by the relay/receive case, not here.
         sinks.push(Arc::new(DataPlaneSink::new(
             session,
             Arc::clone(&source),
             PathBuf::new(),
         )));
     }
 
     let prefetch = dial.prefetch_count().max(1);
     let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
     let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
     let pipe_source = Arc::clone(&source);
     // Bounded by AbortOnDrop: a fault on the control lane that drops the
     // SourceDataPlane aborts the pipeline task instead of leaking it.
     let pipeline = AbortOnDrop::new(tokio::spawn(async move {
         execute_sink_pipeline_elastic(
             pipe_source,
             sinks,
             payload_rx,
             prefetch,
             None,
             Some(control_rx),
         )
         .await
     }));
     Ok(SourceDataPlane {
         payload_tx: Some(payload_tx),
         control_tx,
         pipeline: Some(pipeline),
         dial,
         source,
         host: host.to_string(),
         tcp_port: grant.tcp_port,
         session_token: grant.session_token.clone(),
         pool,
+        resizable: true,
+    })
+}
+
+/// Accept the granted epoch-0 socket(s) off a bound responder listener and
+/// start the elastic SEND pipeline over them — the SOURCE **responder**
+/// half of the pull data plane (otp-5b-1). Symmetric with
+/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
+/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
+/// socket acquisition differs (accept here, dial there).
+/// `DataPlaneSession::from_stream` builds a send session from an already-
+/// accepted socket — the same primitive the old `pull_sync` daemon-send
+/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
+/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
+/// wherever it initiates). Single-stream: `resizable: false`, so no
+/// `DataPlaneResize` is ever proposed on the pull data plane in this slice.
+pub(super) async fn accept_source_data_plane(
+    bound: ResponderDataPlane,
+    receiver_capacity: Option<&CapacityProfile>,
+    source: Arc<dyn TransferSource>,
+) -> Result<SourceDataPlane> {
+    let initial = bound.initial_streams.max(1) as usize;
+    // The byte sender's dial, bounded by the receiver's advertised
+    // capacity; seed the live count to the granted epoch-0 streams. Growth
+    // is disabled below (resizable=false), so the count stays here.
+    let dial = TransferDial::conservative_within(receiver_capacity).shared();
+    dial.set_negotiated_streams(initial);
+
+    // Epoch-0 credential the dialing DESTINATION presents:
+    // session_token ‖ epoch0_sub_token (contract §Transport).
+    let mut epoch0 = bound.session_token.clone();
+    epoch0.extend_from_slice(&bound.epoch0_sub_token);
+
+    let pool = Arc::new(BufferPool::for_data_plane(
+        dial.chunk_bytes(),
+        dial.ceiling_max_streams().max(1),
+    ));
+    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
+    for _ in 0..initial {
+        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
+        let session = DataPlaneSession::from_stream(
+            socket,
+            false,
+            dial.chunk_bytes(),
+            dial.prefetch_count(),
+            Arc::clone(&pool),
+        )
+        .await;
+        sinks.push(Arc::new(DataPlaneSink::new(
+            session,
+            Arc::clone(&source),
+            PathBuf::new(),
+        )));
+    }
+
+    let prefetch = dial.prefetch_count().max(1);
+    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
+    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
+    let pipe_source = Arc::clone(&source);
+    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
+        execute_sink_pipeline_elastic(
+            pipe_source,
+            sinks,
+            payload_rx,
+            prefetch,
+            None,
+            Some(control_rx),
+        )
+        .await
+    }));
+    Ok(SourceDataPlane {
+        payload_tx: Some(payload_tx),
+        control_tx,
+        pipeline: Some(pipeline),
+        dial,
+        source,
+        // Accept-based: this end never dials an epoch-N socket, so the
+        // dial-target fields are unused (add_stream is unreachable while
+        // resizable is false).
+        host: String::new(),
+        tcp_port: 0,
+        session_token: bound.session_token,
+        pool,
+        resizable: false,
     })
 }
 
 impl SourceDataPlane {
     /// The live dial (the byte sender owns it). The driver reads
     /// `live_streams()` for observability and calls `resize_settled` as
     /// each proposal completes.
     pub(super) fn dial(&self) -> &Arc<TransferDial> {
         &self.dial
     }
 
     /// sf-2 shape correction: propose one ADD toward the stream count the
     /// accumulated need list implies, if none is in flight and the shape
     /// wants more than the current live count. Mints the resize
     /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
     /// the record back on the matching ack.
     pub(super) fn propose_resize(
         &self,
         needed_bytes: u64,
         needed_count: usize,
     ) -> Result<Option<PendingResize>> {
+        // A non-resizable data plane (the SOURCE responder, otp-5b-1)
+        // never grows: the accept-based epoch-N socket is otp-5b-2.
+        if !self.resizable {
+            return Ok(None);
+        }
         let desired =
             initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
                 as usize;
         let Some(proposal) = self.dial.propose_shape_resize(desired) else {
             return Ok(None);
         };
         let sub_token = generate_sub_token()
             .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
         Ok(Some(PendingResize {
             epoch: proposal.epoch,
             target_streams: proposal.target_streams as u32,
             sub_token,
         }))
     }
 
     /// Dial the epoch-N data socket for an accepted resize and hand it to
     /// the running pipeline (`SinkControl::Add`). A dial failure is FATAL
     /// (fail-fast): a same-build peer whose listener already accepted
     /// epoch-0 failing an epoch-N dial is a transport fault worth
     /// surfacing — and faulting the session aborts the peer's accept loop
     /// via AbortOnDrop, so its armed slot never orphans. (Old push
     /// recovers non-fatally via an arm TTL; the session trades that for
     /// simplicity — noted in the finding doc.) If the pipeline is already
     /// gone (transfer completing under the ADD), the just-dialed socket
     /// is closed cleanly so the peer's worker sees its END, not a reset.
     pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
         let mut handshake = self.session_token.clone();
         handshake.extend_from_slice(sub_token);
         let session = DataPlaneSession::connect(
             &self.host,
             self.tcp_port,
             &handshake,
             self.dial.chunk_bytes(),
             self.dial.prefetch_count(),
             false,
             self.dial.tcp_buffer_bytes(),
             Arc::clone(&self.pool),
         )
         .await
         .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?;
         let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
             session,
             Arc::clone(&self.source),
             PathBuf::new(),
         ));
         if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
             if let SinkControl::Add(sink) = returned.0 {
                 let _ = sink.finish().await;
             }
         }
         Ok(())
     }
 
     /// Feed one planned batch into the send pipeline. The pipeline
     /// prepares each payload (tar-shard/file) and writes it through the
     /// data-plane record framing across the live socket(s).
     pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
         let tx = self.payload_tx.as_ref().ok_or_else(|| {
             eyre::Report::new(SessionFault::internal("data plane already finished"))
         })?;
         for payload in payloads {
             tx.send(payload).await.map_err(|_| {
                 dp_fault("data-plane send pipeline closed before all payloads sent")
             })?;
         }
         Ok(())
     }
 
     /// Signal end-of-stream, drain the pipeline (each worker emits its
     /// socket's END record on drain), and return the bytes sent. Must be
     /// awaited before `SourceDone` goes out so the destination's receive
     /// pipeline sees END and completes.
     pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
         // Drop the sender: workers observe the closed queue, drain what
         // is left, then `finish()` (END record) and exit.
         self.payload_tx = None;
         let pipeline = self
             .pipeline
             .take()
             .expect("SourceDataPlane::finish called once");

exec
/usr/bin/zsh -lc "rg -n \"struct TransferDial|fn conservative_within|ceiling_max_streams|DATA_PLANE_BUFFER_FLOOR|BufferPool::for_data_plane\" crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/transfer_session/data_plane.rs:587:    let pool = Arc::new(BufferPool::for_data_plane(
crates/blit-core/src/transfer_session/data_plane.rs:589:        dial.ceiling_max_streams().max(1),
crates/blit-core/src/transfer_session/data_plane.rs:674:    let pool = Arc::new(BufferPool::for_data_plane(
crates/blit-core/src/transfer_session/data_plane.rs:676:        dial.ceiling_max_streams().max(1),
crates/blit-core/src/transfer_session/data_plane.rs:752:            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
crates/blit-core/src/buffer.rs:19:/// pool formula (`BufferPool::for_data_plane`), the session chunk
crates/blit-core/src/buffer.rs:23:pub const DATA_PLANE_BUFFER_FLOOR: usize = 64 * KB;
crates/blit-core/src/buffer.rs:159:/// Data-plane paths construct it via [`BufferPool::for_data_plane`], which
crates/blit-core/src/buffer.rs:166:/// let pool = Arc::new(BufferPool::for_data_plane(dial.chunk_bytes(), stream_count));
crates/blit-core/src/buffer.rs:220:/// Pure sizing math behind [`BufferPool::for_data_plane`], split out so
crates/blit-core/src/buffer.rs:225:/// - `buffer_size = chunk_bytes.max(DATA_PLANE_BUFFER_FLOOR)`, shrunk
crates/blit-core/src/buffer.rs:243:    let mut buffer_size = chunk_bytes.max(DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:248:        buffer_size = (cap / (streams * 2)).max(DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:291:    /// `ceiling_max_streams()` rather than the epoch-0 count — buffers
crates/blit-core/src/buffer.rs:300:    /// (down to [`DATA_PLANE_BUFFER_FLOOR`]) instead of the concurrency:
crates/blit-core/src/buffer.rs:559:        assert_eq!(buffer_size, DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:561:        assert_eq!(budget, DATA_PLANE_BUFFER_FLOOR * 6 * 2);
crates/blit-core/src/buffer.rs:596:        assert_eq!(buffer_size, DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:597:        assert_eq!(budget, DATA_PLANE_BUFFER_FLOOR * 16 * 2);
crates/blit-core/src/buffer.rs:606:        for &chunk in &[0, 1024, DATA_PLANE_BUFFER_FLOOR, MIB, 16 * MIB, 64 * MIB] {
crates/blit-core/src/buffer.rs:617:                    assert!(buffer_size >= DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/buffer.rs:635:        let pool = Arc::new(BufferPool::for_data_plane(1024, 2));
crates/blit-core/src/buffer.rs:636:        assert!(pool.buffer_size() >= DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/engine/dial.rs:86:pub struct TransferDial {
crates/blit-core/src/engine/dial.rs:114:    ceiling_max_streams: usize,
crates/blit-core/src/engine/dial.rs:144:    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
crates/blit-core/src/engine/dial.rs:165:                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
crates/blit-core/src/engine/dial.rs:183:            ceiling_max_streams: ceiling_streams,
crates/blit-core/src/engine/dial.rs:214:    pub fn ceiling_max_streams(&self) -> usize {
crates/blit-core/src/engine/dial.rs:215:        self.ceiling_max_streams
crates/blit-core/src/engine/dial.rs:223:        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/engine/dial.rs:266:    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
crates/blit-core/src/engine/dial.rs:304:            (live + 1).min(self.ceiling_max_streams.max(1))
crates/blit-core/src/engine/dial.rs:350:        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/engine/dial.rs:384:            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/engine/dial.rs:393:        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/engine/dial.rs:642:        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
crates/blit-core/src/engine/dial.rs:653:        assert_eq!(dial.ceiling_max_streams(), 4);
crates/blit-core/src/engine/dial.rs:666:        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
crates/blit-core/src/remote/push/client/mod.rs:185:            (Some(_), Some(dial)) => dial.ceiling_max_streams().max(streams),
crates/blit-core/src/remote/push/client/mod.rs:188:        let pool = Arc::new(BufferPool::for_data_plane(chunk_bytes, authorized_streams));
crates/blit-core/src/remote/push/client/mod.rs:545:        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
crates/blit-core/src/remote/transfer/data_plane.rs:174:        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
crates/blit-core/src/remote/transfer/data_plane.rs:691:    let cap = buffer_size.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/dial.rs | sed -n '130,230p' && nl -ba crates/blit-core/src/buffer.rs | sed -n '280,330p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   130	    pub add: bool,
   131	}
   132	
   133	impl TransferDial {
   134	    /// Conservative start with default ceilings (no receiver profile).
   135	    pub fn conservative() -> Self {
   136	        Self::conservative_within(None)
   137	    }
   138	
   139	    /// Conservative start bounded by the receiver's advertised
   140	    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
   141	    /// fields mean UNKNOWN and keep the (already conservative)
   142	    /// default ceiling — never "unlimited". A profile can only lower
   143	    /// ceilings, never raise them above the defaults this slice.
   144	    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
   145	        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
   146	        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
   147	        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
   148	        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
   149	        if let Some(profile) = profile {
   150	            if profile.max_chunk_bytes > 0 {
   151	                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
   152	            }
   153	            if profile.max_streams > 0 {
   154	                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
   155	            }
   156	            if profile.max_inflight_bytes > 0 {
   157	                // The in-flight budget bounds the CHUNK ceiling first
   158	                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
   159	                // smaller than one chunk must still be honored — floor
   160	                // 64 KiB, matching the session's minimum buffer), then
   161	                // prefetch so prefetch × chunk stays within budget
   162	                // (floor of 1 so work still moves).
   163	                let inflight = profile.max_inflight_bytes as usize;
   164	                ceiling_chunk =
   165	                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
   166	                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
   167	                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
   168	            }
   169	        }
   170	        Self {
   171	            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
   172	            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
   173	            tcp_buffer_bytes: AtomicUsize::new(0),
   174	            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   175	            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
   176	            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   177	            resize_epoch: AtomicU32::new(0),
   178	            pending_epoch: AtomicU32::new(0),
   179	            ticks_since_settle: AtomicU32::new(0),
   180	            resize_sustain: AtomicI32::new(0),
   181	            ceiling_chunk_bytes: ceiling_chunk,
   182	            ceiling_prefetch,
   183	            ceiling_max_streams: ceiling_streams,
   184	            ceiling_tcp_buffer_bytes: ceiling_tcp,
   185	        }
   186	    }
   187	
   188	    pub fn shared(self) -> Arc<Self> {
   189	        Arc::new(self)
   190	    }
   191	
   192	    // ── live reads ───────────────────────────────────────────────────
   193	    pub fn chunk_bytes(&self) -> usize {
   194	        self.chunk_bytes.load(Ordering::Relaxed)
   195	    }
   196	    pub fn prefetch_count(&self) -> usize {
   197	        self.prefetch_count.load(Ordering::Relaxed)
   198	    }
   199	    /// `None` = leave the kernel default (old `tcp_buffer_size`
   200	    /// semantics). Connect-time dial.
   201	    pub fn tcp_buffer_bytes(&self) -> Option<usize> {
   202	        match self.tcp_buffer_bytes.load(Ordering::Relaxed) {
   203	            0 => None,
   204	            n => Some(n),
   205	        }
   206	    }
   207	    pub fn initial_streams(&self) -> usize {
   208	        self.initial_streams.load(Ordering::Relaxed)
   209	    }
   210	    /// Ceiling on the negotiated stream count (profile-clamped).
   211	    pub fn max_streams(&self) -> usize {
   212	        self.max_streams.load(Ordering::Relaxed)
   213	    }
   214	    pub fn ceiling_max_streams(&self) -> usize {
   215	        self.ceiling_max_streams
   216	    }
   217	
   218	    /// Record the stream count the negotiation actually settled on
   219	    /// (clamped to the dial's ceiling). This is the epoch-0 settle:
   220	    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
   221	    /// resize proposal steps from.
   222	    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
   223	        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
   224	        self.initial_streams.store(clamped, Ordering::Relaxed);
   225	        self.live_streams.store(clamped, Ordering::Relaxed);
   226	        clamped
   227	    }
   228	
   229	    // ── ue-r2-2 resize policy ────────────────────────────────────────
   230	
   280	    }
   281	
   282	    /// W3.1: the single owner of the data-plane pool formula. Every
   283	    /// data-plane transfer pool (push client, pull-sync multistream,
   284	    /// pull-sync resume) is built here instead of pasting
   285	    /// `streams*2+4` / `.max(64 KiB)` / `budget = size*pool*2` at the
   286	    /// call site.
   287	    ///
   288	    /// `streams` is the pool's **concurrency authorization**: the most
   289	    /// data-plane streams that may ever draw from this pool at once.
   290	    /// Elastic (resize-enabled) paths pass the dial's
   291	    /// `ceiling_max_streams()` rather than the epoch-0 count — buffers
   292	    /// are allocated lazily, so authorizing the ceiling costs no memory
   293	    /// until streams actually grow, and ADDed streams can never starve
   294	    /// against a budget sized for epoch 0.
   295	    ///
   296	    /// The memory budget is capped at a quarter of available system
   297	    /// memory (the OOM-by-constant fix: 16 streams × 64 MiB chunks used
   298	    /// to authorize 4.5 GiB regardless of host RAM). When the cap binds
   299	    /// harder than two buffers per stream, the *buffer size* shrinks
   300	    /// (down to [`DATA_PLANE_BUFFER_FLOOR`]) instead of the concurrency:
   301	    /// the send path holds up to two buffers per stream
   302	    /// (`send_file_double_buffered` acquires them sequentially), so a
   303	    /// budget below `2 × streams` buffers could deadlock a stream
   304	    /// against its own first buffer. The liveness floor
   305	    /// `budget ≥ buffer_size × streams × 2` therefore always wins over
   306	    /// the cap.
   307	    pub fn for_data_plane(chunk_bytes: usize, streams: usize) -> Self {
   308	        let (buffer_size, pool_size, budget) =
   309	            data_plane_pool_params(chunk_bytes, streams, available_memory_bytes());
   310	        Self::new(buffer_size, pool_size, Some(budget))
   311	    }
   312	
   313	    /// Acquire a buffer from the pool.
   314	    ///
   315	    /// Returns a buffer from the cache if available, otherwise allocates a new one.
   316	    /// If a memory budget is set and exceeded, this will wait until memory is released.
   317	    pub async fn acquire(self: &Arc<Self>) -> PoolBuffer {
   318	        // If we have a memory budget, acquire a permit (waits if the
   319	        // budget is exhausted). Keep it OWNED in a local through the
   320	        // allocation below — `std::mem::forget` (transfer to manual
   321	        // release via PoolBuffer::drop) happens only after the
   322	        // possibly-panicking `vec!`. If the allocation panics (capacity
   323	        // overflow / OOM unwind), the local permit drops during unwind
   324	        // and the semaphore is restored instead of leaking a permit
   325	        // forever (audit-12).
   326	        let permit = match self.memory_semaphore {
   327	            Some(ref sem) => Some(
   328	                sem.clone()
   329	                    .acquire_owned()
   330	                    .await

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/pull.rs | sed -n '300,330p;1870,1895p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   300	        Option<tokio::sync::mpsc::UnboundedSender<PullStreamAdd>>,
   301	    )> {
   302	        if negotiation.tcp_port == 0 {
   303	            bail!("server provided zero data-plane port for pull");
   304	        }
   305	        let token = general_purpose::STANDARD_NO_PAD
   306	            .decode(negotiation.one_time_token.as_bytes())
   307	            .map_err(|err| eyre!("failed to decode pull data-plane token: {err}"))?;
   308	
   309	        // Clone/own all values for the spawned task
   310	        let host = self.endpoint.host.clone();
   311	        let port = negotiation.tcp_port;
   312	        let stream_count = bounded_stream_count(negotiation.stream_count);
   313	        let dest_root = dest_root.to_path_buf();
   314	        let progress = progress.cloned();
   315	        let byte_progress = byte_progress.cloned();
   316	
   317	        // ue-r2-2: the daemon negotiated resize — hand the control
   318	        // loop a growth channel into the receiver task, and make the
   319	        // epoch-0 sockets echo the sub-token. A malformed token
   320	        // length reads as "not enabled" (fail toward today's shape).
   321	        let resize_on = negotiation.resize_enabled
   322	            && negotiation.epoch0_sub_token.len() == crate::remote::transfer::SUB_TOKEN_LEN;
   323	        let (growth_tx, resize_arg) = if resize_on {
   324	            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
   325	            (Some(tx), Some((negotiation.epoch0_sub_token.clone(), rx)))
   326	        } else {
   327	            (None, None)
   328	        };
   329	
   330	        let handle = tokio::spawn(async move {
  1870	    }
  1871	
  1872	    Ok(result)
  1873	}
  1874	
  1875	/// Dial the pull data plane and present the handshake bytes
  1876	/// (`one_time_token`, plus the resize sub-token suffix when
  1877	/// negotiated). Split from the receive half at ue-r2-2 so an ADDed
  1878	/// stream can treat dial failure as non-fatal.
  1879	async fn connect_pull_stream(host: &str, port: u32, handshake: &[u8]) -> Result<TcpStream> {
  1880	    let addr = format!("{}:{}", host, port);
  1881	    // design-3: bounded dial (connect + w1-2 socket policy + handshake
  1882	    // write) via the shared data-plane helper. No tuned buffer: the
  1883	    // pull dial lives on the daemon — the byte sender — so the client
  1884	    // has no value to apply.
  1885	    crate::remote::transfer::socket::dial_data_plane(&addr, handshake, None)
  1886	        .await
  1887	        .context("dialing pull data plane")
  1888	}
  1889	
  1890	async fn receive_on_pull_stream(
  1891	    stream: TcpStream,
  1892	    dest_root: &Path,
  1893	    track_paths: bool,
  1894	    progress: Option<&RemotePullProgress>,
  1895	    byte_progress: Option<&ByteProgressSink>,

codex
- `crates/blit-core/src/transfer_session/mod.rs:1735` — **Medium** — If a DESTINATION initiator receives `accept.data_plane` but `data_plane_host` is `None`, it silently takes the in-stream branch. The SOURCE responder has already granted TCP and blocks in `accept_source_data_plane()` waiting for that dial, so this cannot actually fall back; it degrades into a bounded 30s accept timeout instead of fail-fast. This violates the contract that the initiator dials when a grant exists. Mirror `source_send_half`’s explicit “grant but no host” error.

VERDICT: FAIL — one ordering/fallback bug found; push path and single-stream resize suppression otherwise look unchanged/correct from the diff.
tokens used
193,161
- `crates/blit-core/src/transfer_session/mod.rs:1735` — **Medium** — If a DESTINATION initiator receives `accept.data_plane` but `data_plane_host` is `None`, it silently takes the in-stream branch. The SOURCE responder has already granted TCP and blocks in `accept_source_data_plane()` waiting for that dial, so this cannot actually fall back; it degrades into a bounded 30s accept timeout instead of fail-fast. This violates the contract that the initiator dials when a grant exists. Mirror `source_send_half`’s explicit “grant but no host” error.

VERDICT: FAIL — one ordering/fallback bug found; push path and single-stream resize suppression otherwise look unchanged/correct from the diff.
