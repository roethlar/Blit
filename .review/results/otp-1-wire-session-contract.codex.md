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
session id: 019f331d-b85e-7e32-bf4f-5bc01134c60b
--------
user
Review the diff of commit a3e2acb (run: git show a3e2acb). It implements otp-1 of docs/plan/ONE_TRANSFER_PATH.md (Active, D-2026-07-05-4): the unified Transfer session's wire+session contract — docs/TRANSFER_SESSION.md (authoritative state machine + frame table), proto/blit.proto additions (Transfer RPC, TransferRole, SessionHello/Open/Accept, DataPlaneGrant, NeedBatch/NeedEntry/NeedComplete, SourceDone, TransferSummary, SessionError, 20-arm TransferFrame oneof), UNIMPLEMENTED stubs in the daemon service + five test fakes, and an in-process pin test. No behavior lands. Check: (1) contract completeness and internal coherence — could a competent implementer build otp-3/otp-4 from this contract without inventing semantics (phase machine, role rules, error paths, transport selection, resume ordering, mirror rule)? (2) consistency with the governing decisions: D-2026-07-05-1 (one path — no per-direction message or semantics anywhere), D-2026-07-05-2 (same-build only — no version tolerance, no capability bits, no advisory fields), D-2026-06-20-1/-2 (bounded-unilateral dial: receiver capacity DESTINATION→SOURCE, sender-owned dial), sf-2 stream policy, REV4 RELIABLE invariants (StallGuard/cancel/byte-accounting/byte-identical named); (3) proto quality: field numbers coherent with the doc's frozen table, message reuse sound (FileHeader/TarShard*/Block*/DataPlaneResize/CapacityProfile), enum hygiene, nothing that will force a wire break during otp-3..10; (4) gaps/risks the contract should name but doesn't (e.g. large-manifest flow control, need-batch backpressure, data-plane socket auth with session_token vs epoch0_sub_token, cancellation mid-phase, in-stream carrier interleaving rules); (5) the stubs and pin test are honest (test count 1483 -> 1484, no dropped tests). Output a concise markdown findings list — each finding with file:line/section, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this as a diff review only: first re-grounding in the repo guidance, then reading `git show a3e2acb` and the governing docs it references so findings are tied to the contract, not guesses.
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
/usr/bin/zsh -lc 'pwd && git rev-parse --show-toplevel && git status --short && git show --stat --oneline --decorate --no-renames a3e2acb' in /home/michael/dev/Blit
 succeeded in 0ms:
/home/michael/dev/Blit
/home/michael/dev/Blit
?? .review/results/otp-1-wire-session-contract.codex.md
a3e2acb (HEAD -> master) otp-1: unified Transfer session wire contract (doc + proto + stubs)
 .review/findings/otp-1-wire-session-contract.md    |  86 ++++++++++
 crates/blit-cli/tests/jobs_lifecycle.rs            |  13 ++
 crates/blit-cli/tests/remote_remote.rs             |  26 +++
 crates/blit-core/tests/pull_sync_with_spec_wire.rs |  22 +++
 crates/blit-daemon/src/service/core.rs             |  13 ++
 crates/blit-daemon/src/service/mod.rs              |   1 +
 crates/blit-daemon/src/service/transfer.rs         |  68 ++++++++
 docs/TRANSFER_SESSION.md                           | 184 +++++++++++++++++++++
 proto/blit.proto                                   | 163 ++++++++++++++++++
 9 files changed, 576 insertions(+)

exec
/usr/bin/zsh -lc "sed -n '1,260p' .agents/decisions.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Agent Decisions (pointer)

Durable decisions for this repo live canonically in **`docs/DECISIONS.md`** —
read and append there, not here. This stub exists only so tools expecting the
standard `.agents/` layout find their way.

Do not record decisions here. New entries go through the `decision` procedure in
`docs/agent/PROTOCOL.md`, which also propagates supersessions into the affected
plan docs.

exec
/usr/bin/zsh -lc "sed -n '1,260p' .agents/state.md" in /home/michael/dev/Blit
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

codex
The repo’s local guidance redirects state and decisions to `docs/STATE.md` and `docs/DECISIONS.md`; I’m reading those plus the active transfer plan before judging whether the new contract is complete.
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,320p' docs/DECISIONS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 flip: "flip the plan and go") — otp-1 in progress**
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
  be symmetric-fs disk-to-disk (owner: "tmp on one side, spinning
  rust on the other is not a valid test"), tmpfs = wire-reference
  rows only. **D-2026-07-05-2: no version compatibility, EVER —
  same-build peers only, mismatched builds refuse at session open
  (strict handshake specified in otp-1); REV4's negotiate-down
  clause is void, annotated.** Current slice: **otp-1 wire+session
  contract** (doc + proto, no behavior) through the codex loop.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1 `[x]`
  sf-2 `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`, codex 1/1,
  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
  baseline. Its principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence at
  `docs/bench/10gbe-2026-07-05/`; binaries staged at `blit-bin/`.
- **Tool comparison measured (2026-07-05)** — blit fastest on all
  large/pull/local cells at the wire ceiling; rsyncd faster on small/
  mixed push (the paused plan's target cells). CSVs + full detail:
  `docs/bench/10gbe-2026-07-05/`, DEVLOG 2026-07-05 00:51.
- **10 GbE benchmark session DONE (2026-07-04/05)** — REV4 sign-off
  data in; owner declarations pending (see Blocked). Push/pull 1 GiB
  ≈ 9.5 of 9.88 Gbit/s; **ue-1 band holds** (1.8×); no organic
  resize (one stream saturates 10 GbE) — ue-2 interpretation call.
  Digest: DEVLOG 2026-07-05 00:34; evidence
  `docs/bench/10gbe-2026-07-05/`.
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
   codex loop per slice (owner re-affirmed). Current: otp-1
   (wire+session contract, doc+proto, no behavior). Then otp-2
   symmetric baseline (needs the 10 GbE rig + zoey-class endpoints).
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

 succeeded in 0ms:
# DECISIONS — settled choices

**Status**: Active

Append-only ledger of decisions that future sessions must not relitigate or miss.
Add entries via the `decision` procedure in `docs/agent/PROTOCOL.md`. Newest last.
When a decision supersedes plan text, the plan text gets edited in the same
session — this file is the index, not a substitute for fixing the doc.

Format:

```
## D-<YYYY-MM-DD>-<n> — <short title>
- Decision: <one line>
- Why: <one line>
- Supersedes: <doc §/decision ID, or "nothing">
```

---

## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.

## D-2026-05-31-2 — Pick-not-Type TUI direction
- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
- Supersedes: TUI_DESIGN.md §6 (portions).

## D-2026-06-04-1 — R3 overrides R2 in the audit chain
- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
- Why: R3 incorporates the GPT R2 critique and severity rebalance.
- Supersedes: conflicting R2 entries.

## D-2026-06-04-2 — Env vars are out for app + diagnostic config
- Decision: no environment-variable configuration carve-out (R3-L39); purge completed via `audit-l39-m27-env-var-purge`.
- Why: owner policy — config surfaces stay explicit.
- Supersedes: nothing (clarifies prior ambiguity).

## D-2026-06-04-3 — Streaming planner ratified, build deferred
- Decision: `greenfield_plan_v6.md` §1.1 (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical but not yet built; multi-slice implementation queued after audit Round 1 (H10b).
- Why: data-loss/DoS hardening takes priority; the plan claim is ratified rather than retired.
- Supersedes: nothing.

## D-2026-06-06-1 — STATE.md precedence model adopted
- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
- Supersedes: "Agent-Specific Expectations" in the previous AGENTS.md (Serena memories as session persistence).

## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
- Decision: `c793df2` (a `git merge -s ours` octopus whose parents are `600023a` + `eafb187` + `d9d4ec7`) stays on `origin/master`; we do **not** rewrite history or force-push to remove it.
- Why: its tree is byte-identical to `600023a` (`git diff 600023a c793df2` is empty) and the workspace builds, so it is cosmetically ugly but harmless; rewriting already-pushed shared history is riskier than the wart. The merge was pushed without owner approval — the corrective is the new AGENTS.md §8 Git-safety contract, not a second unsafe operation.
- Consequence (the trap): because `eafb187` and `d9d4ec7` are now *ancestors* of master, `git branch --merged` falsely reports them merged and a plain `git merge` of either no-ops without landing code. `d9d4ec7` (adaptive-streams-pr3-resizable) does **not** build and its files are not in master's tree. Branch cleanup in this repo is by explicit name only, never `--merged`.
- Supersedes: nothing.

## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
- Supersedes: nothing.

## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
- Supersedes: nothing.

## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).

## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).

## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).

## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*

## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).

## D-2026-06-20-4 — Unified transfer engine plan review freeze
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.

## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*

## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.

## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).

## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).

## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).

## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).

## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.

## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).

## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.

## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).

exec
/usr/bin/zsh -lc "sed -n '1,280p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# One transfer path — direction-invariant by construction

**Status**: Active
**Created**: 2026-07-05
**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
(D-2026-07-05-1). REV4's mixed-version-peers constraint is superseded
outright by **D-2026-07-05-2 (no version compatibility, ever — same
build only)** — annotated in REV4 §Constraints
**Decision ref**: D-2026-07-05-1 (directive + pause);
**D-2026-07-05-4 (Draft → Active, owner "flip the plan and go",
2026-07-05)**

## Directive (owner, 2026-07-05, verbatim)

> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
> ANYTHING EVER using anything else because anything else does not
> exist."

> "just make it so that I NEVER see a situation where pull is faster
> than push or vice versa. that CAN NEVER be possible because of
> something blit did. it should be identical if I start the transfer
> from skippy and push to this machine or if I start the transfer on
> this machine and pull from skippy."

> On benchmark methodology: "tmp on one side, spinning rust on the
> other is not a valid test."

Scope, wire, and process were explicitly delegated to the agent
("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
owner's requirement is the invariant; everything below is the
architecture that makes the invariant impossible to violate rather
than merely maintained by discipline.

## Goal

One `TransferSession` implementation owns every byte transfer blit
performs. A transfer has a SOURCE role and a DESTINATION role; which
end initiated, and which CLI verb was used, select roles — they do not
select code. When this plan ships, the per-direction drivers (client
push driver, daemon push-receive, client pull driver, daemon
pull-send, delegated-pull driver, local orchestration) **do not
exist**: for fixed endpoints and dataset, direction/initiator/verb
cannot affect behavior or wall time by blit's doing, because there is
no second code path to differ.

## Non-goals

- Version compatibility of ANY kind (D-2026-07-05-2, owner standing
  rule: "backward compatibility is NOT a consideration... same build
  only. do not engineer tech debt into an unshipped product"). A blit
  client talks only to a blit-daemon from the same build; the session
  handshake REFUSES a mismatched peer outright. No negotiate-down, no
  advisory fields, no feature-capability bits for version skew.
  `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
  code coexists in-tree during the migration slices solely so each
  slice lands green — that is migration scaffolding, not wire
  compatibility.)
- Making different hardware perform identically. If src and dst sit
  on different disks, the two *data directions* still differ by
  physics; the invariant is that the same data direction between the
  same endpoints is identical regardless of who initiates and which
  verb is used.
- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
- New features. This is a consolidation; capability parity with
  today (mirror, filters, resume, fallback, delegation, progress,
  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
  after cutover, not one of this plan's slices — see the Design note
  on the write-strategy seam.

## Constraints

- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
  violated at the choreography layer.
- **Converge up, not down**: per benchmark cell, the unified session
  must match the better of today's two directions (within ±10% run
  noise), not their average. Unification that slows the fast
  direction fails review.
- REV4 invariants carry: byte-identical results, StallGuard,
  cancellation, byte-accounting. Existing pins are ported (not
  dropped) as tests become role-parameterized; test count never
  drops.
- The sf-2 shape-correction behavior (stream count corrects as the
  need list accumulates) becomes the one and only stream policy —
  both directions inherit it by construction; its pins carry over.
- **The bounded-unilateral dial contract carries unchanged**
  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
  dial, bounded by the byte RECEIVER's advertised capacity profile
  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
  unlimited). The session's role model must express this — profile
  travels DESTINATION→SOURCE at setup regardless of who initiated —
  and otp-1's contract names it explicitly.
- Wire contract discipline (REV4 rule): the unified session's proto —
  messages, field numbers, capability negotiation, transport
  selection — is a reviewed doc+proto slice **before** any behavior
  depends on it.
- Every slice through the codex loop (D-2026-07-04-1); tree green
  after every slice; transitional coexistence of old+new paths is
  scaffolding only — the plan is not Shipped until the deletion slice
  lands and the deletion proof is recorded.
- Windows parity: suite green on the owner's machine + windows-latest
  CI before Shipped.

## Acceptance criteria

- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
      on a symmetric rig (same filesystem class both ends, cold
      caches, disk-to-disk), for each data direction and workload
      (large / 10k-small / mixed): wall time initiating from end A vs
      end B, and via push-verb vs pull-verb, differs only within
      run-to-run noise (±10%). Matrix committed as evidence.
- [ ] **Converge up, measured (codex F4)**: before cutover, the
      corrected symmetric-fs harness records a per-cell baseline of
      the OLD paths, both directions; after cutover, every unified
      cell must be ≤ the better of that cell's two old directions
      + run noise (±10%). A symmetric-but-slower result fails.
- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
      (driver), daemon `push/control.rs` choreography, daemon
      `pull_sync.rs` choreography, the delegated-pull driver, the
      separate local orchestration path, and the `Push`/`PullSync`
      RPCs no longer exist in the tree; one `TransferSession` and one
      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
      as trigger + progress relay — the proof must show it carries no
      payload bytes (codex F3). Recorded file-by-file in the final
      slice's finding doc.
- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
      guard), filters, block-resume, gRPC fallback carrier, delegated
      transfer, progress events, jobs/cancel, read-only enforcement —
      each demonstrated by ported tests on the session.
- [ ] Suite green throughout; final test count ≥ pre-plan baseline
      (1483); all REV4 invariant pins and the sf-2 pin pass
      role-parameterized.
- [ ] Benchmark methodology corrected and recorded: symmetric-fs
      cells are the verdict cells; tmpfs cells remain only as
      explicitly-labeled wire-reference rows (never compared across
      directions with asymmetric endpoints).
- [ ] Windows: full suite green (owner machine) + windows-latest CI.

## Design

**What already is one code** (kept, becomes the session's engine):
`remote/transfer/` — pipeline, sink/source abstractions, data plane,
diff planner, tar-shard, stall guard, progress, `operation_spec` (the
REV4 unified contract), and the engine dial (stream policy incl. sf-2
shape correction). The defect layer is above it: four driver loops
choreograph these pieces differently per direction.

**The one choreography** (roles, not directions):

1. Initiator opens the single bidi `Transfer` RPC and sends the
   operation spec: which end is SOURCE, which is DESTINATION, path/
   module, filters, mirror/resume flags, capabilities.
2. SOURCE enumerates and **streams** its manifest immediately (no
   buffered-enumeration phase — this generalizes push's fast start;
   pull's full-enumeration-then-negotiate slow start is deleted, which
   absorbs the "pull 1s-start" residue item).
3. DESTINATION diffs incrementally against its own filesystem and
   returns need-list batches (one diff owner, always the end that
   owns the target fs — push's proven model; pull_sync's
   source-side diff is deleted).
4. The data plane opens at the dial floor immediately; stream count
   shape-corrects as the need list accumulates (sf-2 mechanism, now
   the only policy, both roles).
5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   the one pipeline into the data plane; DESTINATION writes through
   the one receive path. The receive sink is built with a
   **runtime-selected write-strategy seam**: buffered relay is the
   universal strategy; capability-gated alternatives slot in behind
   it without new paths — the first is zero-copy/splice
   (D-2026-07-05-3, unparked for CPU-bound receivers like the
   owner's UNAS 8 Pro; design input:
   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
   follow-on slice set after cutover. Strategy selection reads
   capability and payload type, never role or initiator.
6. Mirror: DESTINATION computes deletions from the completed source
   manifest it received (filter-scoped, scan-complete-guarded) and
   executes them locally. One rule, no per-direction delete
   choreography.
7. Resume: optional block-hash phase inside the same session, same
   messages regardless of roles.
8. Summary/byte-accounting: one record shape.

**Transport facts vs choreography**: the connection-initiating end
dials TCP data-plane sockets (NAT reality) — byte direction within a
socket is set by role, not by who dialed. The gRPC-fallback lane
becomes a *byte-carrier option* inside the same session (control-
stream frames instead of TCP sockets), selected at negotiation — not
a separate transfer path. Resize keeps its controller-at-sender rule.

**Delegated transfer**: a daemon receiving a delegated request simply
becomes an initiator of the same session against the other daemon
(destination role on its module fs). The bespoke delegated-pull
driver is deleted; the delegation *gate* (authorization) stays. The
`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
(`DelegatedPullProgress` stream) — it never carries payload bytes;
its handler shrinks to "authorize, spawn the session, relay the
session's progress events." It stays wire-compatible or is folded at
cutover — either way the deletion proof asserts no bytes flow
through it (codex F3).

**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
a strictly-ordered block-hash exchange — the DESTINATION's block map
for a file must complete before the SOURCE sends any block of that
file, and stale/mismatched partials fall back to full-file transfer.
This is an explicit exception to the immediate-start rule, exactly as
today's resume path is an explicit single-stream RELIABLE exception
(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
contract; otp-7 pins the stale-partial and mid-resume-failure cases
in tests.

**Local transfers**: the same session driver over an in-process
transport (both roles in one process, no wire). The engine underneath
is already shared; the separate local orchestration path is deleted
in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
guard the migration.

**Affected crates**: `blit-core` (new `transfer_session` module;
`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
`blit-tui` (progress/jobs consume the same events).

**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
mitigated by strangler slices with the tree green throughout and a
non-optional deletion slice. Per-cell regression risk on today's
faster direction — mitigated by the converge-up constraint and
baseline parity pins per slice. Wire break — lockstep upgrade,
owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
Progress/jobs/TUI integration churn — the session emits the existing
event contract (w6-1) at the same boundaries.

## Slices

One coherent, testable change per slice — sized for the `.review/`
loop. Tree green after every slice; old paths keep working until
otp-9 deletes them.

1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   `Transfer` RPC and message set — roles, phases, field numbers,
   the **strict same-build handshake** (exact protocol/build identity
   exchanged at session open; any mismatch is refused with a clear
   error — D-2026-07-05-2; pinned by test when the session lands),
   the receiver capacity profile + bounded-unilateral dial contract
   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
   that exists), transport selection, resume phase ordering (the
   RELIABLE exception above), mirror phase, error/cancel semantics.
   No feature-capability bits: same build implies same features.
   The new proto text must carry NO version-tolerance semantics; the
   capacity profile's absent/0 fields mean "unknown hardware value"
   only, never "old peer" (today's proto comments frame some of that
   contract as old-peer fallback — those comment blocks describe live
   pre-cutover code and die with their messages at otp-10, per the
   D-2026-07-05-2 review adjudication). Codex-reviewed before any
   code consumes it.
2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   cells, cold caches, tmpfs rows re-labeled wire-reference only —
   and record the OLD paths' per-cell, per-direction baseline on the
   rig. This is the converge-up reference the acceptance criteria
   compare against (codex F4).
3. **otp-3 TransferSession core (blit-core)**: role-parameterized
   state machine over the existing engine with an in-process
   transport; unit/e2e tests run BOTH role assignments over the same
   fixtures — the invariance property enters the test suite here.
4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
   (remote push-equivalent rides the session); A/B parity pins vs
   old push (byte-identical trees, summary parity, sf-2 pin ported).
5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   equivalent) — the same code with roles flipped; the parity suite
   reruns with no per-direction test code.
6. **otp-6 mirror + filters** on the session (one delete rule).
7. **otp-7 resume** block phase (ordering + stale-partial pins per
   the Design's RELIABLE exception).
8. **otp-8 fallback byte-carrier** (control-stream frames) as the
   session's alternate transport.
9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
   delegated-pull driver retired behind the existing gate;

exec
/usr/bin/zsh -lc "sed -n '1,220p' /home/michael/.codex/RTK.md" in /home/michael/dev/Blit
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

codex
I found the imported RTK guidance after the initial reads, so I’ll prefix subsequent shell commands with `rtk`. The review itself still stays read-only: no edits, no verification run unless the workspace permits it.
exec
/usr/bin/zsh -lc 'rtk git show --name-status --no-renames a3e2acb' in /home/michael/dev/Blit
 exited 128 in 0ms:
fatal: options '--name-only', '--name-status', '--check', and '-s' cannot be used together


exec
/usr/bin/zsh -lc "rtk sed -n '280,620p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
 succeeded in 0ms:
   delegated-pull driver retired behind the existing gate;
   `DelegatedPull` RPC reduced to trigger + progress relay.
10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
    operation through the session; `Push`/`PullSync` and all four
    drivers deleted from the tree and the proto, no bridge
    (D-2026-07-05-2); ported-test accounting proves count never
    dropped. Deletion proof recorded, incl. the DelegatedPull
    no-payload-bytes assertion.
11. **otp-11 local transfers** ride the in-process transport; the
    separate local orchestration is deleted; local perf pins hold.
12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
    on the unified path — initiator/verb invariance A/B within noise
    AND every cell ≤ the better old direction + noise; committed as
    this plan's acceptance evidence.
13. **otp-13 verdict**: acceptance checklist walked with the owner;
    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
    against the unified baseline — owner call at that point.

## Open questions

- None requiring owner input now — scope, wire, and process were
  delegated (Directive section). Slice-level unknowns (exact proto
  shapes, resume edge semantics, TUI event wiring) are settled inside
  their slices through the codex loop. — owner

exec
/usr/bin/zsh -lc 'rtk git show --no-ext-diff --unified=80 --no-renames a3e2acb -- docs/TRANSFER_SESSION.md' in /home/michael/dev/Blit
 succeeded in 0ms:
commit a3e2acb651ae8893127f2d75835d047695e4154c
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 12:29:54 2026 -0400

    otp-1: unified Transfer session wire contract (doc + proto + stubs)
    
    docs/TRANSFER_SESSION.md is the authoritative contract: one
    role-tagged frame vocabulary both wire directions, exact-match
    same-build handshake first (D-2026-07-05-2), destination-owned diff,
    dial contract carried (D-2026-06-20-1/-2), sf-2 shape correction as
    the only stream policy, resume as a strictly-ordered RELIABLE
    exception, mirror destination-local. proto gains the Transfer RPC +
    session messages (20-arm TransferFrame reusing the engine payload
    vocabulary; no PeerCapabilities, no spec_version, no delete list).
    Handler + five test fakes stub UNIMPLEMENTED; in-process pin proves
    the RPC is reachable and refusing. Suite 1483 -> 1484.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

diff --git a/docs/TRANSFER_SESSION.md b/docs/TRANSFER_SESSION.md
new file mode 100644
index 0000000..b10d368
--- /dev/null
+++ b/docs/TRANSFER_SESSION.md
@@ -0,0 +1,184 @@
+# TransferSession wire + session contract (otp-1)
+
+**Status**: Active (contract; implementation lands otp-3..otp-10)
+**Created**: 2026-07-05
+**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
+**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
+(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
+
+This document is the authoritative contract for the single `Transfer`
+RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
+truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
+session"; this doc explains the state machine the proto cannot.
+
+## Invariants
+
+1. **One vocabulary, role-tagged.** Both wire directions carry the
+   same frame type (`TransferFrame`). Which frames an end may send is
+   determined by its ROLE (SOURCE or DESTINATION), never by whether
+   it is the gRPC client or server. This is the structural form of
+   the owner's invariant: there is no push-shaped or pull-shaped
+   message set to diverge.
+2. **Same build only (D-2026-07-05-2).** The first frame each way is
+   `SessionHello{build_id, contract_version}`. Both ends compare for
+   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
+   naming both ids, then stream close. No negotiate-down, no advisory
+   fields, no feature-capability bits — same build implies same
+   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
+   composed at compile time; `contract_version` is a belt-and-braces
+   integer bumped on any wire-shape change (exact match required).
+3. **Roles.** The initiator (the end that opened the RPC — a CLI
+   client, or a daemon acting as delegated initiator) declares in
+   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
+   (always a daemon) takes the other role. All four
+   initiator/role combinations run the identical state machine.
+4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
+   from live enumeration (immediate start — no buffered-enumeration
+   phase in any direction). DESTINATION diffs incrementally against
+   its own filesystem and streams need batches back. DESTINATION is
+   authoritative for what it has; SOURCE is authoritative for what
+   exists to send.
+5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
+   (whichever end holds DESTINATION) advertises its
+   `CapacityProfile` at session open — in `SessionOpen` when the
+   initiator is DESTINATION, in `SessionAccept` when the responder
+   is. The byte SENDER (SOURCE) owns the live dial bounded by that
+   profile. Absent/0 profile fields mean "unknown hardware value" —
+   conservative defaults, never unlimited, and NEVER "old peer"
+   (there are no old peers).
+6. **One stream policy.** The data plane opens at the dial floor
+   immediately; SOURCE shape-corrects the stream count upward via
+   resize as the need list accumulates (the sf-2 mechanism —
+   `TransferDial::propose_shape_resize` — now the only policy).
+   SOURCE is the resize controller in every session.
+
+## Phase state machine
+
+```
+INITIATOR                                RESPONDER
+  |-- SessionHello ----------------------->|   (phase: HELLO)
+  |<------------------------ SessionHello--|
+  |     both verify build_id exact match; mismatch => SessionError + close
+  |-- SessionOpen ------------------------>|   (phase: OPEN)
+  |<---------------------- SessionAccept --|
+  |     responder validates module/path/read-only/gate here;
+  |     refusal is a SessionError, never a silent close
+  |                                        |
+  |==== manifest + need + payload phases run CONCURRENTLY =========|
+  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
+  |  DEST streams:    NeedBatch* ... NeedComplete                  |
+  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
+  |                   frames when the in-stream carrier is chosen) |
+  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
+  |                                                                 |
+  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
+  |  `resume=true` is followed by DEST's BlockHashList for that     |
+  |  file BEFORE SOURCE may send any byte of that file; stale or    |
+  |  mismatched partials fall back to full-file transfer.           |
+  |                                                                 |
+  |  mirror: DEST computes deletions LOCALLY from the completed     |
+  |  source manifest (filter-scoped, scan-complete-guarded) and     |
+  |  executes them itself. No delete list crosses the wire.         |
+  |                                                                 |
+  |-- SourceDone (all payloads flushed) -->|   (phase: CLOSING)
+  |<---------------- TransferSummary ------|   (DEST is the scorer)
+  |     stream close                        |
+```
+
+- Phase violations (a frame arriving in a phase where its role may
+  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
+  fail-fast, no tolerant parsing.
+- `NeedComplete` is DESTINATION's promise that no further need
+  batches follow (SOURCE may finish after flushing what was asked).
+- `TransferSummary` always travels DESTINATION → SOURCE (the end
+  that wrote bytes and executed deletes is the end that can attest
+  to them), then the initiator surfaces it to the operator.
+
+## Frame set and field numbers
+
+`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
+
+`TransferFrame.frame` oneof (field numbers frozen by this doc):
+
+| # | frame | sender | phase |
+|---|-------|--------|-------|
+| 1 | `SessionHello` | both, first frame | HELLO |
+| 2 | `SessionOpen` | initiator | OPEN |
+| 3 | `SessionAccept` | responder | OPEN |
+| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
+| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
+| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
+| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
+| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
+| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
+| 10 | `FileData file_data` | SOURCE | in-stream carrier |
+| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
+| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
+| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
+| 14 | `BlockTransfer block` | SOURCE | resume |
+| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
+| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
+| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
+| 18 | `SourceDone source_done` | SOURCE | closing |
+| 19 | `TransferSummary summary` | DESTINATION | closing |
+| 20 | `SessionError error` | both | any |
+
+Reused messages (`FileHeader`, `FileData`, `TarShard*`,
+`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
+`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
+`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
+existing shapes — the session reuses the engine's payload vocabulary
+verbatim. New messages (`SessionHello`, `SessionOpen`,
+`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
+`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
+defined in the proto with their field numbers.
+
+Deliberately absent: `PeerCapabilities` (same build = same
+features), `spec_version` negotiation (the hello's exact match
+replaces it), any delete list (mirror is destination-local), any
+push/pull-specific message.
+
+## Transport selection
+
+- **TCP data plane (default):** the RESPONDER binds the listener and
+  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
+  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
+  dials (NAT/firewall reality — connection topology, not
+  choreography). Byte direction on the sockets is set by role:
+  SOURCE writes, DESTINATION reads. Resize ADD epochs arm one
+  accept per `sub_token`, exactly as ue-r2-2 built.
+- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
+  (operator `--force-grpc` diagnostics) or granted by the responder
+  when it cannot bind a data plane (`SessionAccept` with no grant).
+  Payload frames 9-15 ride the RPC itself. Same choreography, same
+  planner decisions, different byte carrier.
+- **Local (in-process):** the identical session state machine runs
+  with both roles in one process over an in-process frame channel —
+  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
+  file vs block) is planner-owned and reads workload shape +
+  capability, never role/initiator/transport.
+
+## Errors, cancel, stall
+
+- `SessionError{code, message, detail}` codes:
+  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
+  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
+  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
+  or aborts says why before closing; operators never diagnose from a
+  bare stream reset.
+- `CancelJob` interop: the responder registers the session in
+  `ActiveJobs` at OPEN (same transfer_id contract as today); the
+  cancel token races the session exactly as w4-3 wired, and the
+  peer receives `SessionError{CANCELLED}`.
+- StallGuard, byte-accounting, and progress events (w6-1 contract)
+  attach at the same boundaries they do today; the session emits the
+  existing `DaemonEvent` payloads.
+
+## What this replaces
+
+At cutover (otp-10): `Push`, `PullSync`, and their message
+choreographies are deleted from the proto and the tree; the four
+per-direction drivers die with them; `DelegatedPull` shrinks to
+trigger + progress relay (no payload bytes). Until then this
+contract's surface exists compiled-but-refusing
+(`Transfer` returns `UNIMPLEMENTED`; pinned by test).


exec
/usr/bin/zsh -lc 'rtk git show --no-ext-diff --unified=80 --no-renames a3e2acb -- proto/blit.proto' in /home/michael/dev/Blit
 succeeded in 0ms:
commit a3e2acb651ae8893127f2d75835d047695e4154c
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 12:29:54 2026 -0400

    otp-1: unified Transfer session wire contract (doc + proto + stubs)
    
    docs/TRANSFER_SESSION.md is the authoritative contract: one
    role-tagged frame vocabulary both wire directions, exact-match
    same-build handshake first (D-2026-07-05-2), destination-owned diff,
    dial contract carried (D-2026-06-20-1/-2), sf-2 shape correction as
    the only stream policy, resume as a strictly-ordered RELIABLE
    exception, mirror destination-local. proto gains the Transfer RPC +
    session messages (20-arm TransferFrame reusing the engine payload
    vocabulary; no PeerCapabilities, no spec_version, no delete list).
    Handler + five test fakes stub UNIMPLEMENTED; in-process pin proves
    the RPC is reachable and refusing. Suite 1483 -> 1484.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

diff --git a/proto/blit.proto b/proto/blit.proto
index a7b639e..ebeb5a9 100644
--- a/proto/blit.proto
+++ b/proto/blit.proto
@@ -1,88 +1,94 @@
 syntax = "proto3";
 package blit.v2;
 
 // The main service for all data transfer and remote management operations.
 service Blit {
   // Push uses a bidirectional stream for an efficient "check-then-send" workflow.
+  // (Deleted at ONE_TRANSFER_PATH cutover, otp-10 — replaced by Transfer.)
   rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
 
+  // ONE_TRANSFER_PATH (otp-1): the single role-tagged transfer session
+  // that replaces Push and PullSync at cutover. Contract:
+  // docs/TRANSFER_SESSION.md. UNIMPLEMENTED until otp-3/otp-4.
+  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
+
   // Removed 2026-07-03 (ue-r2-1h): rpc Pull(PullRequest) returns
   // (stream PullChunk) — the deprecated server-streaming pull.
   // Superseded whole by PullSync; the relay client's metadata scan and
   // single-file streaming moved to PullSync sessions
   // (TransferOperationSpec.metadata_only / a single-file force_grpc
   // spec). PullRequest/PullChunk were deleted with it (they had no
   // other referents); PullSummary, ManifestBatch, FileHeader, FileData,
   // and DataTransferNegotiation survive — PullSync and push share them.
 
   // Bidirectional pull with manifest comparison for selective transfers.
   // Client sends local manifest, server compares and sends only needed files.
   rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage);
 
   // Lists contents of a remote directory.
   rpc List(ListRequest) returns (ListResponse);
 
   // Deletes files/directories on the server for mirror operations.
   rpc Purge(PurgeRequest) returns (PurgeResponse);
 
   // Provides path completion suggestions for a given remote path prefix.
   rpc CompletePath(CompletionRequest) returns (CompletionResponse);
 
   // Lists the available modules on the server.
   rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
 
   // Recursively finds files/directories starting at a module path.
   rpc Find(FindRequest) returns (stream FindEntry);
 
   // Summarises disk usage for a subtree (du-style).
   rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
 
   // Reports module/storage capacity information (df-style).
   rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
 
   // Destination-side delegated initiator. The CLI calls this on the
   // destination daemon when both endpoints in a `blit copy` are
   // remote. The destination daemon validates the request through the
   // delegation gate, opens its own pull against the named source, and
   // streams progress/results back to the CLI. Bytes flow source→dst
   // directly; the CLI is not in the byte path.
   //
   // See docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md for the design
   // (gate ordering, allowlist semantics, client_capabilities
   // override boundary).
   rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);
 
   // Daemon-state snapshot for the TUI's F1 (Daemons) and F2
   // (Transfers) panes, plus `blit jobs list <remote>`. Always
   // available regardless of `--metrics`; see §6.4 of
   // docs/plan/TUI_DESIGN.md. Counters read from the
   // TransferMetrics atomics (so they're zero when the flag is
   // off), but active[] / recent[] always populate from the
   // always-on ActiveJobs table introduced in milestone B.
   rpc GetState(GetStateRequest) returns (DaemonState);
 
   // Cancel a daemon-side in-flight transfer by transfer_id.
   // M-Jobs of docs/plan/TUI_DESIGN.md §6.5. Every kind that can
   // hold an active row — push, pull_sync, and delegated pull —
   // supports cancellation (D-2026-07-04-3): the daemon fires the
   // row's cancellation token and the dispatcher races it, so an
   // attached transfer tears down promptly and its still-connected
   // client receives a terminal CANCELLED status.
   //
   // Status semantics:
   //   OK                    → the cancellation token was fired;
   //                           the handler will tear down on its
   //                           next .await resolve.
   //   NOT_FOUND             → no active transfer matches the
   //                           requested transfer_id (already
   //                           completed, or never existed).
   //   FAILED_PRECONDITION   → the transfer exists but its kind's
   //                           dispatch policy gates cancellation
   //                           off. Only the history-only pull kind
   //                           (RPC deleted) is gated, and no active
   //                           row of that kind can exist — a
   //                           contract escape hatch, not an
   //                           expected outcome.
   rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
 
   // Clear the daemon's recent-transfers list (GetState.recent[]).
@@ -1129,80 +1135,237 @@ message TransferStarted {
   string transfer_id = 1;
   TransferKind kind = 2;
   // `<ip>:<port>` of the connecting peer, or "unknown" when the
   // transport didn't surface one (in-process tests).
   string peer = 3;
   // Module name on the daemon. Empty for streaming RPCs at
   // registration time — populated by GetState.active[] once the
   // first stream frame parses.
   string module = 4;
   // Module-relative path. Same "empty until first frame" caveat
   // as `module`.
   string path = 5;
   // Unix milliseconds at which the active row was registered.
   uint64 start_unix_ms = 6;
 }
 
 // Periodic chunk-granular progress for an in-flight transfer. The
 // daemon fires one TransferProgress per active row per tick (default
 // 10 Hz, see DEFAULT_PROGRESS_TICK_MS in service/core.rs) so a
 // subscribed TUI can render a live byte-level progress bar without
 // polling GetState.
 //
 // The bytes/files totals are absolute (cumulative since
 // TransferStarted), not deltas — consumers that want per-tick rates
 // subtract the previous observation. `throughput_bps` is the
 // daemon-computed instantaneous rate over the most recent tick
 // interval; a smoothed EWMA is a follow-up slice once we have an
 // operator pain signal to justify the extra state.
 message TransferProgress {
   string transfer_id = 1;
   // Cumulative bytes that landed on disk (same source as
   // GetState.active[].bytes_completed).
   uint64 bytes_completed = 2;
   // Total bytes expected, when known. 0 until a future C
   // sub-slice wires it from the manifest stage.
   uint64 bytes_total = 3;
   // Files completed; 0 until the files-counter slice wires it.
   uint64 files_completed = 4;
   // Total files expected; 0 until the manifest stage wires it.
   uint64 files_total = 5;
   // Instantaneous bytes-per-second over the most recent tick.
   // Future slice may replace with an EWMA so subscribers see
   // smoothed values; field name + semantic are forward-stable.
   uint64 throughput_bps = 6;
 }
 
 // Fired when a transfer drains successfully (`ActiveJobGuard`
 // dropped with `record_outcome(ok=true, ...)`). Carries the
 // terminal byte/file totals and the wall-clock duration. Pairs
 // with `TransferStarted` to bracket a transfer's lifetime on the
 // event stream.
 message TransferComplete {
   string transfer_id = 1;
   // Cumulative bytes that landed on disk. Sourced from the
   // per-row atomic that c-1a/c-1b wired (the same value
   // `TransferRecord.bytes` carries at GetState.recent[]).
   uint64 bytes = 2;
   // Files completed. Always zero until a future C sub-slice
   // wires the files counter; field reserved here so subscribers
   // don't need a proto roll to render it.
   uint64 files = 3;
   uint64 duration_ms = 4;
   // True when the transfer fell back to the gRPC control plane
   // because the data plane couldn't be reserved. Always false
   // in this slice — a future C sub-slice plumbs the bit through
   // from the handler's result.
   bool tcp_fallback_used = 5;
 }
 
 // Fired when a transfer drains in a failure state
 // (`ActiveJobGuard` dropped with `record_outcome(ok=false, ...)`
 // or with no outcome recorded, e.g. spawn-task cancellation).
 message TransferError {
   string transfer_id = 1;
   // Handler's failure message. Empty when the outcome was
   // recorded with no message; "cancelled before outcome
   // recorded" when the spawn task didn't reach the
   // record_outcome call.
   string message = 2;
 }
+
+// ═══════════════════════════════════════════════════════════════════
+// ONE_TRANSFER_PATH unified session (otp-1 wire contract).
+// Authoritative state machine + frame table: docs/TRANSFER_SESSION.md.
+// Compiled-but-refusing until otp-3/otp-4 land behavior (the Transfer
+// handler returns UNIMPLEMENTED, pinned by test). Replaces Push and
+// PullSync whole at cutover (otp-10, D-2026-07-05-1); no bridge
+// (D-2026-07-05-2: same-build peers only).
+// ═══════════════════════════════════════════════════════════════════
+
+// A transfer has a SOURCE role and a DESTINATION role. Which end
+// initiated, and which CLI verb was used, select roles — never code.
+enum TransferRole {
+  TRANSFER_ROLE_UNSPECIFIED = 0;
+  TRANSFER_ROLE_SOURCE = 1;
+  TRANSFER_ROLE_DESTINATION = 2;
+}
+
+// First frame BOTH directions. Exact-match same-build handshake
+// (D-2026-07-05-2): any inequality in either field is a
+// SessionError{BUILD_MISMATCH} naming both ids, then close. No
+// negotiate-down, no advisory fields, no capability bits.
+message SessionHello {
+  // "<crate version>+<git commit>[.dirty]", composed at compile time.
+  string build_id = 1;
+  // Bumped on any wire-shape change; exact match required.
+  uint32 contract_version = 2;
+}
+
+// Initiator's second frame: the whole operation, roles included.
+message SessionOpen {
+  // Role the INITIATOR takes; the responder takes the other.
+  TransferRole initiator_role = 1;
+  // Responder-side module (empty = default root export) and path
+  // within it. The initiator-side path never crosses the wire — the
+  // initiator owns its local endpoint.
+  string module = 2;
+  string path = 3;
+  FilterSpec filter = 4;
+  ComparisonMode compare_mode = 5;
+  // Mirror is explicit: enabled + scope. No implicit mirror.
+  bool mirror_enabled = 6;
+  MirrorMode mirror_kind = 7;
+  ResumeSettings resume = 8;
+  // Request the in-stream byte carrier (diagnostics / unreachable
+  // data-plane environments). The responder may also force it via a
+  // grant-less SessionAccept when it cannot bind a listener.
+  bool in_stream_bytes = 9;
+  bool ignore_existing = 10;
+  bool require_complete_scan = 11;
+  // Set iff the initiator is DESTINATION (dial contract: the byte
+  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
+  // unknown hardware value, conservative, never "old peer").
+  CapacityProfile receiver_capacity = 12;
+}
+
+// Responder's reply. Refusals are SessionError frames, never silent
+// closes.
+message SessionAccept {
+  // Set iff the responder is DESTINATION.
+  CapacityProfile receiver_capacity = 1;
+  // Absent = in-stream carrier (requested, or listener bind failed).
+  DataPlaneGrant data_plane = 2;
+}
+
+// TCP data-plane grant. The RESPONDER always binds; the INITIATOR
+// always dials (connection topology, not choreography — byte
+// direction on the sockets is set by role: SOURCE writes).
+message DataPlaneGrant {
+  uint32 tcp_port = 1;
+  bytes session_token = 2;
+  // Dial floor. SOURCE shape-corrects upward via resize (sf-2
+  // mechanism — the only stream policy).
+  uint32 initial_streams = 3;
+  // Resize is always available (same build): epoch-0 credential.
+  bytes epoch0_sub_token = 4;
+}
+
+// DESTINATION → SOURCE: files the destination wants, in batches.
+message NeedEntry {
+  string relative_path = 1;
+  // RELIABLE resume exception (docs/TRANSFER_SESSION.md): when true,
+  // the destination's BlockHashList for this file follows, and the
+  // source must not send any byte of this file before receiving it;
+  // stale/mismatched partials fall back to full-file transfer.
+  bool resume = 2;
+}
+message NeedBatch {
+  repeated NeedEntry entries = 1;
+}
+// DESTINATION's promise that no further NeedBatch follows.
+message NeedComplete {}
+
+// SOURCE's promise that every requested payload byte is flushed.
+message SourceDone {}
+
+// DESTINATION → SOURCE at close: the end that wrote bytes and
+// executed deletes attests to the outcome (one summary shape for
+// every direction; replaces PushSummary/PullSummary at cutover).
+message TransferSummary {
+  uint64 files_transferred = 1;
+  uint64 bytes_transferred = 2;
+  uint64 entries_deleted = 3;   // mirror executed destination-local
+  bool in_stream_carrier_used = 4;
+  uint64 files_resumed = 5;
+}
+
+// Structured refusal/abort — an end says why before closing.
+message SessionError {
+  enum Code {
+    SESSION_ERROR_UNSPECIFIED = 0;
+    BUILD_MISMATCH = 1;
+    MODULE_UNKNOWN = 2;
+    READ_ONLY = 3;
+    DELEGATION_REFUSED = 4;
+    SCAN_INCOMPLETE = 5;
+    PROTOCOL_VIOLATION = 6;
+    DATA_PLANE_FAILED = 7;
+    CANCELLED = 8;
+    INTERNAL = 9;
+  }
+  Code code = 1;
+  string message = 2;
+  // BUILD_MISMATCH: both build ids, so the operator sees exactly
+  // which end is stale.
+  string local_build_id = 3;
+  string peer_build_id = 4;
+}
+
+// The single frame type BOTH wire directions carry. Which frames an
+// end may send is determined by ROLE and phase
+// (docs/TRANSFER_SESSION.md frame table — field numbers frozen
+// there). Reuses the engine's existing payload vocabulary verbatim.
+message TransferFrame {
+  oneof frame {
+    SessionHello hello = 1;
+    SessionOpen open = 2;
+    SessionAccept accept = 3;
+    FileHeader manifest_entry = 4;
+    ManifestComplete manifest_complete = 5;
+    NeedBatch need_batch = 6;
+    NeedComplete need_complete = 7;
+    BlockHashList block_hashes = 8;
+    FileHeader file_begin = 9;
+    FileData file_data = 10;
+    TarShardHeader tar_shard_header = 11;
+    TarShardChunk tar_shard_chunk = 12;
+    TarShardComplete tar_shard_complete = 13;
+    BlockTransfer block = 14;
+    BlockTransferComplete block_complete = 15;
+    DataPlaneResize resize = 16;
+    DataPlaneResizeAck resize_ack = 17;
+    SourceDone source_done = 18;
+    TransferSummary summary = 19;
+    SessionError error = 20;
+  }
+}


codex
The added contract is substantial and mostly centered in `docs/TRANSFER_SESSION.md` plus the new proto block. I’m now checking the reused message shapes and the stub/test changes, because the biggest risk for otp-1 is a contract that compiles but leaves otp-3/otp-4 inventing rules.
exec
/usr/bin/zsh -lc "rtk nl -ba docs/TRANSFER_SESSION.md | rtk sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# TransferSession wire + session contract (otp-1)
     2	
     3	**Status**: Active (contract; implementation lands otp-3..otp-10)
     4	**Created**: 2026-07-05
     5	**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
     6	**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
     7	(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
     8	
     9	This document is the authoritative contract for the single `Transfer`
    10	RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
    11	truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
    12	session"; this doc explains the state machine the proto cannot.
    13	
    14	## Invariants
    15	
    16	1. **One vocabulary, role-tagged.** Both wire directions carry the
    17	   same frame type (`TransferFrame`). Which frames an end may send is
    18	   determined by its ROLE (SOURCE or DESTINATION), never by whether
    19	   it is the gRPC client or server. This is the structural form of
    20	   the owner's invariant: there is no push-shaped or pull-shaped
    21	   message set to diverge.
    22	2. **Same build only (D-2026-07-05-2).** The first frame each way is
    23	   `SessionHello{build_id, contract_version}`. Both ends compare for
    24	   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
    25	   naming both ids, then stream close. No negotiate-down, no advisory
    26	   fields, no feature-capability bits — same build implies same
    27	   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
    28	   composed at compile time; `contract_version` is a belt-and-braces
    29	   integer bumped on any wire-shape change (exact match required).
    30	3. **Roles.** The initiator (the end that opened the RPC — a CLI
    31	   client, or a daemon acting as delegated initiator) declares in
    32	   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
    33	   (always a daemon) takes the other role. All four
    34	   initiator/role combinations run the identical state machine.
    35	4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
    36	   from live enumeration (immediate start — no buffered-enumeration
    37	   phase in any direction). DESTINATION diffs incrementally against
    38	   its own filesystem and streams need batches back. DESTINATION is
    39	   authoritative for what it has; SOURCE is authoritative for what
    40	   exists to send.
    41	5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
    42	   (whichever end holds DESTINATION) advertises its
    43	   `CapacityProfile` at session open — in `SessionOpen` when the
    44	   initiator is DESTINATION, in `SessionAccept` when the responder
    45	   is. The byte SENDER (SOURCE) owns the live dial bounded by that
    46	   profile. Absent/0 profile fields mean "unknown hardware value" —
    47	   conservative defaults, never unlimited, and NEVER "old peer"
    48	   (there are no old peers).
    49	6. **One stream policy.** The data plane opens at the dial floor
    50	   immediately; SOURCE shape-corrects the stream count upward via
    51	   resize as the need list accumulates (the sf-2 mechanism —
    52	   `TransferDial::propose_shape_resize` — now the only policy).
    53	   SOURCE is the resize controller in every session.
    54	
    55	## Phase state machine
    56	
    57	```
    58	INITIATOR                                RESPONDER
    59	  |-- SessionHello ----------------------->|   (phase: HELLO)
    60	  |<------------------------ SessionHello--|
    61	  |     both verify build_id exact match; mismatch => SessionError + close
    62	  |-- SessionOpen ------------------------>|   (phase: OPEN)
    63	  |<---------------------- SessionAccept --|
    64	  |     responder validates module/path/read-only/gate here;
    65	  |     refusal is a SessionError, never a silent close
    66	  |                                        |
    67	  |==== manifest + need + payload phases run CONCURRENTLY =========|
    68	  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
    69	  |  DEST streams:    NeedBatch* ... NeedComplete                  |
    70	  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
    71	  |                   frames when the in-stream carrier is chosen) |
    72	  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
    73	  |                                                                 |
    74	  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
    75	  |  `resume=true` is followed by DEST's BlockHashList for that     |
    76	  |  file BEFORE SOURCE may send any byte of that file; stale or    |
    77	  |  mismatched partials fall back to full-file transfer.           |
    78	  |                                                                 |
    79	  |  mirror: DEST computes deletions LOCALLY from the completed     |
    80	  |  source manifest (filter-scoped, scan-complete-guarded) and     |
    81	  |  executes them itself. No delete list crosses the wire.         |
    82	  |                                                                 |
    83	  |-- SourceDone (all payloads flushed) -->|   (phase: CLOSING)
    84	  |<---------------- TransferSummary ------|   (DEST is the scorer)
    85	  |     stream close                        |
    86	```
    87	
    88	- Phase violations (a frame arriving in a phase where its role may
    89	  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
    90	  fail-fast, no tolerant parsing.
    91	- `NeedComplete` is DESTINATION's promise that no further need
    92	  batches follow (SOURCE may finish after flushing what was asked).
    93	- `TransferSummary` always travels DESTINATION → SOURCE (the end
    94	  that wrote bytes and executed deletes is the end that can attest
    95	  to them), then the initiator surfaces it to the operator.
    96	
    97	## Frame set and field numbers
    98	
    99	`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
   100	
   101	`TransferFrame.frame` oneof (field numbers frozen by this doc):
   102	
   103	| # | frame | sender | phase |
   104	|---|-------|--------|-------|
   105	| 1 | `SessionHello` | both, first frame | HELLO |
   106	| 2 | `SessionOpen` | initiator | OPEN |
   107	| 3 | `SessionAccept` | responder | OPEN |
   108	| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
   109	| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
   110	| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
   111	| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
   112	| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
   113	| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
   114	| 10 | `FileData file_data` | SOURCE | in-stream carrier |
   115	| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
   116	| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
   117	| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
   118	| 14 | `BlockTransfer block` | SOURCE | resume |
   119	| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
   120	| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
   121	| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
   122	| 18 | `SourceDone source_done` | SOURCE | closing |
   123	| 19 | `TransferSummary summary` | DESTINATION | closing |
   124	| 20 | `SessionError error` | both | any |
   125	
   126	Reused messages (`FileHeader`, `FileData`, `TarShard*`,
   127	`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
   128	`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
   129	`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
   130	existing shapes — the session reuses the engine's payload vocabulary
   131	verbatim. New messages (`SessionHello`, `SessionOpen`,
   132	`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
   133	`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
   134	defined in the proto with their field numbers.
   135	
   136	Deliberately absent: `PeerCapabilities` (same build = same
   137	features), `spec_version` negotiation (the hello's exact match
   138	replaces it), any delete list (mirror is destination-local), any
   139	push/pull-specific message.
   140	
   141	## Transport selection
   142	
   143	- **TCP data plane (default):** the RESPONDER binds the listener and
   144	  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
   145	  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
   146	  dials (NAT/firewall reality — connection topology, not
   147	  choreography). Byte direction on the sockets is set by role:
   148	  SOURCE writes, DESTINATION reads. Resize ADD epochs arm one
   149	  accept per `sub_token`, exactly as ue-r2-2 built.
   150	- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
   151	  (operator `--force-grpc` diagnostics) or granted by the responder
   152	  when it cannot bind a data plane (`SessionAccept` with no grant).
   153	  Payload frames 9-15 ride the RPC itself. Same choreography, same
   154	  planner decisions, different byte carrier.
   155	- **Local (in-process):** the identical session state machine runs
   156	  with both roles in one process over an in-process frame channel —
   157	  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
   158	  file vs block) is planner-owned and reads workload shape +
   159	  capability, never role/initiator/transport.
   160	
   161	## Errors, cancel, stall
   162	
   163	- `SessionError{code, message, detail}` codes:
   164	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
   165	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
   166	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
   167	  or aborts says why before closing; operators never diagnose from a
   168	  bare stream reset.
   169	- `CancelJob` interop: the responder registers the session in
   170	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
   171	  cancel token races the session exactly as w4-3 wired, and the
   172	  peer receives `SessionError{CANCELLED}`.
   173	- StallGuard, byte-accounting, and progress events (w6-1 contract)
   174	  attach at the same boundaries they do today; the session emits the
   175	  existing `DaemonEvent` payloads.
   176	
   177	## What this replaces
   178	
   179	At cutover (otp-10): `Push`, `PullSync`, and their message
   180	choreographies are deleted from the proto and the tree; the four
   181	per-direction drivers die with them; `DelegatedPull` shrinks to
   182	trigger + progress relay (no payload bytes). Until then this
   183	contract's surface exists compiled-but-refusing
   184	(`Transfer` returns `UNIMPLEMENTED`; pinned by test).

exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '1,130p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	syntax = "proto3";
     2	package blit.v2;
     3	
     4	// The main service for all data transfer and remote management operations.
     5	service Blit {
     6	  // Push uses a bidirectional stream for an efficient "check-then-send" workflow.
     7	  // (Deleted at ONE_TRANSFER_PATH cutover, otp-10 — replaced by Transfer.)
     8	  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
     9	
    10	  // ONE_TRANSFER_PATH (otp-1): the single role-tagged transfer session
    11	  // that replaces Push and PullSync at cutover. Contract:
    12	  // docs/TRANSFER_SESSION.md. UNIMPLEMENTED until otp-3/otp-4.
    13	  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
    14	
    15	  // Removed 2026-07-03 (ue-r2-1h): rpc Pull(PullRequest) returns
    16	  // (stream PullChunk) — the deprecated server-streaming pull.
    17	  // Superseded whole by PullSync; the relay client's metadata scan and
    18	  // single-file streaming moved to PullSync sessions
    19	  // (TransferOperationSpec.metadata_only / a single-file force_grpc
    20	  // spec). PullRequest/PullChunk were deleted with it (they had no
    21	  // other referents); PullSummary, ManifestBatch, FileHeader, FileData,
    22	  // and DataTransferNegotiation survive — PullSync and push share them.
    23	
    24	  // Bidirectional pull with manifest comparison for selective transfers.
    25	  // Client sends local manifest, server compares and sends only needed files.
    26	  rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage);
    27	
    28	  // Lists contents of a remote directory.
    29	  rpc List(ListRequest) returns (ListResponse);
    30	
    31	  // Deletes files/directories on the server for mirror operations.
    32	  rpc Purge(PurgeRequest) returns (PurgeResponse);
    33	
    34	  // Provides path completion suggestions for a given remote path prefix.
    35	  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
    36	
    37	  // Lists the available modules on the server.
    38	  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
    39	
    40	  // Recursively finds files/directories starting at a module path.
    41	  rpc Find(FindRequest) returns (stream FindEntry);
    42	
    43	  // Summarises disk usage for a subtree (du-style).
    44	  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
    45	
    46	  // Reports module/storage capacity information (df-style).
    47	  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
    48	
    49	  // Destination-side delegated initiator. The CLI calls this on the
    50	  // destination daemon when both endpoints in a `blit copy` are
    51	  // remote. The destination daemon validates the request through the
    52	  // delegation gate, opens its own pull against the named source, and
    53	  // streams progress/results back to the CLI. Bytes flow source→dst
    54	  // directly; the CLI is not in the byte path.
    55	  //
    56	  // See docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md for the design
    57	  // (gate ordering, allowlist semantics, client_capabilities
    58	  // override boundary).
    59	  rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);
    60	
    61	  // Daemon-state snapshot for the TUI's F1 (Daemons) and F2
    62	  // (Transfers) panes, plus `blit jobs list <remote>`. Always
    63	  // available regardless of `--metrics`; see §6.4 of
    64	  // docs/plan/TUI_DESIGN.md. Counters read from the
    65	  // TransferMetrics atomics (so they're zero when the flag is
    66	  // off), but active[] / recent[] always populate from the
    67	  // always-on ActiveJobs table introduced in milestone B.
    68	  rpc GetState(GetStateRequest) returns (DaemonState);
    69	
    70	  // Cancel a daemon-side in-flight transfer by transfer_id.
    71	  // M-Jobs of docs/plan/TUI_DESIGN.md §6.5. Every kind that can
    72	  // hold an active row — push, pull_sync, and delegated pull —
    73	  // supports cancellation (D-2026-07-04-3): the daemon fires the
    74	  // row's cancellation token and the dispatcher races it, so an
    75	  // attached transfer tears down promptly and its still-connected
    76	  // client receives a terminal CANCELLED status.
    77	  //
    78	  // Status semantics:
    79	  //   OK                    → the cancellation token was fired;
    80	  //                           the handler will tear down on its
    81	  //                           next .await resolve.
    82	  //   NOT_FOUND             → no active transfer matches the
    83	  //                           requested transfer_id (already
    84	  //                           completed, or never existed).
    85	  //   FAILED_PRECONDITION   → the transfer exists but its kind's
    86	  //                           dispatch policy gates cancellation
    87	  //                           off. Only the history-only pull kind
    88	  //                           (RPC deleted) is gated, and no active
    89	  //                           row of that kind can exist — a
    90	  //                           contract escape hatch, not an
    91	  //                           expected outcome.
    92	  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
    93	
    94	  // Clear the daemon's recent-transfers list (GetState.recent[]).
    95	  // Wipes the in-memory recent-runs ring AND its persisted backing
    96	  // store (recents.jsonl). It deliberately does NOT touch the
    97	  // planner/predictor's historical telemetry (perf_local.jsonl) —
    98	  // that is a separate store and the operator clearing their recents
    99	  // view must never degrade prediction quality. rec-2 of the
   100	  // recent-persistence feature.
   101	  //
   102	  // Status semantics:
   103	  //   OK → recents cleared. ClearRecentResponse.cleared carries the
   104	  //        number of entries removed from the in-memory ring.
   105	  rpc ClearRecent(ClearRecentRequest) returns (ClearRecentResponse);
   106	
   107	  // Server-streaming subscription to live daemon events. Used by the
   108	  // TUI's F2 (Transfers) pane and by future `blit jobs watch` callers
   109	  // that want a push stream rather than `GetState` polling.
   110	  //
   111	  // c-2-subscribe-skeleton lands the wire surface with a single event
   112	  // variant (TransferStarted) and the daemon-side broadcast channel.
   113	  // Subsequent C sub-slices fan more variants into the oneof
   114	  // (TransferProgress, TransferComplete, TransferError,
   115	  // ModuleListChanged, DaemonHeartbeat) and add `transfer_id_filter`
   116	  // / `replay_recent` request fields. The field numbers in
   117	  // `DaemonEvent.payload` are stable; new variants append.
   118	  //
   119	  // Slow consumers receive a `gRPC Status::Aborted` on the stream
   120	  // (the daemon's broadcast channel dropped events while they fell
   121	  // behind). TUI consumers handle this by re-subscribing and
   122	  // refreshing snapshot state via `GetState`.
   123	  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
   124	}
   125	
   126	// Removed: `BlitAuth` service stub (2026-05-13). Original design
   127	// reserved an `Authenticate(token) -> bool` RPC for a future token
   128	// auth layer. Removed entirely from 0.1.0 scope — the trust model is
   129	// "operator network controls + per-transfer data-plane tokens"
   130	// (`docs/DAEMON_CONFIG.md` Trust Model). If a real auth scheme is

exec
/usr/bin/zsh -lc 'rtk rg -n "message (CapacityProfile|FileHeader|FileData|TarShard|TarShardHeader|TarShardChunk|TarShardComplete|BlockHashList|BlockTransfer|BlockTransferComplete|DataPlaneResize|DataPlaneResizeAck|ManifestComplete|ResumeSettings|FilterSpec)|enum (ComparisonMode|MirrorMode)|PeerCapabilities|supports_stream_resize|DataTransferNegotiation" proto/blit.proto' in /home/michael/dev/Blit
 succeeded in 0ms:
35 matches in 1 files:

22:// and DataTransferNegotiation survive — PullSync and push share them.
137:message DataTransferNegotiation {
171:// support (PushHeader.supports_stream_resize or
172:// PeerCapabilities.supports_stream_resize), the daemon itself supports
195://   push:      dst daemon → client, in DataTransferNegotiation.receiver_capa...
203:message CapacityProfile {
240:// DataTransferNegotiation.resize_enabled — old peers never receive one.
255:message DataPlaneResize {
266:message DataPlaneResizeAck {
297:DataTransferNegotiation negotiation = 4; // Negotiation event before summary
342:// not carry PeerCapabilities), so the capability bit lives here;
343:// the daemon folds it into DataTransferNegotiation.resize_enabled.
347:bool supports_stream_resize = 8;
350:message FileHeader {
358:message FileData {
362:message TarShardHeader {
367:message TarShardChunk {
371:message TarShardComplete {}
373:message ManifestComplete {
437:message BlockHashList {
443:message BlockTransfer {
449:message BlockTransferComplete {
458:DataTransferNegotiation negotiation = 3;  // Data plane setup
631:PeerCapabilities client_capabilities = 8;
663:// DataTransferNegotiation.receiver_capacity.
  +10 more in proto/blit.proto

exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '1080,1280p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1080	  // c-4 + c-2 + c-3 only emit transfer-family events, so the
  1081	  // mask has nothing to filter; field is parsed and ignored.
  1082	  // Locking the tag here keeps the wire shape stable.
  1083	  uint32 event_mask = 1;
  1084	
  1085	  // c-5b: when true AND `transfer_id_filter` is non-empty,
  1086	  // the daemon replays the per-job event ring for the
  1087	  // targeted transfer (up to JOB_EVENT_RING_CAP recent events)
  1088	  // before forwarding live broadcast events. Lets a TUI that
  1089	  // connects mid-transfer pick up the bytes-completed history
  1090	  // without missing the early progress.
  1091	  //
  1092	  // No-op when `transfer_id_filter` is empty (no specific
  1093	  // ring to drain). No-op when set on its own (the row may
  1094	  // not exist yet); same row-not-found behavior as filter
  1095	  // alone.
  1096	  bool replay_recent = 2;
  1097	
  1098	  // c-5a: when set, the daemon only forwards events whose
  1099	  // `transfer_id` matches this string. Empty means "every
  1100	  // transfer-related event" (the c-2/c-3/c-4 behavior).
  1101	  //
  1102	  // Filtering is applied daemon-side so a subscriber tracking
  1103	  // a single transfer doesn't pay the wire/CPU cost of every
  1104	  // other transfer's events. ModuleListChanged / DaemonHeartbeat
  1105	  // (future variants) are not transfer-scoped — they always
  1106	  // flow regardless of this filter.
  1107	  string transfer_id_filter = 3;
  1108	}
  1109	
  1110	// One frame on the Subscribe stream. New event variants are added by
  1111	// appending to the oneof — field numbers are part of the wire
  1112	// contract. c-2-subscribe-skeleton ships only TransferStarted; future
  1113	// slices in milestone C add TransferProgress, TransferComplete,
  1114	// TransferError, ModuleListChanged, DaemonHeartbeat.
  1115	message DaemonEvent {
  1116	  oneof payload {
  1117	    TransferStarted  transfer_started  = 1;
  1118	    TransferProgress transfer_progress = 2;
  1119	    TransferComplete transfer_complete = 3;
  1120	    TransferError    transfer_error    = 4;
  1121	    // 5..6 reserved for ModuleListChanged, DaemonHeartbeat
  1122	    // (later C sub-slices).
  1123	  }
  1124	  reserved 5, 6;
  1125	}
  1126	
  1127	// Fired when the daemon registers a new active transfer in its
  1128	// `ActiveJobs` table (the same registration that surfaces it in
  1129	// `GetState.active[]`). Streaming RPCs (push, pull_sync) fire this
  1130	// with empty `module`/`path` because those values arrive in the first
  1131	// stream frame, not at dispatch time — subscribers can reconcile via
  1132	// a follow-up GetState query if they need the populated endpoint
  1133	// before the transfer completes.
  1134	message TransferStarted {
  1135	  string transfer_id = 1;
  1136	  TransferKind kind = 2;
  1137	  // `<ip>:<port>` of the connecting peer, or "unknown" when the
  1138	  // transport didn't surface one (in-process tests).
  1139	  string peer = 3;
  1140	  // Module name on the daemon. Empty for streaming RPCs at
  1141	  // registration time — populated by GetState.active[] once the
  1142	  // first stream frame parses.
  1143	  string module = 4;
  1144	  // Module-relative path. Same "empty until first frame" caveat
  1145	  // as `module`.
  1146	  string path = 5;
  1147	  // Unix milliseconds at which the active row was registered.
  1148	  uint64 start_unix_ms = 6;
  1149	}
  1150	
  1151	// Periodic chunk-granular progress for an in-flight transfer. The
  1152	// daemon fires one TransferProgress per active row per tick (default
  1153	// 10 Hz, see DEFAULT_PROGRESS_TICK_MS in service/core.rs) so a
  1154	// subscribed TUI can render a live byte-level progress bar without
  1155	// polling GetState.
  1156	//
  1157	// The bytes/files totals are absolute (cumulative since
  1158	// TransferStarted), not deltas — consumers that want per-tick rates
  1159	// subtract the previous observation. `throughput_bps` is the
  1160	// daemon-computed instantaneous rate over the most recent tick
  1161	// interval; a smoothed EWMA is a follow-up slice once we have an
  1162	// operator pain signal to justify the extra state.
  1163	message TransferProgress {
  1164	  string transfer_id = 1;
  1165	  // Cumulative bytes that landed on disk (same source as
  1166	  // GetState.active[].bytes_completed).
  1167	  uint64 bytes_completed = 2;
  1168	  // Total bytes expected, when known. 0 until a future C
  1169	  // sub-slice wires it from the manifest stage.
  1170	  uint64 bytes_total = 3;
  1171	  // Files completed; 0 until the files-counter slice wires it.
  1172	  uint64 files_completed = 4;
  1173	  // Total files expected; 0 until the manifest stage wires it.
  1174	  uint64 files_total = 5;
  1175	  // Instantaneous bytes-per-second over the most recent tick.
  1176	  // Future slice may replace with an EWMA so subscribers see
  1177	  // smoothed values; field name + semantic are forward-stable.
  1178	  uint64 throughput_bps = 6;
  1179	}
  1180	
  1181	// Fired when a transfer drains successfully (`ActiveJobGuard`
  1182	// dropped with `record_outcome(ok=true, ...)`). Carries the
  1183	// terminal byte/file totals and the wall-clock duration. Pairs
  1184	// with `TransferStarted` to bracket a transfer's lifetime on the
  1185	// event stream.
  1186	message TransferComplete {
  1187	  string transfer_id = 1;
  1188	  // Cumulative bytes that landed on disk. Sourced from the
  1189	  // per-row atomic that c-1a/c-1b wired (the same value
  1190	  // `TransferRecord.bytes` carries at GetState.recent[]).
  1191	  uint64 bytes = 2;
  1192	  // Files completed. Always zero until a future C sub-slice
  1193	  // wires the files counter; field reserved here so subscribers
  1194	  // don't need a proto roll to render it.
  1195	  uint64 files = 3;
  1196	  uint64 duration_ms = 4;
  1197	  // True when the transfer fell back to the gRPC control plane
  1198	  // because the data plane couldn't be reserved. Always false
  1199	  // in this slice — a future C sub-slice plumbs the bit through
  1200	  // from the handler's result.
  1201	  bool tcp_fallback_used = 5;
  1202	}
  1203	
  1204	// Fired when a transfer drains in a failure state
  1205	// (`ActiveJobGuard` dropped with `record_outcome(ok=false, ...)`
  1206	// or with no outcome recorded, e.g. spawn-task cancellation).
  1207	message TransferError {
  1208	  string transfer_id = 1;
  1209	  // Handler's failure message. Empty when the outcome was
  1210	  // recorded with no message; "cancelled before outcome
  1211	  // recorded" when the spawn task didn't reach the
  1212	  // record_outcome call.
  1213	  string message = 2;
  1214	}
  1215	
  1216	// ═══════════════════════════════════════════════════════════════════
  1217	// ONE_TRANSFER_PATH unified session (otp-1 wire contract).
  1218	// Authoritative state machine + frame table: docs/TRANSFER_SESSION.md.
  1219	// Compiled-but-refusing until otp-3/otp-4 land behavior (the Transfer
  1220	// handler returns UNIMPLEMENTED, pinned by test). Replaces Push and
  1221	// PullSync whole at cutover (otp-10, D-2026-07-05-1); no bridge
  1222	// (D-2026-07-05-2: same-build peers only).
  1223	// ═══════════════════════════════════════════════════════════════════
  1224	
  1225	// A transfer has a SOURCE role and a DESTINATION role. Which end
  1226	// initiated, and which CLI verb was used, select roles — never code.
  1227	enum TransferRole {
  1228	  TRANSFER_ROLE_UNSPECIFIED = 0;
  1229	  TRANSFER_ROLE_SOURCE = 1;
  1230	  TRANSFER_ROLE_DESTINATION = 2;
  1231	}
  1232	
  1233	// First frame BOTH directions. Exact-match same-build handshake
  1234	// (D-2026-07-05-2): any inequality in either field is a
  1235	// SessionError{BUILD_MISMATCH} naming both ids, then close. No
  1236	// negotiate-down, no advisory fields, no capability bits.
  1237	message SessionHello {
  1238	  // "<crate version>+<git commit>[.dirty]", composed at compile time.
  1239	  string build_id = 1;
  1240	  // Bumped on any wire-shape change; exact match required.
  1241	  uint32 contract_version = 2;
  1242	}
  1243	
  1244	// Initiator's second frame: the whole operation, roles included.
  1245	message SessionOpen {
  1246	  // Role the INITIATOR takes; the responder takes the other.
  1247	  TransferRole initiator_role = 1;
  1248	  // Responder-side module (empty = default root export) and path
  1249	  // within it. The initiator-side path never crosses the wire — the
  1250	  // initiator owns its local endpoint.
  1251	  string module = 2;
  1252	  string path = 3;
  1253	  FilterSpec filter = 4;
  1254	  ComparisonMode compare_mode = 5;
  1255	  // Mirror is explicit: enabled + scope. No implicit mirror.
  1256	  bool mirror_enabled = 6;
  1257	  MirrorMode mirror_kind = 7;
  1258	  ResumeSettings resume = 8;
  1259	  // Request the in-stream byte carrier (diagnostics / unreachable
  1260	  // data-plane environments). The responder may also force it via a
  1261	  // grant-less SessionAccept when it cannot bind a listener.
  1262	  bool in_stream_bytes = 9;
  1263	  bool ignore_existing = 10;
  1264	  bool require_complete_scan = 11;
  1265	  // Set iff the initiator is DESTINATION (dial contract: the byte
  1266	  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
  1267	  // unknown hardware value, conservative, never "old peer").
  1268	  CapacityProfile receiver_capacity = 12;
  1269	}
  1270	
  1271	// Responder's reply. Refusals are SessionError frames, never silent
  1272	// closes.
  1273	message SessionAccept {
  1274	  // Set iff the responder is DESTINATION.
  1275	  CapacityProfile receiver_capacity = 1;
  1276	  // Absent = in-stream carrier (requested, or listener bind failed).
  1277	  DataPlaneGrant data_plane = 2;
  1278	}
  1279	
  1280	// TCP data-plane grant. The RESPONDER always binds; the INITIATOR

codex
One thing already stands out as a likely review target: the new session reuses older messages whose comments still encode pre-cutover negotiation/version semantics. I’m checking whether that is merely transitional documentation or whether it leaks into the new contract in a way that would mislead later implementation.
exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '130,280p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   130	// (`docs/DAEMON_CONFIG.md` Trust Model). If a real auth scheme is
   131	// ever needed, design it from scratch rather than retain a misleading
   132	// stub.
   133	
   134	// --- Message Definitions ---
   135	
   136	// Negotiation response indicating how the data plane (TCP/RDMA) is configured.
   137	message DataTransferNegotiation {
   138	  uint32 tcp_port = 1;
   139	  string one_time_token = 2;
   140	  bool tcp_fallback = 3; // true when the server could not reserve the preferred data-plane.
   141	  uint32 stream_count = 4; // number of parallel TCP streams negotiated for the data plane
   142	  reserved 5 to 10; // RDMA fields (QP numbers, GID, etc.) for Phase 3.5
   143	
   144	  // ── ue-r2-1b wire dial contract (UNIFIED_TRANSFER_ENGINE_REV4 §5) ──
   145	  // Fields 11-13 are defined ahead of the behavior slices that consume
   146	  // them (ue-r2-1e live dials, ue-r2-2 stream resize). Until those land,
   147	  // senders leave all three unset. Mixed-version behavior: an old peer
   148	  // skips unknown fields, a new peer treats "absent" as "old peer" —
   149	  // both degrade to today's static-ladder behavior.
   150	  //
   151	  // NOTE on field numbers: the abandoned adaptive-streams PR3 branch
   152	  // (d9d4ec7) used 11-14 for min/max_stream_count/adaptive_enabled/
   153	  // epoch0_sub_token. That contract never shipped anywhere, so the
   154	  // numbers are free to reassign; REV4 assigns 11 to receiver_capacity.
   155	  // PR3's negotiated min/max stream bounds are subsumed by
   156	  // CapacityProfile.max_streams (ceiling) with an implicit floor of 1.
   157	
   158	  // The byte receiver's capacity profile, advertised to the byte sender
   159	  // during setup. On push this message already travels daemon→client and
   160	  // the daemon is the receiver, so the daemon stamps its own profile
   161	  // here. On pull_sync the negotiation travels daemon→client but the
   162	  // CLIENT is the byte receiver — the profile travels client→daemon in
   163	  // TransferOperationSpec.receiver_capacity instead, and this field
   164	  // stays unset. Absent = "peer predates the profile or has nothing to
   165	  // advertise" → the sender keeps today's conservative/static behavior.
   166	  CapacityProfile receiver_capacity = 11;
   167	
   168	  // Daemon-authoritative resize gate: the single source of truth both
   169	  // ends read before any DataPlaneResize may be sent (ue-r2-2). The
   170	  // daemon sets it true only when ALL hold: the peer advertised resize
   171	  // support (PushHeader.supports_stream_resize or
   172	  // PeerCapabilities.supports_stream_resize), the daemon itself supports
   173	  // resize, tcp_port != 0, and !tcp_fallback. False/absent = fixed
   174	  // stream count for the whole transfer; resize messages must not be
   175	  // sent in either direction (an old peer would decode them as an
   176	  // unknown payload variant and see an empty frame).
   177	  bool resize_enabled = 12;
   178	
   179	  // 16 random bytes that every initial (epoch 0) data socket must echo
   180	  // after the one_time_token when resize_enabled is true. Empty and
   181	  // ignored when resize_enabled is false (the pre-resize handshake is
   182	  // unchanged, so old peers never see a suffixed handshake).
   183	  bytes epoch0_sub_token = 13;
   184	}
   185	
   186	// ── ue-r2-1b: receiver capacity profile ─────────────────────────────
   187	// The rich profile the byte RECEIVER advertises to the byte SENDER at
   188	// setup. The sender owns the live dial (chunk size, prefetch, in-flight
   189	// bytes, and — after ue-r2-2 — stream count) and must keep it within
   190	// this profile; the initial dial additionally starts BELOW the ceiling
   191	// with margin (REV4 "Risks": a receiver may over-advertise, and there
   192	// is no probe phase to catch it before the first byte).
   193	//
   194	// Travel direction per path (receiver → sender):
   195	//   push:      dst daemon → client, in DataTransferNegotiation.receiver_capacity
   196	//   pull_sync: client → src daemon, in TransferOperationSpec.receiver_capacity
   197	//   delegated: dst daemon → src daemon, in the forwarded spec (the dst
   198	//              daemon REPLACES any CLI-supplied value — same override
   199	//              boundary as client_capabilities)
   200	//
   201	// Every field uses 0 (or UNSPECIFIED) as "unknown"; the sender treats
   202	// unknown as "no information — stay conservative", never as "unlimited".
   203	message CapacityProfile {
   204	  // Logical CPU cores the receiver can devote to this transfer.
   205	  uint32 cpu_cores = 1;
   206	  // Storage class of the receive target, the coarse drain-speed signal.
   207	  DrainClass drain_class = 2;
   208	  // Receiver's current overall load estimate, percent (0-100+; may
   209	  // exceed 100 when oversubscribed, e.g. loadavg > cores). 0 = unknown
   210	  // or idle — senders must not distinguish the two.
   211	  uint32 load_percent = 3;
   212	  // Maximum parallel data-plane streams the receiver will accept for
   213	  // this transfer (the dial's hard ceiling; floor is always 1).
   214	  // 0 = unknown → sender stays at today's negotiated stream_count.
   215	  uint32 max_streams = 4;
   216	  // Estimated sustainable drain (write-to-storage) rate in bytes/sec.
   217	  uint64 drain_rate_bytes_per_sec = 5;
   218	  // Largest single chunk the receiver wants on the wire, bytes.
   219	  uint64 max_chunk_bytes = 6;
   220	  // Ceiling on prefetch / un-acked in-flight bytes the receiver can
   221	  // buffer safely.
   222	  uint64 max_inflight_bytes = 7;
   223	}
   224	
   225	// Coarse storage class for CapacityProfile.drain_class. Deliberately
   226	// coarse: a hint for the sender's starting dial, not a benchmark.
   227	enum DrainClass {
   228	  DRAIN_CLASS_UNSPECIFIED = 0;
   229	  DRAIN_CLASS_HDD = 1;
   230	  DRAIN_CLASS_SSD_SATA = 2;
   231	  DRAIN_CLASS_SSD_NVME = 3;
   232	  DRAIN_CLASS_NETWORK_FS = 4; // receive target is itself remote (NFS/SMB/…)
   233	  DRAIN_CLASS_MEMORY = 5;     // tmpfs/ramdisk-class target
   234	}
   235	
   236	// ── ue-r2-1b: mid-transfer stream resize (consumed at ue-r2-2) ──────
   237	// Control-plane request to grow/shrink the live data-plane stream set.
   238	// Carried on the transfer control streams (never as a blind TCP
   239	// data-plane record), and only after the daemon set
   240	// DataTransferNegotiation.resize_enabled — old peers never receive one.
   241	// Shape carried over from the adaptive-streams PR3 prior art (d9d4ec7).
   242	enum DataPlaneResizeOp {
   243	  DATA_PLANE_RESIZE_OP_UNSPECIFIED = 0;
   244	  DATA_PLANE_RESIZE_OP_ADD = 1;
   245	  DATA_PLANE_RESIZE_OP_REMOVE = 2;
   246	}
   247	
   248	// Controller → peer request to resize the live stream set. `epoch` is a
   249	// monotonic resize id (0 is reserved for the initial streams);
   250	// `target_stream_count` is the absolute desired live count (idempotent),
   251	// bounded by CapacityProfile.max_streams. For ADD, `sub_token` is the
   252	// 16-byte credential the newly dialed data socket must present after
   253	// the one_time_token; the accepting side registers it before the dialer
   254	// dials. For REMOVE, sub_token is empty.
   255	message DataPlaneResize {
   256	  DataPlaneResizeOp op = 1;
   257	  uint32 epoch = 2;
   258	  uint32 target_stream_count = 3;
   259	  bytes sub_token = 4;
   260	}
   261	
   262	// Ack of a DataPlaneResize. `accepted` is false if the peer could not
   263	// honor it (e.g. registration refused, dial failed);
   264	// `effective_stream_count` is the live count the acking side now
   265	// believes is in effect.
   266	message DataPlaneResizeAck {
   267	  uint32 epoch = 1;
   268	  uint32 effective_stream_count = 2;
   269	  bool accepted = 3;
   270	}
   271	
   272	// Push Operation
   273	message ClientPushRequest {
   274	  oneof payload {
   275	    PushHeader header = 1;
   276	    FileHeader file_manifest = 2;
   277	    ManifestComplete manifest_complete = 3;
   278	    FileData file_data = 4;
   279	    UploadComplete upload_complete = 5;
   280	    TarShardHeader tar_shard_header = 6;

exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '330,470p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   330	  // R59 #1 F1: client signals it intends the source-side scan
   331	  // to be complete (no unreadable subtrees). When true, the
   332	  // daemon refuses to purge any destination entries if the
   333	  // client's ManifestComplete.scan_complete arrives as false —
   334	  // because a partial scan would let absent-due-to-unreadable
   335	  // files be deleted from the destination. Independent of
   336	  // mirror_kind: required for mirror correctness, also useful
   337	  // for move-like pushes once those exist.
   338	  bool require_complete_scan = 7;
   339	
   340	  // ue-r2-1b: the push client advertises that it can drive
   341	  // mid-transfer stream resize. Push opens with PushHeader (it does
   342	  // not carry PeerCapabilities), so the capability bit lives here;
   343	  // the daemon folds it into DataTransferNegotiation.resize_enabled.
   344	  // Old clients leave it false and run at the fixed negotiated
   345	  // stream count. Clients keep it false until ue-r2-2 actually
   346	  // implements resize.
   347	  bool supports_stream_resize = 8;
   348	}
   349	
   350	message FileHeader {
   351	  string relative_path = 1;
   352	  uint64 size = 2;
   353	  int64 mtime_seconds = 3;
   354	  uint32 permissions = 4;
   355	  bytes checksum = 5;  // Blake3 hash (32 bytes), empty if not computed
   356	}
   357	
   358	message FileData {
   359	  bytes content = 1;
   360	}
   361	
   362	message TarShardHeader {
   363	  repeated FileHeader files = 1;
   364	  uint64 archive_size = 2;
   365	}
   366	
   367	message TarShardChunk {
   368	  bytes content = 1;
   369	}
   370	
   371	message TarShardComplete {}
   372	
   373	message ManifestComplete {
   374	  // R59 #1 F1: client tells the daemon whether its source-side
   375	  // scan finished cleanly. False when any subtree was unreadable
   376	  // (EACCES, ELOOP, IO errors). Required for the daemon to
   377	  // safely purge in mirror mode — see PushHeader.require_complete_scan.
   378	  // Pre-fix the daemon purged destination entries unconditionally
   379	  // after upload, so a permission error mid-scan caused silent
   380	  // data loss on the destination.
   381	  bool scan_complete = 1;
   382	}
   383	message UploadComplete {}
   384	message Ack {}
   385	
   386	// Acknowledgment for PullSync with server capabilities
   387	message PullSyncAck {
   388	  bool server_checksums_enabled = 1;  // Whether daemon computed checksums for manifest
   389	}
   390	message FileList { repeated string relative_paths = 1; }
   391	message PushSummary {
   392	    uint64 files_transferred = 1;
   393	    uint64 bytes_transferred = 2;
   394	    uint64 bytes_zero_copy = 3; // bytes sent via zero-copy kernel paths
   395	    bool tcp_fallback_used = 4; // true if we had to fall back to gRPC streaming
   396	    uint64 entries_deleted = 5; // count of files/dirs removed during mirror purge
   397	}
   398	
   399	// Pull progress/summary messages. (PullRequest/PullChunk, the
   400	// deprecated Pull RPC's messages, were removed 2026-07-03 at
   401	// ue-r2-1h — see the note in the service block. These two survive
   402	// because ServerPullMessage and DelegatedPullProgress reuse them.)
   403	message ManifestBatch {
   404	  uint64 file_count = 1;
   405	  uint64 total_bytes = 2;
   406	}
   407	
   408	message PullSummary {
   409	  uint64 files_transferred = 1;
   410	  uint64 bytes_transferred = 2;
   411	  uint64 bytes_zero_copy = 3;
   412	  bool tcp_fallback_used = 4;
   413	  uint64 entries_deleted = 5; // count of files/dirs removed during mirror purge
   414	}
   415	
   416	// PullSync Operation (bidirectional pull with manifest comparison).
   417	// The leading message is now `TransferOperationSpec` (the unified
   418	// contract — see step 2 of docs/plan/PIPELINE_UNIFICATION.md). The
   419	// previous `PullSyncHeader` bool-soup shape was removed entirely
   420	// because we don't carry backward compatibility into the next release.
   421	message ClientPullMessage {
   422	  oneof payload {
   423	    TransferOperationSpec spec = 1;      // Initial request: module, path, filter, compare/mirror/resume
   424	    FileHeader local_file = 2;           // Client's local file metadata
   425	    ManifestComplete manifest_done = 3;  // Signals end of client manifest
   426	    BlockHashList block_hashes = 4;      // Client's block hashes for resume mode
   427	    // ue-r2-1b: on pull the daemon is the sender/controller and the
   428	    // client is the dialer. The client acks the daemon's resize command
   429	    // here, on the outbound half — a different stream direction than
   430	    // the inbound ServerPullMessage carrying the command, so no
   431	    // head-of-line block. Unused until ue-r2-2.
   432	    DataPlaneResizeAck data_plane_resize_ack = 5;
   433	  }
   434	}
   435	
   436	// Block-level resume messages
   437	message BlockHashList {
   438	  string relative_path = 1;              // File this applies to
   439	  uint32 block_size = 2;                 // Size of each block (last may be smaller)
   440	  repeated bytes hashes = 3;             // Blake3 hashes (32 bytes each), in order
   441	}
   442	
   443	message BlockTransfer {
   444	  string relative_path = 1;              // File this block belongs to
   445	  uint64 offset = 2;                     // Byte offset in file
   446	  bytes content = 3;                     // Block data
   447	}
   448	
   449	message BlockTransferComplete {
   450	  string relative_path = 1;              // File that's now complete
   451	  uint64 total_bytes = 2;                // Final file size (for truncation if needed)
   452	}
   453	
   454	message ServerPullMessage {
   455	  oneof payload {
   456	    Ack ack = 1;                              // Acknowledgment of header (deprecated, use pull_sync_ack)
   457	    FileList files_to_download = 2;           // The "NeedList" - files client should receive
   458	    DataTransferNegotiation negotiation = 3;  // Data plane setup
   459	    PullSummary summary = 4;                  // Final summary
   460	    ManifestBatch manifest_batch = 5;         // Progress: server discovered X files
   461	    FileHeader file_header = 6;               // For gRPC fallback: file being sent
   462	    FileData file_data = 7;                   // For gRPC fallback: file content
   463	    PullSyncAck pull_sync_ack = 8;            // Acknowledgment with server capabilities
   464	    BlockHashRequest block_hash_request = 9;  // Server requests block hashes for resume
   465	    BlockTransfer block_transfer = 10;        // Server sends a block for resume
   466	    BlockTransferComplete block_complete = 11; // Server signals file resume complete
   467	    // Authoritative deletion list for mirror mode. Server has the
   468	    // filtered source manifest and the unfiltered client manifest;
   469	    // it computes exactly which client paths should be removed (with
   470	    // MirrorMode::FilteredSubset, only client files that match the

exec
/usr/bin/zsh -lc "rtk rg -n \"Transfer\\(|transfer\\(|UNIMPLEMENTED|unimplemented|TransferFrame|transfer_frame|transfer_unimplemented|1484|1483\" crates proto docs .review REVIEW.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '1240,1355p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1240	  // Bumped on any wire-shape change; exact match required.
  1241	  uint32 contract_version = 2;
  1242	}
  1243	
  1244	// Initiator's second frame: the whole operation, roles included.
  1245	message SessionOpen {
  1246	  // Role the INITIATOR takes; the responder takes the other.
  1247	  TransferRole initiator_role = 1;
  1248	  // Responder-side module (empty = default root export) and path
  1249	  // within it. The initiator-side path never crosses the wire — the
  1250	  // initiator owns its local endpoint.
  1251	  string module = 2;
  1252	  string path = 3;
  1253	  FilterSpec filter = 4;
  1254	  ComparisonMode compare_mode = 5;
  1255	  // Mirror is explicit: enabled + scope. No implicit mirror.
  1256	  bool mirror_enabled = 6;
  1257	  MirrorMode mirror_kind = 7;
  1258	  ResumeSettings resume = 8;
  1259	  // Request the in-stream byte carrier (diagnostics / unreachable
  1260	  // data-plane environments). The responder may also force it via a
  1261	  // grant-less SessionAccept when it cannot bind a listener.
  1262	  bool in_stream_bytes = 9;
  1263	  bool ignore_existing = 10;
  1264	  bool require_complete_scan = 11;
  1265	  // Set iff the initiator is DESTINATION (dial contract: the byte
  1266	  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
  1267	  // unknown hardware value, conservative, never "old peer").
  1268	  CapacityProfile receiver_capacity = 12;
  1269	}
  1270	
  1271	// Responder's reply. Refusals are SessionError frames, never silent
  1272	// closes.
  1273	message SessionAccept {
  1274	  // Set iff the responder is DESTINATION.
  1275	  CapacityProfile receiver_capacity = 1;
  1276	  // Absent = in-stream carrier (requested, or listener bind failed).
  1277	  DataPlaneGrant data_plane = 2;
  1278	}
  1279	
  1280	// TCP data-plane grant. The RESPONDER always binds; the INITIATOR
  1281	// always dials (connection topology, not choreography — byte
  1282	// direction on the sockets is set by role: SOURCE writes).
  1283	message DataPlaneGrant {
  1284	  uint32 tcp_port = 1;
  1285	  bytes session_token = 2;
  1286	  // Dial floor. SOURCE shape-corrects upward via resize (sf-2
  1287	  // mechanism — the only stream policy).
  1288	  uint32 initial_streams = 3;
  1289	  // Resize is always available (same build): epoch-0 credential.
  1290	  bytes epoch0_sub_token = 4;
  1291	}
  1292	
  1293	// DESTINATION → SOURCE: files the destination wants, in batches.
  1294	message NeedEntry {
  1295	  string relative_path = 1;
  1296	  // RELIABLE resume exception (docs/TRANSFER_SESSION.md): when true,
  1297	  // the destination's BlockHashList for this file follows, and the
  1298	  // source must not send any byte of this file before receiving it;
  1299	  // stale/mismatched partials fall back to full-file transfer.
  1300	  bool resume = 2;
  1301	}
  1302	message NeedBatch {
  1303	  repeated NeedEntry entries = 1;
  1304	}
  1305	// DESTINATION's promise that no further NeedBatch follows.
  1306	message NeedComplete {}
  1307	
  1308	// SOURCE's promise that every requested payload byte is flushed.
  1309	message SourceDone {}
  1310	
  1311	// DESTINATION → SOURCE at close: the end that wrote bytes and
  1312	// executed deletes attests to the outcome (one summary shape for
  1313	// every direction; replaces PushSummary/PullSummary at cutover).
  1314	message TransferSummary {
  1315	  uint64 files_transferred = 1;
  1316	  uint64 bytes_transferred = 2;
  1317	  uint64 entries_deleted = 3;   // mirror executed destination-local
  1318	  bool in_stream_carrier_used = 4;
  1319	  uint64 files_resumed = 5;
  1320	}
  1321	
  1322	// Structured refusal/abort — an end says why before closing.
  1323	message SessionError {
  1324	  enum Code {
  1325	    SESSION_ERROR_UNSPECIFIED = 0;
  1326	    BUILD_MISMATCH = 1;
  1327	    MODULE_UNKNOWN = 2;
  1328	    READ_ONLY = 3;
  1329	    DELEGATION_REFUSED = 4;
  1330	    SCAN_INCOMPLETE = 5;
  1331	    PROTOCOL_VIOLATION = 6;
  1332	    DATA_PLANE_FAILED = 7;
  1333	    CANCELLED = 8;
  1334	    INTERNAL = 9;
  1335	  }
  1336	  Code code = 1;
  1337	  string message = 2;
  1338	  // BUILD_MISMATCH: both build ids, so the operator sees exactly
  1339	  // which end is stale.
  1340	  string local_build_id = 3;
  1341	  string peer_build_id = 4;
  1342	}
  1343	
  1344	// The single frame type BOTH wire directions carry. Which frames an
  1345	// end may send is determined by ROLE and phase
  1346	// (docs/TRANSFER_SESSION.md frame table — field numbers frozen
  1347	// there). Reuses the engine's existing payload vocabulary verbatim.
  1348	message TransferFrame {
  1349	  oneof frame {
  1350	    SessionHello hello = 1;
  1351	    SessionOpen open = 2;
  1352	    SessionAccept accept = 3;
  1353	    FileHeader manifest_entry = 4;
  1354	    ManifestComplete manifest_complete = 5;
  1355	    NeedBatch need_batch = 6;

 succeeded in 0ms:
399 matches in 62 files:

.review/findings/audit-h1-mirror-relay-incomplete-scan.md:16:`run_remote_push_transfer(args, Endpoint::Remote(src), dst, mirror)` (R2-H1 e...
.review/findings/b-4-getstate.md:68:`Status::unimplemented`. Same treatment as the prior
.review/findings/c-2-subscribe-skeleton.md:174:`fn subscribe` returning `Status::unimplemented`.
.review/findings/d-22-f2-cancel-selected.md:75:spawn_cancel_transfer(rid, endpoint, id, tx);
.review/findings/d-22-f2-cancel-selected.md:86:fn spawn_cancel_transfer(request_id, endpoint, transfer_id, tx)
.review/findings/d-30-batch-cancel.md:105:spawn_cancel_transfer(*cancel_request_seq, endpoint.clone(), id, tx.clone());
.review/findings/d-4-f4-local-transfers.md:61:`spawn_local_transfer(id, kind, src, dst, tx)`:
.review/findings/d-4-f4-local-transfers.md:81:- F4 arm gates on `can_start_transfer(app)`:
.review/findings/d-5-f4-local-move.md:79:UserAction::TransferMove if can_start_transfer(app) => {
.review/findings/d-5-f4-local-move.md:80:match prepare_local_transfer(...) {
.review/findings/otp-1-wire-session-contract.md:11:UNIMPLEMENTED, pinned by test.
.review/findings/otp-1-wire-session-contract.md:16:role-tagged single frame vocabulary (`TransferFrame` both wire
.review/findings/otp-1-wire-session-contract.md:33:- **`proto/blit.proto`**: `rpc Transfer(stream TransferFrame) returns
.review/findings/otp-1-wire-session-contract.md:34:(stream TransferFrame)` + `TransferRole`, `SessionHello`,
.review/findings/otp-1-wire-session-contract.md:39:the 20-arm `TransferFrame` oneof reusing the engine's existing
.review/findings/otp-1-wire-session-contract.md:45:methods): `BlitService::transfer` → UNIMPLEMENTED with a pointer to
.review/findings/otp-1-wire-session-contract.md:65:Suite 1483 → **1484 passed / 0 failed** (37 suites, same 2 ignored);
.review/findings/otp-1-wire-session-contract.md:68:- `transfer_rpc_exists_and_refuses_unimplemented` (in-process real
.review/findings/otp-1-wire-session-contract.md:70:and refuses with UNIMPLEMENTED — not UNKNOWN — until otp-3/otp-4.
.review/findings/rec-2-clear-recent.md:42:their existing style (`unimplemented!()` / `Status::unimplemented`).
.review/findings/sf-2-shape-correction-resize.md:66:Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
.review/results/bench-script-fix.codex.md:4247:crates/blit-core/tests/pull_sync_with_spec_wire.rs:67:        unimplemented!(...
.review/results/bench-script-fix.codex.md:4249:crates/blit-core/tests/pull_sync_with_spec_wire.rs:74:        unimplemented!(...
.review/results/bench-script-fix.codex.md:4317:crates/blit-core/tests/pull_sync_with_spec_wire.rs:531:        unimplemented!...
.review/results/bench-script-fix.codex.md:5649:crates/blit-cli/tests/remote_remote.rs:205:    let stale = spawn_fake_blit_se...
.review/results/bench-script-fix.codex.md:5676:crates/blit-cli/tests/remote_remote.rs:364:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5679:crates/blit-cli/tests/remote_remote.rs:371:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5681:crates/blit-cli/tests/remote_remote.rs:378:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5684:crates/blit-cli/tests/remote_remote.rs:385:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5687:crates/blit-cli/tests/remote_remote.rs:392:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5690:crates/blit-cli/tests/remote_remote.rs:399:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5693:crates/blit-cli/tests/remote_remote.rs:406:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5695:crates/blit-cli/tests/remote_remote.rs:413:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5697:crates/blit-cli/tests/remote_remote.rs:420:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5700:crates/blit-cli/tests/remote_remote.rs:427:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5703:crates/blit-cli/tests/remote_remote.rs:434:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5706:crates/blit-cli/tests/remote_remote.rs:441:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5709:crates/blit-cli/tests/remote_remote.rs:448:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:5712:crates/blit-cli/tests/remote_remote.rs:455:        Err(tonic::Status::unimple...
.review/results/bench-script-fix.codex.md:6189:crates/blit-cli/src/transfers/remote.rs:161:pub async fn run_remote_push_tran...
.review/results/bench-script-fix.codex.md:6216:crates/blit-cli/src/transfers/remote.rs:271:pub async fn run_remote_pull_tran...
.review/results/bench-script-fix.codex.md:6316:crates/blit-core/src/remote/pull.rs:54:            Self::Transfer(message) =>...
.review/results/bench-script-fix.codex.md:6476:crates/blit-core/src/remote/pull.rs:917:                Some(server_pull_mess...
.review/results/bench-script-fix.codex.md:6839:crates/blit-cli/src/transfers/mod.rs:249:            run_remote_push_transfer...
.review/results/bench-script-fix.codex.md:6842:crates/blit-cli/src/transfers/mod.rs:254:            run_remote_pull_transfer(
.review/results/bench-script-fix.codex.md:6846:crates/blit-cli/src/transfers/mod.rs:277:            run_remote_push_transfer...
  +10 more in .review/results/bench-script-fix.codex.md
.review/results/d-2026-07-05-2-compat.codex.md:250:**2026-07-05 06:45:00Z** - **CODER (sf-2 shape-correction stream resize, clau...
.review/results/d-2026-07-05-2-compat.codex.md:372:suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
.review/results/d-2026-07-05-2-compat.codex.md:838:docs/audit/DESIGN_MAP_2026-06-11.md:228:- audit-h3c slice 2 (the gRPC-fallbac...
.review/results/d-2026-07-05-2-compat.codex.md:932:docs/audit/findings/drift-perf.md:74:**Code does**: `PerformancePredictor::ob...
.review/results/d-2026-07-05-3-zerocopy.codex.md:427:suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
.review/results/d-2026-07-05-3-zerocopy.codex.md:580:1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none.
.review/results/d-2026-07-05-3-zerocopy.codex.md:983:(1483); all REV4 invariant pins and the sf-2 pin pass
.review/results/d-2026-07-05-3-zerocopy.codex.md:1181:docs/STATE.md:39:  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ bl...
.review/results/d-2026-07-05-3-zerocopy.codex.md:1233:docs/ARCHITECTURE.md:341:    fn zero_copy_transfer(&self, src: &Path, dst: &P...
.review/results/d-2026-07-05-3-zerocopy.codex.md:1468:docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:709:**Mechanism**: rg for 'j...
.review/results/d-2026-07-05-3-zerocopy.codex.md:2073:.review/results/d-2026-07-05-3-zerocopy.codex.md:427:  suite 1479 → 1483/0, D...
.review/results/d-2026-07-05-3-zerocopy.codex.md:2286:.review/results/d-2026-07-05-2-compat.codex.md:372:   suite 1479 → 1483/0, DE...
.review/results/d-2026-07-05-3-zerocopy.codex.md:2560:.review/results/one-transfer-path-plan.codex.md:161:+  suite 1479 → 1483/0, D...
.review/results/d-2026-07-05-3-zerocopy.codex.md:2692:.review/results/sf-2-shape-correction-resize.codex.md:14:Review the diff of c...
.review/results/d-2026-07-05-3-zerocopy.codex.md:2738:.review/results/sf-2-shape-correction-resize.codex.md:4783:.review/results/sf...
.review/results/d-2026-07-05-3-zerocopy.codex.md:4074:132	      (1483); all REV4 invariant pins and the sf-2 pin pass
.review/results/d-2026-07-05-3-zerocopy.codex.md:4158:docs/STATE.md:39:  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ bl...
.review/results/one-transfer-path-plan.codex.md:79:**2026-07-05 06:45:00Z** - **CODER (sf-2 shape-correction stream resize, clau...
.review/results/one-transfer-path-plan.codex.md:138:-  stream, guard proven by revert; suite 1479 → 1483/0. Pull side
.review/results/one-transfer-path-plan.codex.md:161:+  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
.review/results/one-transfer-path-plan.codex.md:277:-  1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none. **Exact
.review/results/one-transfer-path-plan.codex.md:292:+  1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none.
.review/results/one-transfer-path-plan.codex.md:407:+      (1483); all REV4 invariant pins and the sf-2 pin pass
.review/results/one-transfer-path-plan.codex.md:1575:107	      (1483); all REV4 invariant pins and the sf-2 pin pass
.review/results/otp-1-wire-session-contract.codex.md:14:Review the diff of commit a3e2acb (run: git show a3e2acb). It implements otp-...
.review/results/otp-1-wire-session-contract.codex.md:290:suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
.review/results/otp-1-wire-session-contract.codex.md:445:1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none.
.review/results/otp-1-wire-session-contract.codex.md:733:(1483); all REV4 invariant pins and the sf-2 pin pass
.review/results/otp-1-wire-session-contract.codex.md:969:session messages (20-arm TransferFrame reusing the engine payload
.review/results/otp-1-wire-session-contract.codex.md:971:Handler + five test fakes stub UNIMPLEMENTED; in-process pin proves
.review/results/otp-1-wire-session-contract.codex.md:972:the RPC is reachable and refusing. Suite 1483 -> 1484.
.review/results/otp-1-wire-session-contract.codex.md:998:+   same frame type (`TransferFrame`). Which frames an end may send is
.review/results/otp-1-wire-session-contract.codex.md:1080:+`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
.review/results/otp-1-wire-session-contract.codex.md:1082:+`TransferFrame.frame` oneof (field numbers frozen by this doc):
.review/results/otp-1-wire-session-contract.codex.md:1165:+(`Transfer` returns `UNIMPLEMENTED`; pinned by test).
.review/results/otp-1-wire-session-contract.codex.md:1183:session messages (20-arm TransferFrame reusing the engine payload
.review/results/otp-1-wire-session-contract.codex.md:1185:Handler + five test fakes stub UNIMPLEMENTED; in-process pin proves
.review/results/otp-1-wire-session-contract.codex.md:1186:the RPC is reachable and refusing. Suite 1483 -> 1484.
.review/results/otp-1-wire-session-contract.codex.md:1206:+  // docs/TRANSFER_SESSION.md. UNIMPLEMENTED until otp-3/otp-4.
.review/results/otp-1-wire-session-contract.codex.md:1207:+  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
.review/results/otp-1-wire-session-contract.codex.md:1375:+// handler returns UNIMPLEMENTED, pinned by test). Replaces Push and
.review/results/otp-1-wire-session-contract.codex.md:1503:+message TransferFrame {
.review/results/otp-1-wire-session-contract.codex.md:1550:17	   same frame type (`TransferFrame`). Which frames an end may send is
.review/results/otp-1-wire-session-contract.codex.md:1632:99	`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
.review/results/otp-1-wire-session-contract.codex.md:1634:101	`TransferFrame.frame` oneof (field numbers frozen by this doc):
.review/results/otp-1-wire-session-contract.codex.md:1717:184	(`Transfer` returns `UNIMPLEMENTED`; pinned by test).
.review/results/otp-1-wire-session-contract.codex.md:1733:12	  // docs/TRANSFER_SESSION.md. UNIMPLEMENTED until otp-3/otp-4.
.review/results/otp-1-wire-session-contract.codex.md:1734:13	  rpc Transfer(stream TransferFrame) returns (stream TransferFrame);
.review/results/otp-1-wire-session-contract.codex.md:2028:1220	// handler returns UNIMPLEMENTED, pinned by test). Replaces Push and
.review/results/sf-2-shape-correction-resize.codex.md:14:Review the diff of commit c70c2ac (run: git show c70c2ac). It implements sf-2...
.review/results/sf-2-shape-correction-resize.codex.md:52:1479 -> 1483.
.review/results/sf-2-shape-correction-resize.codex.md:134:+Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
.review/results/sf-2-shape-correction-resize.codex.md:2447:1483
.review/results/sf-2-shape-correction-resize.codex.md:2448:1484	        if let Some(sender) = data_plane_sender.take() {
.review/results/sf-2-shape-correction-resize.codex.md:4682:/usr/bin/zsh -lc "rg -n \"shape_resize|many_tiny_file|initial_stream_proposal...
.review/results/sf-2-shape-correction-resize.codex.md:4777:.review/findings/sf-2-shape-correction-resize.md:65:Suite 1479 → **1483 passe...
.review/results/sf-2-shape-correction-resize.codex.md:4783:.review/results/sf-2-shape-correction-resize.codex.md:14:Review the diff of c...
.review/results/sf-2-shape-correction-resize.codex.md:4786:.review/results/sf-2-shape-correction-resize.codex.md:52:    1479 -> 1483.
.review/results/sf-2-shape-correction-resize.codex.md:4794:.review/results/sf-2-shape-correction-resize.codex.md:134:+Suite 1479 → **148...
.review/results/sf-2-shape-correction-resize.codex.md:4886:.review/results/sf-2-shape-correction-resize.codex.md:2447:  1483
.review/results/sf-2-shape-correction-resize.codex.md:5086:.review/results/small-file-ceiling-plan.codex.md:1484:.review/results/w1-3-tc...
.review/results/sf-2-shape-correction-resize.codex.md:5555:For the count gate, I’m not going to run the suite in this read-only sandbox ...
.review/results/sf-2-shape-correction-resize.codex.md:6701:VERDICT: NEEDS FIXES. Static diff shows +4 tests and no removed test annotati...
.review/results/sf-2-shape-correction-resize.codex.md:6706:VERDICT: NEEDS FIXES. Static diff shows +4 tests and no removed test annotati...
.review/results/sf-2-shape-correction-resize.gpt-verdict.md:27:1483 passed / 0 failed.
.review/results/sf-2-shape-correction-resize.gpt-verdict.md:31:executed here: 1483/0).
.review/results/small-file-ceiling-plan.codex.md:1421:.review/results/bench-script-fix.codex.md:8940:crates/blit-core/src/remote/tr...
.review/results/small-file-ceiling-plan.codex.md:1453:.review/results/bench-script-fix.codex.md:12710:crates/blit-core/src/remote/t...
.review/results/small-file-ceiling-plan.codex.md:1463:.review/results/w9-3-test-harness-builder.codex.md:4144:crates/blit-daemon/sr...
.review/results/small-file-ceiling-plan.codex.md:1498:docs/reviews/followup_review_2026-05-02.md:457:- Included commits: `d68f9f7`,...
.review/results/small-file-ceiling-plan.codex.md:1903:**2026-05-05 18:00:00Z** - **ACTION**: §2.8 phase 1 of `docs/plan/RELEASE_PLA...
.review/results/small-file-ceiling-plan.codex.md:1935:**2026-05-02 21:30:00Z** - **ACTION**: Closed F5, F11, F12, F9 from `docs/rev...
.review/results/small-file-ceiling-plan.codex.md:2125:**2026-05-05 18:00:00Z** - **ACTION**: §2.8 phase 1 of `docs/plan/RELEASE_PLA...
.review/results/small-file-ceiling-plan.codex.md:2157:**2026-05-02 21:30:00Z** - **ACTION**: Closed F5, F11, F12, F9 from `docs/rev...
.review/results/ue-r2-1h.gpt-verdict.md:87:specs; the 4 shrunk mock `pull` impls were pure `unimplemented!` stubs
.review/results/w9-3-test-harness-builder.codex.md:1929:@@ -320,8 +201,8 @@ fn stale_destination_unimplemented_does_not_fall_back_to_...
.review/results/w9-3-test-harness-builder.codex.md:1934:-    let stale = spawn_unimplemented_blit_server();
.review/results/w9-3-test-harness-builder.codex.md:1936:+    let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimplemente...
.review/results/w9-3-test-harness-builder.codex.md:1940:@@ -354,7 +235,7 @@ fn stale_destination_unimplemented_does_not_fall_back_to_...
.review/results/w9-3-test-harness-builder.codex.md:2069:-fn spawn_unimplemented_blit_server() -> UnimplementedServerGuard {
.review/results/w9-3-test-harness-builder.codex.md:2099:-    wait_for_port(port, "fake unimplemented destination");
.review/results/w9-3-test-harness-builder.codex.md:3061:-// answers. Everything else is unimplemented (same shape as
.review/results/w9-3-test-harness-builder.codex.md:3063:+// answers. Everything else is unimplemented. Served through the
.review/results/w9-3-test-harness-builder.codex.md:4144:crates/blit-daemon/src/service/pull_sync.rs:1484:    module: &ModuleConfig,
.review/results/w9-3-test-harness-builder.codex.md:4319:crates/blit-cli/tests/remote_remote.rs:200:fn stale_destination_unimplemented...
.review/results/w9-3-test-harness-builder.codex.md:5660:464	// answers. Everything else is unimplemented (same shape as
.review/results/w9-3-test-harness-builder.codex.md:6136:308	// answers. Everything else is unimplemented. Served through the
.review/results/w9-3-test-harness-builder.codex.md:6175:347	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6191:363	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6198:370	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6205:377	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6212:384	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6219:391	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6226:398	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6233:405	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6240:412	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6247:419	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6254:426	        Err(tonic::Status::unimplemented("stalling fake source"))
.review/results/w9-3-test-harness-builder.codex.md:6462:200	fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
.review/results/w9-3-test-harness-builder.codex.md:6467:205	    let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimpleme...
  +17 more in .review/results/w9-3-test-harness-builder.codex.md
crates/blit-app/src/transfers/filter.rs:4://! Pre-A.0 the struct had a `from_transfer(&TransferArgs)`
crates/blit-cli/src/main.rs:59:run_transfer(&ctx, &args, TransferKind::Copy)
crates/blit-cli/src/main.rs:66:run_transfer(&ctx, &args, TransferKind::Mirror)
crates/blit-cli/src/transfers/local.rs:15:pub async fn run_local_transfer(
crates/blit-cli/src/transfers/mod.rs:101:pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: Transf...
crates/blit-cli/src/transfers/mod.rs:239:run_local_transfer(ctx, args, &src, &dst, mirror)
crates/blit-cli/src/transfers/mod.rs:249:run_remote_push_transfer(args, Endpoint::Local(src), dst, mirror).await
crates/blit-cli/src/transfers/mod.rs:254:run_remote_pull_transfer(
crates/blit-cli/src/transfers/mod.rs:277:run_remote_push_transfer(args, Endpoint::Remote(src), dst, mirror).await
crates/blit-cli/src/transfers/mod.rs:720:runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:769:runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:854:.block_on(run_transfer(&ctx, &args, TransferKind::Copy))
crates/blit-cli/src/transfers/mod.rs:868:.block_on(run_transfer(&ctx, &args, TransferKind::Copy))
crates/blit-cli/src/transfers/mod.rs:906:.block_on(run_transfer(&ctx, &args, TransferKind::Mirror))
crates/blit-cli/src/transfers/mod.rs:930:.block_on(run_transfer(&ctx, &args, TransferKind::Copy))
crates/blit-cli/src/transfers/mod.rs:959:.block_on(run_transfer(&ctx, &args, TransferKind::Mirror))
crates/blit-cli/src/transfers/remote.rs:161:pub async fn run_remote_push_transfer(
crates/blit-cli/src/transfers/remote.rs:271:pub async fn run_remote_pull_transfer(
crates/blit-cli/tests/jobs_lifecycle.rs:308:// answers. Everything else is unimplemented. Served through the
crates/blit-cli/tests/jobs_lifecycle.rs:344:Result<blit_core::generated::TransferFrame, tonic::Status>,
crates/blit-cli/tests/jobs_lifecycle.rs:349:async fn transfer(
crates/blit-cli/tests/jobs_lifecycle.rs:351:_: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
crates/blit-cli/tests/jobs_lifecycle.rs:353:Err(tonic::Status::unimplemented("otp-1 stub"))
crates/blit-cli/tests/jobs_lifecycle.rs:360:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:376:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:383:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:390:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:397:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:404:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:411:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:418:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:425:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:432:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:439:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:446:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/jobs_lifecycle.rs:453:Err(tonic::Status::unimplemented("stalling fake source"))
crates/blit-cli/tests/remote_remote.rs:200:fn stale_destination_unimplemented_does_not_fall_back_to_relay() {
crates/blit-cli/tests/remote_remote.rs:205:let stale = spawn_fake_blit_server(UnimplementedBlit, "fake unimplemented des...
crates/blit-cli/tests/remote_remote.rs:361:Result<blit_core::generated::TransferFrame, tonic::Status>,
crates/blit-cli/tests/remote_remote.rs:366:async fn transfer(
crates/blit-cli/tests/remote_remote.rs:368:_: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
crates/blit-cli/tests/remote_remote.rs:370:Err(tonic::Status::unimplemented("otp-1 stub"))
crates/blit-cli/tests/remote_remote.rs:377:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:384:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:391:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:398:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:405:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:412:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:419:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:426:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:433:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:440:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:447:Err(tonic::Status::unimplemented("stale daemon"))
crates/blit-cli/tests/remote_remote.rs:454:Err(tonic::Status::unimplemented("stale daemon"))
  +19 more in crates/blit-cli/tests/remote_remote.rs
+35 more files

codex
The wire table and proto oneof match at a first pass, but I’m checking for hidden contract holes around `NeedBatch`, resume block ordering, data-plane credentials, and the in-stream carrier, since those are the places implementers can accidentally fork behavior later.
exec
/usr/bin/zsh -lc "rtk nl -ba crates/blit-daemon/src/service/core.rs | rtk sed -n '1,180p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk nl -ba crates/blit-daemon/src/service/transfer.rs | rtk sed -n '1,180p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use super::admin::{
     2	    delete_rel_paths, filesystem_stats_for_path, list_completions, sanitize_request_paths,
     3	    split_completion_prefix, stream_disk_usage, stream_find_entries,
     4	};
     5	use super::pull_sync::handle_pull_sync_stream;
     6	use super::push::handle_push_stream;
     7	use super::util::{
     8	    metadata_mtime_seconds, resolve_contained_path, resolve_module, resolve_relative_path,
     9	};
    10	use super::{DiskUsageSender, FindSender};
    11	use crate::active_jobs::{ActiveJobKind, ActiveJobs, CancelOutcome};
    12	use crate::metrics::TransferMetrics;
    13	use crate::runtime::{ModuleConfig, RootExport};
    14	use blit_core::generated::blit_server::Blit;
    15	pub use blit_core::generated::blit_server::BlitServer;
    16	use blit_core::generated::{
    17	    daemon_event, ActiveTransfer, CancelJobRequest, CancelJobResponse, ClearRecentRequest,
    18	    ClearRecentResponse, ClientPullMessage, ClientPushRequest, CompletionRequest,
    19	    CompletionResponse, Counters, DaemonEvent, DaemonState, DelegatedPullProgress,
    20	    DelegatedPullRequest, DiskUsageEntry, DiskUsageRequest, FileInfo, FilesystemStatsRequest,
    21	    FilesystemStatsResponse, FindEntry, FindRequest, GetStateRequest, ListModulesRequest,
    22	    ListModulesResponse, ListRequest, ListResponse, ModuleInfo, PurgeRequest, PurgeResponse,
    23	    ServerPullMessage, ServerPushResponse, SubscribeRequest, TransferComplete, TransferError,
    24	    TransferProgress, TransferRecord, TransferStarted,
    25	};
    26	use std::collections::HashMap;
    27	use std::fs;
    28	use std::path::PathBuf;
    29	use std::sync::Arc;
    30	use tokio::sync::{broadcast, mpsc, Mutex};
    31	use tokio_stream::wrappers::ReceiverStream;
    32	use tokio_util::sync::CancellationToken;
    33	use tonic::{Request, Response, Status, Streaming};
    34	
    35	/// Capacity of the daemon's `Subscribe` event broadcast channel.
    36	/// Sized for a handful of subscribers (operator TUI + maybe a
    37	/// Prometheus scraper bridge) plus burst headroom — enough that a
    38	/// momentary stall on the subscriber side doesn't immediately drop
    39	/// events. Slow consumers that lag more than this many events behind
    40	/// receive a `tonic::Status::aborted` and re-subscribe.
    41	const SUBSCRIBE_BROADCAST_CAPACITY: usize = 256;
    42	
    43	/// Capacity of the per-subscriber `mpsc` buffer behind the
    44	/// c-5a Subscribe forwarder. Sized so a momentary client stall
    45	/// (one or two tick intervals' worth of matching events) doesn't
    46	/// back up into the forwarder. Smaller than
    47	/// `SUBSCRIBE_BROADCAST_CAPACITY` because the filter is already
    48	/// applied — the buffer holds only events the client wanted.
    49	/// A client whose mpsc fills causes the forwarder to block on
    50	/// `send().await`, which eventually triggers a broadcast Lagged
    51	/// when global event rate exceeds the broadcast ring capacity —
    52	/// the correct "this client really is too slow" signal.
    53	const SUBSCRIBE_MPSC_CAPACITY: usize = 64;
    54	
    55	/// Cadence of the c-4 progress ticker. Default 100ms (10 Hz) —
    56	/// matches the TUI_DESIGN.md §6.2 step-3 estimate. The cost is one
    57	/// broadcast event per active transfer per tick, so at typical
    58	/// active-counts of 1-4 transfers we send 10-40 events/sec.
    59	/// Subscribers that can't keep up get the c-2 Lagged → Status::aborted
    60	/// path; the broadcast itself never blocks the ticker.
    61	pub const DEFAULT_PROGRESS_TICK_MS: u64 = 100;
    62	
    63	pub struct BlitService {
    64	    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    65	    default_root: Option<RootExport>,
    66	    force_grpc_data: bool,
    67	    server_checksums_enabled: bool,
    68	    metrics: Arc<TransferMetrics>,
    69	    /// Delegation gate config. The handler reads it on every
    70	    /// `DelegatedPull` request; default-disabled means no caller can
    71	    /// make this daemon initiate outbound connects until the operator
    72	    /// flips `[delegation] allow_delegated_pull = true`.
    73	    pub(crate) delegation: Arc<crate::delegation_gate::DelegationConfig>,
    74	    /// Always-on registry of in-flight transfers. Populated
    75	    /// from the dispatch boundary in this file; read by
    76	    /// `GetState.active[]` once that RPC lands (milestone B
    77	    /// sub-slice). See `crate::active_jobs`.
    78	    pub(crate) active_jobs: ActiveJobs,
    79	    /// Daemon-side event broadcast channel feeding the `Subscribe`
    80	    /// RPC. Producers (dispatch sites in this file) send `DaemonEvent`
    81	    /// payloads; subscribers receive their own `Receiver` via
    82	    /// `events_tx.subscribe()`. Default capacity
    83	    /// `SUBSCRIBE_BROADCAST_CAPACITY` (256) — slow subscribers that
    84	    /// lag past that get a `tonic::Status::aborted` and re-subscribe.
    85	    events_tx: broadcast::Sender<DaemonEvent>,
    86	    /// Wall-clock at construction. `GetState.uptime_seconds`
    87	    /// reports `Instant::now().duration_since(started_at)`.
    88	    /// Captured once so a clock jump between construction and
    89	    /// the GetState call doesn't show up as negative uptime.
    90	    started_at: std::time::Instant,
    91	}
    92	
    93	impl BlitService {
    94	    pub(crate) fn from_runtime(
    95	        modules: HashMap<String, ModuleConfig>,
    96	        default_root: Option<RootExport>,
    97	        force_grpc_data: bool,
    98	        server_checksums_enabled: bool,
    99	        metrics: Arc<TransferMetrics>,
   100	        delegation: crate::delegation_gate::DelegationConfig,
   101	    ) -> Self {
   102	        let (events_tx, _) = broadcast::channel(SUBSCRIBE_BROADCAST_CAPACITY);
   103	        Self {
   104	            modules: Arc::new(Mutex::new(modules)),
   105	            default_root,
   106	            force_grpc_data,
   107	            server_checksums_enabled,
   108	            metrics,
   109	            delegation: Arc::new(delegation),
   110	            active_jobs: ActiveJobs::new(),
   111	            events_tx,
   112	            started_at: std::time::Instant::now(),
   113	        }
   114	    }
   115	
   116	    #[cfg(test)]
   117	    #[allow(dead_code)]
   118	    pub(crate) fn with_modules(
   119	        modules: HashMap<String, ModuleConfig>,
   120	        force_grpc_data: bool,
   121	    ) -> Self {
   122	        Self::from_runtime(
   123	            modules,
   124	            None,
   125	            force_grpc_data,
   126	            true,
   127	            TransferMetrics::disabled(),
   128	            crate::delegation_gate::DelegationConfig::default(),
   129	        )
   130	    }
   131	
   132	    /// Inner purge body. Extracted from the trait method so the
   133	    /// `--metrics` completion log can wrap a single call site and
   134	    /// branch on Result without duplicating the response shape.
   135	    /// §3.1 followup.
   136	    async fn purge_inner(&self, req: PurgeRequest) -> Result<Response<PurgeResponse>, Status> {
   137	        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
   138	        if module.read_only {
   139	            return Err(Status::permission_denied(format!(
   140	                "module '{}' is read-only",
   141	                module.name
   142	            )));
   143	        }
   144	        let sanitized = sanitize_request_paths(req.paths_to_delete)?;
   145	        if sanitized.is_empty() {
   146	            return Ok(Response::new(PurgeResponse { files_deleted: 0 }));
   147	        }
   148	        let stats = delete_rel_paths(
   149	            module.path.clone(),
   150	            module.canonical_root.clone(),
   151	            sanitized,
   152	        )
   153	        .await?;
   154	        Ok(Response::new(PurgeResponse {
   155	            files_deleted: stats.total(),
   156	        }))
   157	    }
   158	
   159	    /// Send a `TransferStarted` event onto the broadcast channel.
   160	    /// Called from each RPC dispatch site immediately after the
   161	    /// `ActiveJob` is registered, with the same values that
   162	    /// populated the row. A `SendError` return from `broadcast::Sender::send`
   163	    /// just means there are no current subscribers — that is the
   164	    /// normal state and we ignore it.
   165	    ///
   166	    /// Caller-passed values rather than re-reading from the
   167	    /// `ActiveJobGuard`: the dispatch site already has all the
   168	    /// inputs in scope as locals, and using them here avoids a
   169	    /// table lookup + clone on every transfer. Module/path are
   170	    /// empty strings for streaming RPCs at registration time;
   171	    /// that matches `GetState.active[]`'s view of the same row.
   172	    pub(crate) fn emit_transfer_started(
   173	        &self,
   174	        guard: &crate::active_jobs::ActiveJobGuard,
   175	        kind: ActiveJobKind,
   176	        peer: &str,
   177	        module: &str,
   178	        path: &str,
   179	    ) {
   180	        let event = DaemonEvent {

 succeeded in 0ms:
     1	//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
     2	//!
     3	//! otp-1 (D-2026-07-05-4) lands the wire surface only: the RPC, the
     4	//! frame set, and the contract (`docs/TRANSFER_SESSION.md`). The
     5	//! handler in `core.rs` refuses with UNIMPLEMENTED — pinned below —
     6	//! until otp-3/otp-4 land the role-tagged session state machine,
     7	//! which will live in this module.
     8	
     9	#[cfg(test)]
    10	mod tests {
    11	    //! otp-1 pin: the `Transfer` RPC exists on the wire (same-build
    12	    //! peers can reach it) and refuses with UNIMPLEMENTED — not
    13	    //! UNKNOWN/NOT_FOUND — until the session lands. If the RPC
    14	    //! vanished from the proto this file would not compile; if the
    15	    //! stub's contract changed this test fails.
    16	
    17	    use std::collections::HashMap;
    18	
    19	    use blit_core::generated::blit_client::BlitClient;
    20	    use blit_core::generated::blit_server::BlitServer;
    21	    use blit_core::generated::TransferFrame;
    22	
    23	    use crate::service::BlitService;
    24	
    25	    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    26	    async fn transfer_rpc_exists_and_refuses_unimplemented() {
    27	        let service = BlitService::with_modules(HashMap::new(), false);
    28	        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
    29	            .await
    30	            .expect("bind loopback listener");
    31	        let port = listener.local_addr().expect("listener addr").port();
    32	        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    33	        let server = tokio::spawn(async move {
    34	            blit_core::remote::grpc_server::production_server_builder()
    35	                .add_service(BlitServer::new(service))
    36	                .serve_with_incoming_shutdown(
    37	                    tokio_stream::wrappers::TcpListenerStream::new(listener),
    38	                    async {
    39	                        let _ = shutdown_rx.await;
    40	                    },
    41	                )
    42	                .await
    43	                .expect("in-process daemon serves");
    44	        });
    45	
    46	        let mut client = BlitClient::connect(format!("http://127.0.0.1:{port}"))
    47	            .await
    48	            .expect("client connects");
    49	        let outbound = tokio_stream::iter(vec![TransferFrame { frame: None }]);
    50	        let status = match client.transfer(outbound).await {
    51	            Err(status) => status,
    52	            Ok(mut streaming) => streaming
    53	                .get_mut()
    54	                .message()
    55	                .await
    56	                .expect_err("otp-1 stub must refuse"),
    57	        };
    58	        assert_eq!(
    59	            status.code(),
    60	            tonic::Code::Unimplemented,
    61	            "Transfer must exist on the wire and refuse with \
    62	             UNIMPLEMENTED until otp-3/otp-4; got: {status}"
    63	        );
    64	
    65	        let _ = shutdown_tx.send(());
    66	        server.await.expect("server task joins");
    67	    }
    68	}

exec
/usr/bin/zsh -lc 'rtk git show --no-ext-diff --unified=50 --no-renames a3e2acb -- crates/blit-cli/tests/jobs_lifecycle.rs crates/blit-cli/tests/remote_remote.rs crates/blit-core/tests/pull_sync_with_spec_wire.rs crates/blit-daemon/src/service/core.rs crates/blit-daemon/src/service/mod.rs crates/blit-daemon/src/service/transfer.rs' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk nl -ba crates/blit-daemon/src/service/mod.rs | rtk sed -n '1,160p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	mod admin;
     2	mod core;
     3	pub(crate) mod delegated_pull;
     4	mod pull_sync;
     5	mod push;
     6	mod transfer;
     7	mod util;
     8	
     9	pub use core::{spawn_progress_ticker, BlitServer, BlitService};
    10	
    11	use blit_core::generated::{DiskUsageEntry, FindEntry, ServerPullMessage, ServerPushResponse};
    12	use tokio::sync::mpsc;
    13	use tonic::Status;
    14	
    15	pub(crate) type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
    16	pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
    17	pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
    18	pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;

 succeeded in 0ms:
commit a3e2acb651ae8893127f2d75835d047695e4154c
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 12:29:54 2026 -0400

    otp-1: unified Transfer session wire contract (doc + proto + stubs)
    
    docs/TRANSFER_SESSION.md is the authoritative contract: one
    role-tagged frame vocabulary both wire directions, exact-match
    same-build handshake first (D-2026-07-05-2), destination-owned diff,
    dial contract carried (D-2026-06-20-1/-2), sf-2 shape correction as
    the only stream policy, resume as a strictly-ordered RELIABLE
    exception, mirror destination-local. proto gains the Transfer RPC +
    session messages (20-arm TransferFrame reusing the engine payload
    vocabulary; no PeerCapabilities, no spec_version, no delete list).
    Handler + five test fakes stub UNIMPLEMENTED; in-process pin proves
    the RPC is reachable and refusing. Suite 1483 -> 1484.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

diff --git a/crates/blit-cli/tests/jobs_lifecycle.rs b/crates/blit-cli/tests/jobs_lifecycle.rs
index 44beb2c..3d8ec5e 100644
--- a/crates/blit-cli/tests/jobs_lifecycle.rs
+++ b/crates/blit-cli/tests/jobs_lifecycle.rs
@@ -293,100 +293,113 @@ fn cancel_of_active_delegated_job_exits_zero() {
     let transfer_id = detach_copy(&ctx);
 
     let dest_host = ctx.dest_host();
     let cancel = ctx.run_blit(&["jobs", "cancel", &dest_host, &transfer_id]);
     assert_eq!(
         cancel.status.code(),
         Some(0),
         "cancel of an active delegated job must exit 0 (Cancelled)\nstdout:\n{}\nstderr:\n{}",
         String::from_utf8_lossy(&cancel.stdout),
         String::from_utf8_lossy(&cancel.stderr)
     );
 }
 
 // ---------------------------------------------------------------
 // Fake stalling source: a tonic server whose pull_sync never
 // answers. Everything else is unimplemented. Served through the
 // shared production-shaped scaffold (common::spawn_fake_blit_server).
 // ---------------------------------------------------------------
 
 fn spawn_stalling_source() -> common::FakeServerGuard {
     spawn_fake_blit_server(StallingPullSyncBlit, "fake stalling source")
 }
 
 struct StallingPullSyncBlit;
 
 #[tonic::async_trait]
 impl blit_core::generated::blit_server::Blit for StallingPullSyncBlit {
     type PushStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPushResponse, tonic::Status>,
     >;
     type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPullMessage, tonic::Status>,
     >;
     type FindStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::FindEntry, tonic::Status>,
     >;
     type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
     >;
     type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
     >;
     type SubscribeStream = std::pin::Pin<
         Box<
             dyn tokio_stream::Stream<
                     Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                 > + Send,
         >,
     >;
 
+    type TransferStream = tokio_stream::wrappers::ReceiverStream<
+        Result<blit_core::generated::TransferFrame, tonic::Status>,
+    >;
+
+    // otp-1: unified-session wire surface; fakes refuse like the
+    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
+    async fn transfer(
+        &self,
+        _: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
+        Err(tonic::Status::unimplemented("otp-1 stub"))
+    }
+
     async fn push(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
     ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
 
     /// The point of this fake: accept the RPC and never answer.
     async fn pull_sync(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
     ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
         std::future::pending::<()>().await;
         unreachable!("pending() never resolves")
     }
 
     async fn subscribe(
         &self,
         _: tonic::Request<blit_core::generated::SubscribeRequest>,
     ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
 
     async fn list(
         &self,
         _: tonic::Request<blit_core::generated::ListRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
 
     async fn purge(
         &self,
         _: tonic::Request<blit_core::generated::PurgeRequest>,
     ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
 
     async fn complete_path(
         &self,
         _: tonic::Request<blit_core::generated::CompletionRequest>,
     ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
 
     async fn list_modules(
         &self,
         _: tonic::Request<blit_core::generated::ListModulesRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stalling fake source"))
     }
diff --git a/crates/blit-cli/tests/remote_remote.rs b/crates/blit-cli/tests/remote_remote.rs
index 1028130..6e205f7 100644
--- a/crates/blit-cli/tests/remote_remote.rs
+++ b/crates/blit-cli/tests/remote_remote.rs
@@ -310,100 +310,113 @@ fn read_counters(path: &Path) -> CounterValues {
     };
     for line in contents.lines() {
         let mut parts = line.split_whitespace();
         let Some(name) = parts.next() else { continue };
         let value = parts
             .next()
             .and_then(|v| v.parse::<u64>().ok())
             .unwrap_or(0);
         match name {
             "cli_data_plane_outbound_bytes" => {
                 out.cli_data_plane_outbound_bytes =
                     out.cli_data_plane_outbound_bytes.saturating_add(value);
             }
             "remote_transfer_source_constructed" => {
                 out.remote_transfer_source_constructed =
                     out.remote_transfer_source_constructed.saturating_add(value);
             }
             _ => {}
         }
     }
     out
 }
 
 struct UnimplementedBlit;
 
 #[tonic::async_trait]
 impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
     type PushStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPushResponse, tonic::Status>,
     >;
     type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPullMessage, tonic::Status>,
     >;
     type FindStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::FindEntry, tonic::Status>,
     >;
     type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
     >;
     type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
     >;
     type SubscribeStream = std::pin::Pin<
         Box<
             dyn tokio_stream::Stream<
                     Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                 > + Send,
         >,
     >;
 
+    type TransferStream = tokio_stream::wrappers::ReceiverStream<
+        Result<blit_core::generated::TransferFrame, tonic::Status>,
+    >;
+
+    // otp-1: unified-session wire surface; fakes refuse like the
+    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
+    async fn transfer(
+        &self,
+        _: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
+        Err(tonic::Status::unimplemented("otp-1 stub"))
+    }
+
     async fn push(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
     ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn pull_sync(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
     ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn subscribe(
         &self,
         _: tonic::Request<blit_core::generated::SubscribeRequest>,
     ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn list(
         &self,
         _: tonic::Request<blit_core::generated::ListRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn purge(
         &self,
         _: tonic::Request<blit_core::generated::PurgeRequest>,
     ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn complete_path(
         &self,
         _: tonic::Request<blit_core::generated::CompletionRequest>,
     ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn list_modules(
         &self,
         _: tonic::Request<blit_core::generated::ListModulesRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ListModulesResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn find(
@@ -436,100 +449,113 @@ impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
 
     async fn get_state(
         &self,
         _: tonic::Request<blit_core::generated::GetStateRequest>,
     ) -> Result<tonic::Response<blit_core::generated::DaemonState>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn cancel_job(
         &self,
         _: tonic::Request<blit_core::generated::CancelJobRequest>,
     ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 
     async fn clear_recent(
         &self,
         _: tonic::Request<blit_core::generated::ClearRecentRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ClearRecentResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented("stale daemon"))
     }
 }
 
 struct RejectingPullSyncBlit;
 
 #[tonic::async_trait]
 impl blit_core::generated::blit_server::Blit for RejectingPullSyncBlit {
     type PushStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPushResponse, tonic::Status>,
     >;
     type PullSyncStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::ServerPullMessage, tonic::Status>,
     >;
     type FindStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::FindEntry, tonic::Status>,
     >;
     type DiskUsageStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DiskUsageEntry, tonic::Status>,
     >;
     type DelegatedPullStream = tokio_stream::wrappers::ReceiverStream<
         Result<blit_core::generated::DelegatedPullProgress, tonic::Status>,
     >;
     type SubscribeStream = std::pin::Pin<
         Box<
             dyn tokio_stream::Stream<
                     Item = Result<blit_core::generated::DaemonEvent, tonic::Status>,
                 > + Send,
         >,
     >;
 
+    type TransferStream = tokio_stream::wrappers::ReceiverStream<
+        Result<blit_core::generated::TransferFrame, tonic::Status>,
+    >;
+
+    // otp-1: unified-session wire surface; fakes refuse like the
+    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
+    async fn transfer(
+        &self,
+        _: tonic::Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<tonic::Response<Self::TransferStream>, tonic::Status> {
+        Err(tonic::Status::unimplemented("otp-1 stub"))
+    }
+
     async fn push(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPushRequest>>,
     ) -> Result<tonic::Response<Self::PushStream>, tonic::Status> {
         Err(tonic::Status::unimplemented(
             "test only exercises pull_sync",
         ))
     }
 
     async fn pull_sync(
         &self,
         _: tonic::Request<tonic::Streaming<blit_core::generated::ClientPullMessage>>,
     ) -> Result<tonic::Response<Self::PullSyncStream>, tonic::Status> {
         Err(tonic::Status::permission_denied(
             "source ACL rejected delegated peer",
         ))
     }
 
     async fn subscribe(
         &self,
         _: tonic::Request<blit_core::generated::SubscribeRequest>,
     ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
         Err(tonic::Status::unimplemented(
             "test only exercises pull_sync",
         ))
     }
 
     async fn list(
         &self,
         _: tonic::Request<blit_core::generated::ListRequest>,
     ) -> Result<tonic::Response<blit_core::generated::ListResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented(
             "test only exercises pull_sync",
         ))
     }
 
     async fn purge(
         &self,
         _: tonic::Request<blit_core::generated::PurgeRequest>,
     ) -> Result<tonic::Response<blit_core::generated::PurgeResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented(
             "test only exercises pull_sync",
         ))
     }
 
     async fn complete_path(
         &self,
         _: tonic::Request<blit_core::generated::CompletionRequest>,
     ) -> Result<tonic::Response<blit_core::generated::CompletionResponse>, tonic::Status> {
         Err(tonic::Status::unimplemented(
diff --git a/crates/blit-core/tests/pull_sync_with_spec_wire.rs b/crates/blit-core/tests/pull_sync_with_spec_wire.rs
index cbf0378..8cbd6a7 100644
--- a/crates/blit-core/tests/pull_sync_with_spec_wire.rs
+++ b/crates/blit-core/tests/pull_sync_with_spec_wire.rs
@@ -13,100 +13,111 @@
 //! The stub server implements the full `Blit` trait. Methods other
 //! than `pull_sync` panic if hit — the test only exercises one RPC.
 
 use std::sync::Arc;
 use std::time::Duration;
 
 use blit_core::generated::blit_server::{Blit, BlitServer};
 use blit_core::generated::{
     client_pull_message, server_pull_message, ClientPullMessage, ClientPushRequest,
     CompletionRequest, CompletionResponse, DelegatedPullProgress, DelegatedPullRequest,
     DiskUsageEntry, DiskUsageRequest, FileHeader, FilesystemStatsRequest, FilesystemStatsResponse,
     FindEntry, FindRequest, ListModulesRequest, ListModulesResponse, ListRequest, ListResponse,
     PeerCapabilities, PullSyncAck, PurgeRequest, PurgeResponse, ServerPullMessage,
     ServerPushResponse, TransferOperationSpec,
 };
 use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
 use blit_core::remote::pull::{PullSyncError, RemotePullClient};
 use tokio::sync::Mutex;
 use tokio_stream::wrappers::ReceiverStream;
 // Fake servers start from the shared production-shaped builder
 // (blit_core::remote::grpc_server) so this wire-contract harness
 // carries the deployed HTTP/2 keepalive config (w9-3).
 use blit_core::remote::grpc_server::production_server_builder;
 use tonic::{Request, Response, Status, Streaming};
 
 /// Stub `Blit` impl that captures the first incoming
 /// `ClientPullMessage::Spec` and immediately ends the response stream
 /// after sending a benign `PullSyncAck`. That makes
 /// `pull_sync_with_spec` return without doing any data-plane setup or
 /// transfer work — the only thing we care about is the spec byte
 /// shape arriving on the server side.
 struct SpyServer {
     captured: Arc<Mutex<Option<TransferOperationSpec>>>,
     reject_pull_sync: Option<(tonic::Code, &'static str)>,
 }
 
 #[tonic::async_trait]
 impl Blit for SpyServer {
     type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
     type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
     type FindStream = ReceiverStream<Result<FindEntry, Status>>;
     type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
     type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
     type SubscribeStream = std::pin::Pin<
         Box<
             dyn tokio_stream::Stream<Item = Result<blit_core::generated::DaemonEvent, Status>>
                 + Send,
         >,
     >;
 
+    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
+
+    // otp-1: unified-session wire surface; fakes refuse like the
+    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
+    async fn transfer(
+        &self,
+        _: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<Response<Self::TransferStream>, Status> {
+        Err(Status::unimplemented("otp-1 stub"))
+    }
+
     async fn subscribe(
         &self,
         _: Request<blit_core::generated::SubscribeRequest>,
     ) -> Result<Response<Self::SubscribeStream>, Status> {
         unimplemented!("test only exercises pull_sync")
     }
 
     async fn push(
         &self,
         _: Request<Streaming<ClientPushRequest>>,
     ) -> Result<Response<Self::PushStream>, Status> {
         unimplemented!("test only exercises pull_sync")
     }
 
     async fn pull_sync(
         &self,
         request: Request<Streaming<ClientPullMessage>>,
     ) -> Result<Response<Self::PullSyncStream>, Status> {
         if let Some((code, message)) = self.reject_pull_sync {
             return Err(Status::new(code, message));
         }
         let captured = Arc::clone(&self.captured);
         let mut stream = request.into_inner();
         let (tx, rx) = tokio::sync::mpsc::channel(8);
 
         tokio::spawn(async move {
             // The client sends Spec as the very first message in
             // pull_sync_with_spec. Capture it, then close the stream.
             while let Ok(Some(msg)) = stream.message().await {
                 if let Some(client_pull_message::Payload::Spec(spec)) = msg.payload {
                     *captured.lock().await = Some(spec);
                     // Send a PullSyncAck so the client can return
                     // cleanly without hitting --checksum mismatch
                     // logic. Immediately drop tx so the stream ends.
                     let _ = tx
                         .send(Ok(ServerPullMessage {
                             payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
                                 server_checksums_enabled: true,
                             })),
                         }))
                         .await;
                     break;
                 }
             }
             // dropping tx here closes the response stream
         });
 
         Ok(Response::new(ReceiverStream::new(rx)))
     }
 
@@ -429,100 +440,111 @@ async fn pull_sync_with_spec_classifies_initial_rpc_rejection_as_negotiation() {
     let pull_err = err
         .downcast_ref::<PullSyncError>()
         .expect("initial pull_sync RPC rejection should preserve PullSyncError");
     assert!(
         pull_err.is_negotiation(),
         "initial RPC rejection must be classified as negotiation: {err}"
     );
     assert!(
         err.to_string()
             .contains("source ACL rejected delegated peer"),
         "source rejection reason should survive, got: {err}"
     );
 }
 
 // ─── ue-r2-1h: relay session wire tests ──────────────────────────────
 //
 // `scan_remote_files` and `open_remote_file` (the remote→remote
 // relay's primitives) rode the deprecated Pull RPC until ue-r2-1h
 // deleted it; they now open their own PullSync sessions. These tests
 // pin the client half of that port against a daemon-shaped frame
 // script: the spec each session sends, the frames it consumes, and
 // the mixed-version degradation the proto comment promises
 // (an old daemon ignoring `metadata_only` streams data — the scan
 // must still return exactly the headers).
 
 /// `Blit` impl that captures the pull_sync spec and then plays back a
 /// fixed frame script. Unlike `SpyServer` it never inspects the
 /// client's manifest phase — the relay sessions send an empty
 /// manifest and the script is unconditional. ue-r2-2: after the spec
 /// it keeps draining the client stream, capturing any resize acks.
 struct CannedFramesServer {
     captured: Arc<Mutex<Option<TransferOperationSpec>>>,
     frames: Vec<server_pull_message::Payload>,
     acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>>,
 }
 
 #[tonic::async_trait]
 impl Blit for CannedFramesServer {
     type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
     type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
     type FindStream = ReceiverStream<Result<FindEntry, Status>>;
     type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
     type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
     type SubscribeStream = std::pin::Pin<
         Box<
             dyn tokio_stream::Stream<Item = Result<blit_core::generated::DaemonEvent, Status>>
                 + Send,
         >,
     >;
 
+    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
+
+    // otp-1: unified-session wire surface; fakes refuse like the
+    // real service until otp-3/otp-4 (docs/TRANSFER_SESSION.md).
+    async fn transfer(
+        &self,
+        _: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<Response<Self::TransferStream>, Status> {
+        Err(Status::unimplemented("otp-1 stub"))
+    }
+
     async fn pull_sync(
         &self,
         request: Request<Streaming<ClientPullMessage>>,
     ) -> Result<Response<Self::PullSyncStream>, Status> {
         let captured = Arc::clone(&self.captured);
         let acks = Arc::clone(&self.acks);
         let frames = self.frames.clone();
         let mut stream = request.into_inner();
         let (tx, rx) = tokio::sync::mpsc::channel(16);
 
         tokio::spawn(async move {
             // Capture the spec, then play the script and close.
             while let Ok(Some(msg)) = stream.message().await {
                 if let Some(client_pull_message::Payload::Spec(spec)) = msg.payload {
                     *captured.lock().await = Some(spec);
                     break;
                 }
             }
             // ue-r2-2: keep the client stream drained so resize acks
             // are observable by tests.
             let ack_drain = tokio::spawn(async move {
                 while let Ok(Some(msg)) = stream.message().await {
                     if let Some(client_pull_message::Payload::DataPlaneResizeAck(ack)) = msg.payload
                     {
                         acks.lock().await.push(ack);
                     }
                 }
             });
             for payload in frames {
                 if tx
                     .send(Ok(ServerPullMessage {
                         payload: Some(payload),
                     }))
                     .await
                     .is_err()
                 {
                     break;
                 }
             }
             // Give the drain a beat to observe trailing acks, then
             // stop (dropping tx above ends the client loop anyway).
             tokio::time::sleep(Duration::from_millis(100)).await;
             ack_drain.abort();
         });
 
         Ok(Response::new(ReceiverStream::new(rx)))
     }
 
     async fn push(
         &self,
diff --git a/crates/blit-daemon/src/service/core.rs b/crates/blit-daemon/src/service/core.rs
index 1fc5777..bb09f1f 100644
--- a/crates/blit-daemon/src/service/core.rs
+++ b/crates/blit-daemon/src/service/core.rs
@@ -301,100 +301,113 @@ pub(crate) fn event_matches_filter(event: &DaemonEvent, filter: &str) -> bool {
     }
 }
 
 /// Build the terminal event for a transfer that's draining. Called
 /// from each RPC's spawn closure after `record_outcome` and before
 /// `drop(job)`, with the same `(ok, error_message)` pair that the
 /// ActiveJobs ring records. Pairs with `emit_transfer_started` on
 /// the receive side: every transfer that emitted Started will also
 /// emit either Complete or Error.
 ///
 /// Sourced from the guard so byte total and duration match what
 /// `GetState.recent[]` will surface on the same row.
 pub(crate) fn build_transfer_finished_event(
     guard: &crate::active_jobs::ActiveJobGuard,
     ok: bool,
     error_message: Option<&str>,
 ) -> DaemonEvent {
     if ok {
         DaemonEvent {
             payload: Some(daemon_event::Payload::TransferComplete(TransferComplete {
                 transfer_id: guard.transfer_id().to_string(),
                 bytes: guard.bytes_completed_load(),
                 // `files` is wired in a follow-up C sub-slice
                 // (file-level counter analogous to bytes).
                 files: 0,
                 duration_ms: guard.elapsed_ms(),
                 // `tcp_fallback_used` plumbs through the handler's
                 // result struct in a follow-up; false today.
                 tcp_fallback_used: false,
             })),
         }
     } else {
         DaemonEvent {
             payload: Some(daemon_event::Payload::TransferError(TransferError {
                 transfer_id: guard.transfer_id().to_string(),
                 message: error_message.unwrap_or("").to_string(),
             })),
         }
     }
 }
 
 #[tonic::async_trait]
 impl Blit for BlitService {
     type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
     type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
     type FindStream = ReceiverStream<Result<FindEntry, Status>>;
     type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
     type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
     type SubscribeStream =
         std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<DaemonEvent, Status>> + Send>>;
+    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
+
+    /// ONE_TRANSFER_PATH otp-1: the unified session's wire surface
+    /// exists compiled-but-refusing until otp-3/otp-4 land the
+    /// session behavior. Contract: docs/TRANSFER_SESSION.md.
+    async fn transfer(
+        &self,
+        _request: Request<tonic::Streaming<blit_core::generated::TransferFrame>>,
+    ) -> Result<Response<Self::TransferStream>, Status> {
+        Err(Status::unimplemented(
+            "Transfer session lands at ONE_TRANSFER_PATH otp-3/otp-4",
+        ))
+    }
 
     async fn subscribe(
         &self,
         request: Request<SubscribeRequest>,
     ) -> Result<Response<Self::SubscribeStream>, Status> {
         let req = request.into_inner();
         let transfer_id_filter = req.transfer_id_filter;
         // c-5b: atomically register a broadcast Receiver AND
         // snapshot the per-row event ring (if replay_recent &&
         // filter is non-empty AND the row exists). Both happen
         // under the table lock so no event can be observed
         // both via replay and via broadcast — see emit_event /
         // subscribe_with_ring rustdoc for the full ordering.
         let (mut broadcast_rx, replay) = self.active_jobs.subscribe_with_ring(
             &self.events_tx,
             &transfer_id_filter,
             req.replay_recent,
         );
 
         // c-5a round 2: per-subscriber forwarder. The round-1
         // shape (returning `BroadcastStream::filter_map` directly)
         // still advanced the subscriber's broadcast cursor
         // through every event — so a `jobs watch <id>` consumer
         // could be aborted with Lagged when unrelated transfers
         // overflowed the global ring, even though the filter
         // rejected those events anyway.
         //
         // Fix: spawn a task that eagerly drains the broadcast
         // (cursor stays caught up independent of client read
         // pace), applies the filter, and forwards only matching
         // events into a bounded per-subscriber `mpsc`. The mpsc
         // receiver is what tonic streams to the client.
         //
         // Lagged semantics now mean "the FORWARDER couldn't
         // keep up with global event rate" — a daemon-side CPU
         // problem, not "this client is slow on its filtered
         // subset." If the client is slow on the matching
         // subset the mpsc fills first, the forwarder's
         // `send().await` blocks, and Lagged eventually fires
         // through the normal broadcast over-capacity path —
         // which is the correct "this client really is too
         // slow" signal.
         let (tx, rx) = mpsc::channel::<Result<DaemonEvent, Status>>(SUBSCRIBE_MPSC_CAPACITY);
         tokio::spawn(async move {
             // c-5b: drain replay events first (empty Vec when
             // replay_recent=false or filter is empty or row
             // doesn't exist). The forwarder then transitions
             // to live broadcast forwarding. Note that replay
             // events have ALREADY been deduped against the
             // broadcast Receiver under the table lock — the
diff --git a/crates/blit-daemon/src/service/mod.rs b/crates/blit-daemon/src/service/mod.rs
index 416b881..907d6b3 100644
--- a/crates/blit-daemon/src/service/mod.rs
+++ b/crates/blit-daemon/src/service/mod.rs
@@ -1,17 +1,18 @@
 mod admin;
 mod core;
 pub(crate) mod delegated_pull;
 mod pull_sync;
 mod push;
+mod transfer;
 mod util;
 
 pub use core::{spawn_progress_ticker, BlitServer, BlitService};
 
 use blit_core::generated::{DiskUsageEntry, FindEntry, ServerPullMessage, ServerPushResponse};
 use tokio::sync::mpsc;
 use tonic::Status;
 
 pub(crate) type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
 pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
 pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
 pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;
diff --git a/crates/blit-daemon/src/service/transfer.rs b/crates/blit-daemon/src/service/transfer.rs
new file mode 100644
index 0000000..2e61db3
--- /dev/null
+++ b/crates/blit-daemon/src/service/transfer.rs
@@ -0,0 +1,68 @@
+//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
+//!
+//! otp-1 (D-2026-07-05-4) lands the wire surface only: the RPC, the
+//! frame set, and the contract (`docs/TRANSFER_SESSION.md`). The
+//! handler in `core.rs` refuses with UNIMPLEMENTED — pinned below —
+//! until otp-3/otp-4 land the role-tagged session state machine,
+//! which will live in this module.
+
+#[cfg(test)]
+mod tests {
+    //! otp-1 pin: the `Transfer` RPC exists on the wire (same-build
+    //! peers can reach it) and refuses with UNIMPLEMENTED — not
+    //! UNKNOWN/NOT_FOUND — until the session lands. If the RPC
+    //! vanished from the proto this file would not compile; if the
+    //! stub's contract changed this test fails.
+
+    use std::collections::HashMap;
+
+    use blit_core::generated::blit_client::BlitClient;
+    use blit_core::generated::blit_server::BlitServer;
+    use blit_core::generated::TransferFrame;
+
+    use crate::service::BlitService;
+
+    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+    async fn transfer_rpc_exists_and_refuses_unimplemented() {
+        let service = BlitService::with_modules(HashMap::new(), false);
+        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
+            .await
+            .expect("bind loopback listener");
+        let port = listener.local_addr().expect("listener addr").port();
+        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
+        let server = tokio::spawn(async move {
+            blit_core::remote::grpc_server::production_server_builder()
+                .add_service(BlitServer::new(service))
+                .serve_with_incoming_shutdown(
+                    tokio_stream::wrappers::TcpListenerStream::new(listener),
+                    async {
+                        let _ = shutdown_rx.await;
+                    },
+                )
+                .await
+                .expect("in-process daemon serves");
+        });
+
+        let mut client = BlitClient::connect(format!("http://127.0.0.1:{port}"))
+            .await
+            .expect("client connects");
+        let outbound = tokio_stream::iter(vec![TransferFrame { frame: None }]);
+        let status = match client.transfer(outbound).await {
+            Err(status) => status,
+            Ok(mut streaming) => streaming
+                .get_mut()
+                .message()
+                .await
+                .expect_err("otp-1 stub must refuse"),
+        };
+        assert_eq!(
+            status.code(),
+            tonic::Code::Unimplemented,
+            "Transfer must exist on the wire and refuse with \
+             UNIMPLEMENTED until otp-3/otp-4; got: {status}"
+        );
+
+        let _ = shutdown_tx.send(());
+        server.await.expect("server task joins");
+    }
+}


exec
/usr/bin/zsh -lc "rtk nl -ba proto/blit.proto | rtk sed -n '1344,1385p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1344	// The single frame type BOTH wire directions carry. Which frames an
  1345	// end may send is determined by ROLE and phase
  1346	// (docs/TRANSFER_SESSION.md frame table — field numbers frozen
  1347	// there). Reuses the engine's existing payload vocabulary verbatim.
  1348	message TransferFrame {
  1349	  oneof frame {
  1350	    SessionHello hello = 1;
  1351	    SessionOpen open = 2;
  1352	    SessionAccept accept = 3;
  1353	    FileHeader manifest_entry = 4;
  1354	    ManifestComplete manifest_complete = 5;
  1355	    NeedBatch need_batch = 6;
  1356	    NeedComplete need_complete = 7;
  1357	    BlockHashList block_hashes = 8;
  1358	    FileHeader file_begin = 9;
  1359	    FileData file_data = 10;
  1360	    TarShardHeader tar_shard_header = 11;
  1361	    TarShardChunk tar_shard_chunk = 12;
  1362	    TarShardComplete tar_shard_complete = 13;
  1363	    BlockTransfer block = 14;
  1364	    BlockTransferComplete block_complete = 15;
  1365	    DataPlaneResize resize = 16;
  1366	    DataPlaneResizeAck resize_ack = 17;
  1367	    SourceDone source_done = 18;
  1368	    TransferSummary summary = 19;
  1369	    SessionError error = 20;
  1370	  }
  1371	}

exec
/usr/bin/zsh -lc "rtk sed -n '1,120p' .review/findings/otp-1-wire-session-contract.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# otp-1 — Unified Transfer session: wire + session contract

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-1.
**Status**: implemented, codex review pending.

## What

The complete wire surface of the single `Transfer` session that
replaces `Push` and `PullSync` at cutover — contract doc, proto, and
compiled-but-refusing stubs. No behavior: the daemon handler returns
UNIMPLEMENTED, pinned by test.

## Approach

- **`docs/TRANSFER_SESSION.md`** (new, the authoritative contract):
  role-tagged single frame vocabulary (`TransferFrame` both wire
  directions — no push-shaped or pull-shaped message set exists to
  diverge); exact-match same-build handshake as the FIRST frame each
  way (D-2026-07-05-2 — `build_id` + `contract_version`, mismatch →
  `SessionError{BUILD_MISMATCH}` naming both ids); phase state machine
  (hello → open/accept → concurrent manifest/need/payload → closing)
  with fail-fast `PROTOCOL_VIOLATION` on phase misuse; diff owner =
  DESTINATION always; dial contract carried (D-2026-06-20-1/-2:
  receiver capacity travels DESTINATION→SOURCE in open OR accept
  depending on who holds the role; absent/0 = unknown hardware, never
  "old peer"); sf-2 shape correction named as the only stream policy,
  SOURCE is resize controller in every session; transport facts
  (responder binds, initiator dials; in-stream carrier as byte-carrier
  option; local = in-process frame channel); resume RELIABLE exception
  (per-file `NeedEntry.resume` → destination `BlockHashList` strictly
  before that file's bytes); mirror destination-local (no delete list
  crosses the wire); error/cancel/StallGuard/jobs semantics.
- **`proto/blit.proto`**: `rpc Transfer(stream TransferFrame) returns
  (stream TransferFrame)` + `TransferRole`, `SessionHello`,
  `SessionOpen`, `SessionAccept`, `DataPlaneGrant`,
  `NeedEntry`/`NeedBatch`/`NeedComplete`, `SourceDone`,
  `TransferSummary` (one summary shape, DESTINATION→SOURCE),
  `SessionError` (structured refusal codes incl. BUILD_MISMATCH), and
  the 20-arm `TransferFrame` oneof reusing the engine's existing
  payload vocabulary verbatim (`FileHeader`, `FileData`, `TarShard*`,
  `Block*`, `DataPlaneResize`/`Ack`, `CapacityProfile`, `FilterSpec`,
  enums). Deliberately absent: `PeerCapabilities`, `spec_version`
  negotiation, delete lists, any per-direction message.
- **Stubs** (mechanical, required by tonic's non-optional trait
  methods): `BlitService::transfer` → UNIMPLEMENTED with a pointer to
  the plan; the five test fakes (remote_remote ×2, jobs_lifecycle,
  pull_sync_with_spec_wire ×2) gain the same refusing stub.
- **Future home staked**: `crates/blit-daemon/src/service/transfer.rs`
  holds the pin test now and becomes the session module at otp-4.

## Files

- `docs/TRANSFER_SESSION.md` (new — contract)
- `proto/blit.proto` (Transfer RPC + session messages; Push annotated
  with its otp-10 deletion notice)
- `crates/blit-daemon/src/service/core.rs` (stub handler)
- `crates/blit-daemon/src/service/transfer.rs` (new — pin test +
  future session home), `service/mod.rs` (registration)
- `crates/blit-cli/tests/remote_remote.rs`,
  `crates/blit-cli/tests/jobs_lifecycle.rs`,
  `crates/blit-core/tests/pull_sync_with_spec_wire.rs` (fake stubs)

## Tests

Suite 1483 → **1484 passed / 0 failed** (37 suites, same 2 ignored);
fmt + clippy clean.

- `transfer_rpc_exists_and_refuses_unimplemented` (in-process real
  service + real generated client): the RPC is reachable on the wire
  and refuses with UNIMPLEMENTED — not UNKNOWN — until otp-3/otp-4.
  Guard shape: if the RPC left the proto this test does not compile;
  if the stub's refusal contract changed it fails.

## Known gaps

- The handshake's `build_id` composition (version + git sha + dirty
  flag) is specified in the contract but not yet emitted by any build
  script — that lands with the first session behavior (otp-3), which
  is also when the mismatch-refusal test becomes writable.
- `TransferSummary` unifies Push/PullSummary on the wire; the CLI
  rendering migration happens at cutover (otp-10), not before.
- Frame table field numbers are frozen by the doc from this slice on;
  any change before cutover is a contract change and re-enters review.
- The contract's in-stream carrier reuses `FileHeader file_begin`
  framing; exact per-frame chunk sizing stays dial-owned (w2-2) and
  is not a wire constant.

exec
/usr/bin/zsh -lc 'rtk git diff --unified=0 a3e2acb''^ a3e2acb -- | rtk rg "''^[+-]'"\\s*#\\[(tokio::)?test|"'^[+-]'"\\s*(async\\s+)?fn\\s+[A-Za-z0-9_]+\\(\"" in /home/michael/dev/Blit
 succeeded in 0ms:
115:+    async fn transfer(
133:+    async fn transfer(
147:+    async fn transfer(
163:+    async fn transfer(
175:+    async fn transfer(
192:+    async fn transfer(
236:+    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
237:+    async fn transfer_rpc_exists_and_refuses_unimplemented() {

exec
/usr/bin/zsh -lc 'rtk git diff --numstat a3e2acb''^ a3e2acb' in /home/michael/dev/Blit
 succeeded in 0ms:
86	0	.review/findings/otp-1-wire-session-contract.md
13	0	crates/blit-cli/tests/jobs_lifecycle.rs
26	0	crates/blit-cli/tests/remote_remote.rs
22	0	crates/blit-core/tests/pull_sync_with_spec_wire.rs
13	0	crates/blit-daemon/src/service/core.rs
1	0	crates/blit-daemon/src/service/mod.rs
68	0	crates/blit-daemon/src/service/transfer.rs
184	0	docs/TRANSFER_SESSION.md
163	0	proto/blit.proto

codex
The stub side looks mechanically honest so far: the diff adds one real daemon stub plus the fake stubs required by the generated trait, and one new pin test. I’m doing one more pass for trait impl coverage and line-specific evidence before I write the findings.
exec
/usr/bin/zsh -lc "rtk rg -n \"type TransferStream|async fn transfer\\(\" crates --glob '*.rs'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "rtk rg -n \"impl (blit_core::generated::blit_server::)?Blit for|impl blit_core::generated::blit_server::Blit for|impl Blit for\" crates --glob '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/core.rs:351:    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
crates/blit-daemon/src/service/core.rs:356:    async fn transfer(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:63:    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
crates/blit-core/tests/pull_sync_with_spec_wire.rs:67:    async fn transfer(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:490:    type TransferStream = ReceiverStream<Result<blit_core::generated::TransferFrame, Status>>;
crates/blit-core/tests/pull_sync_with_spec_wire.rs:494:    async fn transfer(
crates/blit-cli/tests/jobs_lifecycle.rs:343:    type TransferStream = tokio_stream::wrappers::ReceiverStream<
crates/blit-cli/tests/jobs_lifecycle.rs:349:    async fn transfer(
crates/blit-cli/tests/remote_remote.rs:360:    type TransferStream = tokio_stream::wrappers::ReceiverStream<
crates/blit-cli/tests/remote_remote.rs:366:    async fn transfer(
crates/blit-cli/tests/remote_remote.rs:499:    type TransferStream = tokio_stream::wrappers::ReceiverStream<
crates/blit-cli/tests/remote_remote.rs:505:    async fn transfer(

 succeeded in 0ms:
crates/blit-daemon/src/service/core.rs:343:impl Blit for BlitService {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:50:impl Blit for SpyServer {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:477:impl Blit for CannedFramesServer {
crates/blit-cli/tests/remote_remote.rs:336:impl blit_core::generated::blit_server::Blit for UnimplementedBlit {
crates/blit-cli/tests/remote_remote.rs:475:impl blit_core::generated::blit_server::Blit for RejectingPullSyncBlit {
crates/blit-cli/tests/jobs_lifecycle.rs:319:impl blit_core::generated::blit_server::Blit for StallingPullSyncBlit {

exec
/usr/bin/zsh -lc 'rtk rg -n "BUILD_MISMATCH|PeerCapabilities|capabilit|advisory|version|negotiate|same build|same-build|old peer|old-peer|spec_version|supports_stream_resize" docs/TRANSFER_SESSION.md proto/blit.proto docs/plan/ONE_TRANSFER_PATH.md' in /home/michael/dev/Blit
 succeeded in 0ms:
84 matches in 3 files:

docs/TRANSFER_SESSION.md:7:(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
docs/TRANSFER_SESSION.md:23:`SessionHello{build_id, contract_version}`. Both ends compare for
docs/TRANSFER_SESSION.md:24:EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
docs/TRANSFER_SESSION.md:25:naming both ids, then stream close. No negotiate-down, no advisory
docs/TRANSFER_SESSION.md:26:fields, no feature-capability bits — same build implies same
docs/TRANSFER_SESSION.md:27:features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
docs/TRANSFER_SESSION.md:28:composed at compile time; `contract_version` is a belt-and-braces
docs/TRANSFER_SESSION.md:47:conservative defaults, never unlimited, and NEVER "old peer"
docs/TRANSFER_SESSION.md:48:(there are no old peers).
docs/TRANSFER_SESSION.md:136:Deliberately absent: `PeerCapabilities` (same build = same
docs/TRANSFER_SESSION.md:137:features), `spec_version` negotiation (the hello's exact match
docs/TRANSFER_SESSION.md:159:capability, never role/initiator/transport.
docs/TRANSFER_SESSION.md:164:`BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
docs/plan/ONE_TRANSFER_PATH.md:7:(D-2026-07-05-1). REV4's mixed-version-peers constraint is superseded
docs/plan/ONE_TRANSFER_PATH.md:8:outright by **D-2026-07-05-2 (no version compatibility, ever — same
docs/plan/ONE_TRANSFER_PATH.md:50:rule: "backward compatibility is NOT a consideration... same build
docs/plan/ONE_TRANSFER_PATH.md:52:client talks only to a blit-daemon from the same build; the session
docs/plan/ONE_TRANSFER_PATH.md:53:handshake REFUSES a mismatched peer outright. No negotiate-down, no
docs/plan/ONE_TRANSFER_PATH.md:54:advisory fields, no feature-capability bits for version skew.
docs/plan/ONE_TRANSFER_PATH.md:65:- New features. This is a consolidation; capability parity with
docs/plan/ONE_TRANSFER_PATH.md:96:messages, field numbers, capability negotiation, transport
docs/plan/ONE_TRANSFER_PATH.md:154:module, filters, mirror/resume flags, capabilities.
docs/plan/ONE_TRANSFER_PATH.md:157:pull's full-enumeration-then-negotiate slow start is deleted, which
docs/plan/ONE_TRANSFER_PATH.md:170:universal strategy; capability-gated alternatives slot in behind
docs/plan/ONE_TRANSFER_PATH.md:176:capability and payload type, never role or initiator.
docs/plan/ONE_TRANSFER_PATH.md:243:the **strict same-build handshake** (exact protocol/build identity
docs/plan/ONE_TRANSFER_PATH.md:250:No feature-capability bits: same build implies same features.
docs/plan/ONE_TRANSFER_PATH.md:251:The new proto text must carry NO version-tolerance semantics; the
docs/plan/ONE_TRANSFER_PATH.md:253:only, never "old peer" (today's proto comments frame some of that
docs/plan/ONE_TRANSFER_PATH.md:254:contract as old-peer fallback — those comment blocks describe live
proto/blit.proto:57:// (gate ordering, allowlist semantics, client_capabilities
proto/blit.proto:141:uint32 stream_count = 4; // number of parallel TCP streams negotiated for the...
proto/blit.proto:147:// senders leave all three unset. Mixed-version behavior: an old peer
proto/blit.proto:148:// skips unknown fields, a new peer treats "absent" as "old peer" —
proto/blit.proto:155:// PR3's negotiated min/max stream bounds are subsumed by
proto/blit.proto:171:// support (PushHeader.supports_stream_resize or
proto/blit.proto:172:// PeerCapabilities.supports_stream_resize), the daemon itself supports
proto/blit.proto:175:// sent in either direction (an old peer would decode them as an
proto/blit.proto:182:// unchanged, so old peers never see a suffixed handshake).
proto/blit.proto:199://              boundary as client_capabilities)
proto/blit.proto:214:// 0 = unknown → sender stays at today's negotiated stream_count.
proto/blit.proto:240:// DataTransferNegotiation.resize_enabled — old peers never receive one.
proto/blit.proto:342:// not carry PeerCapabilities), so the capability bit lives here;
proto/blit.proto:344:// Old clients leave it false and run at the fixed negotiated
proto/blit.proto:347:bool supports_stream_resize = 8;
proto/blit.proto:386:// Acknowledgment for PullSync with server capabilities
proto/blit.proto:463:PullSyncAck pull_sync_ack = 8;            // Acknowledgment with server capab...
proto/blit.proto:487:// unless resize_enabled was negotiated. Unused until ue-r2-2.
proto/blit.proto:586:// should reject specs with a version they don't understand.
proto/blit.proto:594://      bump. Receivers enforce spec_version by exact match, so a
proto/blit.proto:598://      intended mixed-version outcome — nothing safety-critical
proto/blit.proto:600:uint32 spec_version = 1;
proto/blit.proto:631:PeerCapabilities client_capabilities = 8;
proto/blit.proto:665:// OVERRIDE BOUNDARY (same rule as client_capabilities): in
proto/blit.proto:672:// today's static/conservative behavior. Added at spec_version 2
  +29 more in proto/blit.proto

exec
/usr/bin/zsh -lc 'rtk rg -n "NeedBatch|NeedComplete|ManifestComplete|manifest|backpressure|flow|BlockHashList|SourceDone|TransferSummary|session_token|epoch0_sub_token|in-stream|carrier|interleav|cancel|StallGuard|byte-account" docs/TRANSFER_SESSION.md proto/blit.proto docs/plan/ONE_TRANSFER_PATH.md' in /home/michael/dev/Blit
 succeeded in 0ms:
102 matches in 3 files:

docs/TRANSFER_SESSION.md:35:4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
docs/TRANSFER_SESSION.md:67:|==== manifest + need + payload phases run CONCURRENTLY =========|
docs/TRANSFER_SESSION.md:68:|  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
docs/TRANSFER_SESSION.md:69:|  DEST streams:    NeedBatch* ... NeedComplete                  |
docs/TRANSFER_SESSION.md:70:|  SOURCE streams:  payload (data plane sockets, or in-stream    |
docs/TRANSFER_SESSION.md:71:|                   frames when the in-stream carrier is chosen) |
docs/TRANSFER_SESSION.md:74:|  resume exception (RELIABLE): a NeedBatch entry flagged         |
docs/TRANSFER_SESSION.md:75:|  `resume=true` is followed by DEST's BlockHashList for that     |
docs/TRANSFER_SESSION.md:80:|  source manifest (filter-scoped, scan-complete-guarded) and     |
docs/TRANSFER_SESSION.md:83:|-- SourceDone (all payloads flushed) -->|   (phase: CLOSING)
docs/TRANSFER_SESSION.md:84:|<---------------- TransferSummary ------|   (DEST is the scorer)
docs/TRANSFER_SESSION.md:91:- `NeedComplete` is DESTINATION's promise that no further need
docs/TRANSFER_SESSION.md:93:- `TransferSummary` always travels DESTINATION → SOURCE (the end
docs/TRANSFER_SESSION.md:108:| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
docs/TRANSFER_SESSION.md:109:| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
docs/TRANSFER_SESSION.md:110:| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
docs/TRANSFER_SESSION.md:111:| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
docs/TRANSFER_SESSION.md:112:| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
docs/TRANSFER_SESSION.md:113:| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
docs/TRANSFER_SESSION.md:114:| 10 | `FileData file_data` | SOURCE | in-stream carrier |
docs/TRANSFER_SESSION.md:115:| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
docs/TRANSFER_SESSION.md:116:| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
docs/TRANSFER_SESSION.md:117:| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
docs/TRANSFER_SESSION.md:122:| 18 | `SourceDone source_done` | SOURCE | closing |
docs/TRANSFER_SESSION.md:123:| 19 | `TransferSummary summary` | DESTINATION | closing |
  +10 more in docs/TRANSFER_SESSION.md
docs/plan/ONE_TRANSFER_PATH.md:67:jobs, cancellation) is the bar. Zero-copy receive is **unparked**
docs/plan/ONE_TRANSFER_PATH.md:81:- REV4 invariants carry: byte-identical results, StallGuard,
docs/plan/ONE_TRANSFER_PATH.md:82:cancellation, byte-accounting. Existing pins are ported (not
docs/plan/ONE_TRANSFER_PATH.md:129:guard), filters, block-resume, gRPC fallback carrier, delegated
docs/plan/ONE_TRANSFER_PATH.md:130:transfer, progress events, jobs/cancel, read-only enforcement —
docs/plan/ONE_TRANSFER_PATH.md:155:2. SOURCE enumerates and **streams** its manifest immediately (no
docs/plan/ONE_TRANSFER_PATH.md:178:manifest it received (filter-scoped, scan-complete-guarded) and
docs/plan/ONE_TRANSFER_PATH.md:183:8. Summary/byte-accounting: one record shape.
docs/plan/ONE_TRANSFER_PATH.md:188:becomes a *byte-carrier option* inside the same session (control-
docs/plan/ONE_TRANSFER_PATH.md:200:cutover — either way the deletion proof asserts no bytes flow
docs/plan/ONE_TRANSFER_PATH.md:249:RELIABLE exception above), mirror phase, error/cancel semantics.
docs/plan/ONE_TRANSFER_PATH.md:277:8. **otp-8 fallback byte-carrier** (control-stream frames) as the
proto/blit.proto:6:// Push uses a bidirectional stream for an efficient "check-then-send" workflow.
proto/blit.proto:24:// Bidirectional pull with manifest comparison for selective transfers.
proto/blit.proto:25:// Client sends local manifest, server compares and sends only needed files.
proto/blit.proto:53:// streams progress/results back to the CLI. Bytes flow source→dst
proto/blit.proto:73:// supports cancellation (D-2026-07-04-3): the daemon fires the
proto/blit.proto:74:// row's cancellation token and the dispatcher races it, so an
proto/blit.proto:79://   OK                    → the cancellation token was fired;
proto/blit.proto:86://                           dispatch policy gates cancellation
proto/blit.proto:153:// epoch0_sub_token. That contract never shipped anywhere, so the
proto/blit.proto:183:bytes epoch0_sub_token = 13;
proto/blit.proto:276:FileHeader file_manifest = 2;
proto/blit.proto:277:ManifestComplete manifest_complete = 3;
proto/blit.proto:308:// entries absent from the source manifest. The scope is governed
proto/blit.proto:333:// client's ManifestComplete.scan_complete arrives as false —
proto/blit.proto:373:message ManifestComplete {
proto/blit.proto:388:bool server_checksums_enabled = 1;  // Whether daemon computed checksums for ...
proto/blit.proto:416:// PullSync Operation (bidirectional pull with manifest comparison).
proto/blit.proto:425:ManifestComplete manifest_done = 3;  // Signals end of client manifest
proto/blit.proto:426:BlockHashList block_hashes = 4;      // Client's block hashes for resume mode
proto/blit.proto:437:message BlockHashList {
proto/blit.proto:460:ManifestBatch manifest_batch = 5;         // Progress: server discovered X files
proto/blit.proto:468:// filtered source manifest and the unfiltered client manifest;
proto/blit.proto:679:// plane. This is how the remote→remote relay's manifest scan rides
proto/blit.proto:721:// Currently inline; may move to a streamed manifest if list sizes
proto/blit.proto:830:// field flows through unchanged.
  +30 more in proto/blit.proto

codex
The governing-decision check is clean on the newly added `Transfer` surface: I’m not seeing PeerCapabilities/spec-version negotiation in the new messages. The remaining “old peer” comments are on pre-cutover Push/PullSync-era messages, which the active plan explicitly allows to coexist until otp-10.
exec
/usr/bin/zsh -lc 'rtk git diff --unified=0 a3e2acb''^ a3e2acb -- docs/TRANSFER_SESSION.md proto/blit.proto | rtk rg "''^'"\\+.*(PeerCapabilities|spec_version|old peer|old-peer|capab|advisory|negotiate|version|push|pull|Push|Pull)\"" in /home/michael/dev/Blit
 succeeded in 0ms:
20:+RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
30:+   the owner's invariant: there is no push-shaped or pull-shaped
33:+   `SessionHello{build_id, contract_version}`. Both ends compare for
35:+   naming both ids, then stream close. No negotiate-down, no advisory
36:+   fields, no feature-capability bits — same build implies same
37:+   features. `build_id` = `<crate version>+<git commit hash>[.dirty]`
38:+   composed at compile time; `contract_version` is a belt-and-braces
57:+   conservative defaults, never unlimited, and NEVER "old peer"
58:+   (there are no old peers).
146:+Deliberately absent: `PeerCapabilities` (same build = same
147:+features), `spec_version` negotiation (the hello's exact match
149:+push/pull-specific message.
169:+  capability, never role/initiator/transport.
189:+At cutover (otp-10): `Push`, `PullSync`, and their message
191:+per-direction drivers die with them; `DelegatedPull` shrinks to
203:+  // that replaces Push and PullSync at cutover. Contract:
213:+// handler returns UNIMPLEMENTED, pinned by test). Replaces Push and
214:+// PullSync whole at cutover (otp-10, D-2026-07-05-1); no bridge
229:+// negotiate-down, no advisory fields, no capability bits.
231:+  // "<crate version>+<git commit>[.dirty]", composed at compile time.
234:+  uint32 contract_version = 2;
260:+  // unknown hardware value, conservative, never "old peer").
306:+// every direction; replaces PushSummary/PullSummary at cutover).

exec
/usr/bin/zsh -lc "rtk nl -ba docs/plan/ONE_TRANSFER_PATH.md | rtk sed -n '70,110p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    70	  on the write-strategy seam.
    71	
    72	## Constraints
    73	
    74	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    75	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    76	  violated at the choreography layer.
    77	- **Converge up, not down**: per benchmark cell, the unified session
    78	  must match the better of today's two directions (within ±10% run
    79	  noise), not their average. Unification that slows the fast
    80	  direction fails review.
    81	- REV4 invariants carry: byte-identical results, StallGuard,
    82	  cancellation, byte-accounting. Existing pins are ported (not
    83	  dropped) as tests become role-parameterized; test count never
    84	  drops.
    85	- The sf-2 shape-correction behavior (stream count corrects as the
    86	  need list accumulates) becomes the one and only stream policy —
    87	  both directions inherit it by construction; its pins carry over.
    88	- **The bounded-unilateral dial contract carries unchanged**
    89	  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
    90	  dial, bounded by the byte RECEIVER's advertised capacity profile
    91	  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
    92	  unlimited). The session's role model must express this — profile
    93	  travels DESTINATION→SOURCE at setup regardless of who initiated —
    94	  and otp-1's contract names it explicitly.
    95	- Wire contract discipline (REV4 rule): the unified session's proto —
    96	  messages, field numbers, capability negotiation, transport
    97	  selection — is a reviewed doc+proto slice **before** any behavior
    98	  depends on it.
    99	- Every slice through the codex loop (D-2026-07-04-1); tree green
   100	  after every slice; transitional coexistence of old+new paths is
   101	  scaffolding only — the plan is not Shipped until the deletion slice
   102	  lands and the deletion proof is recorded.
   103	- Windows parity: suite green on the owner's machine + windows-latest
   104	  CI before Shipped.
   105	
   106	## Acceptance criteria
   107	
   108	- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
   109	      on a symmetric rig (same filesystem class both ends, cold
   110	      caches, disk-to-disk), for each data direction and workload

exec
/usr/bin/zsh -lc "rtk nl -ba docs/plan/ONE_TRANSFER_PATH.md | rtk sed -n '238,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   238	loop. Tree green after every slice; old paths keep working until
   239	otp-9 deletes them.
   240	
   241	1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   242	   `Transfer` RPC and message set — roles, phases, field numbers,
   243	   the **strict same-build handshake** (exact protocol/build identity
   244	   exchanged at session open; any mismatch is refused with a clear
   245	   error — D-2026-07-05-2; pinned by test when the session lands),
   246	   the receiver capacity profile + bounded-unilateral dial contract
   247	   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
   248	   that exists), transport selection, resume phase ordering (the
   249	   RELIABLE exception above), mirror phase, error/cancel semantics.
   250	   No feature-capability bits: same build implies same features.
   251	   The new proto text must carry NO version-tolerance semantics; the
   252	   capacity profile's absent/0 fields mean "unknown hardware value"
   253	   only, never "old peer" (today's proto comments frame some of that
   254	   contract as old-peer fallback — those comment blocks describe live
   255	   pre-cutover code and die with their messages at otp-10, per the
   256	   D-2026-07-05-2 review adjudication). Codex-reviewed before any
   257	   code consumes it.
   258	2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   259	   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   260	   cells, cold caches, tmpfs rows re-labeled wire-reference only —

exec
/usr/bin/zsh -lc 'rtk git diff --unified=0 a3e2acb''^ a3e2acb -- | rtk rg "''^[-+]" | rtk rg "#'"\\[(tokio::)?test|#\\[ignore|mod tests|fn [a-zA-Z0-9_]+\\(\"" in /home/michael/dev/Blit
 exited 1 in 0ms:
codex
**Findings**

- **High** — [docs/TRANSFER_SESSION.md:57](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:57) §Phase state machine: the diagram fixes `SourceDone` as initiator→responder and `TransferSummary` as responder→initiator, but the frame table says those are SOURCE→DESTINATION and DESTINATION→SOURCE. For initiator=DESTINATION sessions, the diagram reverses the closing flow, so the authoritative state machine contradicts the role rule.

- **High** — [docs/TRANSFER_SESSION.md:143](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:143), [proto/blit.proto:1283](/home/michael/dev/Blit/proto/blit.proto:1283): `DataPlaneGrant.initial_streams` is always emitted by the responder. When the responder is DESTINATION, that lets the receiver choose the initial dial unless the contract says this is only a pre-armed accept ceiling and SOURCE may dial fewer. This blurs the D-2026-06-20-1/-2 sender-owned dial rule.

- **Medium** — [docs/TRANSFER_SESSION.md:143](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:143), [proto/blit.proto:1283](/home/michael/dev/Blit/proto/blit.proto:1283): data-plane socket auth is not specified enough. The contract names both `session_token` and `epoch0_sub_token`, but not the exact initial-socket handshake, token ordering, replay scope, or whether epoch-0 streams must present both credentials.

- **Medium** — [docs/TRANSFER_SESSION.md:150](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:150), [proto/blit.proto:1358](/home/michael/dev/Blit/proto/blit.proto:1358): in-stream carrier framing lacks a record grammar. `FileData` has no path and there is no `FileComplete`, so the contract must state whether file payloads are strictly serialized after `file_begin`, how completion is inferred, and whether control/manifest/need frames may interleave inside a file/tar/block record.

- **Medium** — [docs/TRANSFER_SESSION.md:67](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:67), [docs/TRANSFER_SESSION.md:91](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:91), [proto/blit.proto:1302](/home/michael/dev/Blit/proto/blit.proto:1302): large-manifest and need-batch backpressure are left to invention. If the intended answer is gRPC stream backpressure plus bounded internal queues, name it; also specify `NeedComplete` must follow `ManifestComplete` plus full diff processing, especially for mirror safety.

- **Low** — [docs/TRANSFER_SESSION.md:163](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:163), [proto/blit.proto:1322](/home/michael/dev/Blit/proto/blit.proto:1322): docs say `SessionError{code, message, detail}`, but proto has no `detail` field. Either remove `detail` from the contract text or add the field before implementations depend on structured errors.

Static checks on the stub/test part look honest: all six `Blit` impls have the new `transfer` method, the diff adds exactly one `#[tokio::test]`, and I saw no removed test markers. I did not rerun the cargo suite in this read-only sandbox.

VERDICT: NEEDS FIXES before otp-3/otp-4 consume the contract.
tokens used
175,754
**Findings**

- **High** — [docs/TRANSFER_SESSION.md:57](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:57) §Phase state machine: the diagram fixes `SourceDone` as initiator→responder and `TransferSummary` as responder→initiator, but the frame table says those are SOURCE→DESTINATION and DESTINATION→SOURCE. For initiator=DESTINATION sessions, the diagram reverses the closing flow, so the authoritative state machine contradicts the role rule.

- **High** — [docs/TRANSFER_SESSION.md:143](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:143), [proto/blit.proto:1283](/home/michael/dev/Blit/proto/blit.proto:1283): `DataPlaneGrant.initial_streams` is always emitted by the responder. When the responder is DESTINATION, that lets the receiver choose the initial dial unless the contract says this is only a pre-armed accept ceiling and SOURCE may dial fewer. This blurs the D-2026-06-20-1/-2 sender-owned dial rule.

- **Medium** — [docs/TRANSFER_SESSION.md:143](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:143), [proto/blit.proto:1283](/home/michael/dev/Blit/proto/blit.proto:1283): data-plane socket auth is not specified enough. The contract names both `session_token` and `epoch0_sub_token`, but not the exact initial-socket handshake, token ordering, replay scope, or whether epoch-0 streams must present both credentials.

- **Medium** — [docs/TRANSFER_SESSION.md:150](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:150), [proto/blit.proto:1358](/home/michael/dev/Blit/proto/blit.proto:1358): in-stream carrier framing lacks a record grammar. `FileData` has no path and there is no `FileComplete`, so the contract must state whether file payloads are strictly serialized after `file_begin`, how completion is inferred, and whether control/manifest/need frames may interleave inside a file/tar/block record.

- **Medium** — [docs/TRANSFER_SESSION.md:67](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:67), [docs/TRANSFER_SESSION.md:91](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:91), [proto/blit.proto:1302](/home/michael/dev/Blit/proto/blit.proto:1302): large-manifest and need-batch backpressure are left to invention. If the intended answer is gRPC stream backpressure plus bounded internal queues, name it; also specify `NeedComplete` must follow `ManifestComplete` plus full diff processing, especially for mirror safety.

- **Low** — [docs/TRANSFER_SESSION.md:163](/home/michael/dev/Blit/docs/TRANSFER_SESSION.md:163), [proto/blit.proto:1322](/home/michael/dev/Blit/proto/blit.proto:1322): docs say `SessionError{code, message, detail}`, but proto has no `detail` field. Either remove `detail` from the contract text or add the field before implementations depend on structured errors.

Static checks on the stub/test part look honest: all six `Blit` impls have the new `transfer` method, the diff adds exactly one `#[tokio::test]`, and I saw no removed test markers. I did not rerun the cargo suite in this read-only sandbox.

VERDICT: NEEDS FIXES before otp-3/otp-4 consume the contract.
