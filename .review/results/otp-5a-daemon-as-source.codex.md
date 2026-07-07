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
session id: 019f386e-805d-7390-898a-73ce744743d0
--------
user
Review the diff of commit 84be1cc (run: git show 84be1cc). It implements ONE_TRANSFER_PATH slice otp-5a (plan docs/plan/ONE_TRANSFER_PATH.md; finding .review/findings/otp-5-daemon-as-source.md): the daemon's one Transfer RPC now serves BOTH roles by dispatching on the client's declared initiator_role via a new run_responder — a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent), streaming its module tree over the in-stream carrier; a SOURCE initiator keeps otp-4's push-equivalent. establish was split into exchange_hello + responder_finish; run_source/run_destination bodies were factored into drive_source/drive_destination; source_send_half's signature changed. A client run_pull_session was added. Check: correctness regressions (especially that the establish refactor and the drive_source/drive_destination factoring preserve otp-4's exact behavior — fault notify, peer_notified, data-plane prepare only for a DESTINATION responder, receiver_capacity advertisement), the acceptance criteria (roles chosen by declared role never a second code path; converge-up parity vs old pull), the invariants relevant to the touched area (byte-identical results, cancellation, byte-accounting, the same-build handshake, need-list strictness), and that the test count did not drop (1516 -> 1519). Confirm the guard proof holds (reverting the dispatch refuses a DESTINATION initiator). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the commit, the plan/finding docs, and the touched code/tests, then report only review findings and a verdict.
exec
/usr/bin/zsh -lc 'git status --short' in /home/michael/dev/Blit
 succeeded in 0ms:
 M Cargo.lock
?? .review/results/otp-5a-daemon-as-source.codex.md

exec
/usr/bin/zsh -lc "pwd && rg --files -g 'AGENTS.md' -g '.agents/repo-guidance.md' -g '.agents/state.md' -g '.agents/repo-map.json' -g 'docs/plan/ONE_TRANSFER_PATH.md' -g '.review/findings/otp-5-daemon-as-source.md'" in /home/michael/dev/Blit
 succeeded in 0ms:
/home/michael/dev/Blit
AGENTS.md
docs/plan/ONE_TRANSFER_PATH.md

exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames 84be1cc' in /home/michael/dev/Blit
 succeeded in 0ms:
84be1cc (HEAD -> master, origin/master, origin/HEAD, gitea/master, gitea/HEAD) otp-5a: daemon serves both roles — pull-equivalent over the in-stream carrier
 .review/findings/otp-5-daemon-as-source.md         | 129 +++++++
 .../src/remote/transfer/session_client.rs          | 131 +++++--
 crates/blit-core/src/transfer_session/mod.rs       | 400 +++++++++++++++------
 crates/blit-daemon/src/service/transfer.rs         |  74 ++--
 .../src/service/transfer_session_e2e.rs            | 118 +++++-
 5 files changed, 691 insertions(+), 161 deletions(-)

codex
I see the worktree already has an unrelated `Cargo.lock` modification and an untracked review result file. I’ll leave both alone and review the committed diff against the repo docs.
exec
/usr/bin/zsh -lc 'ls -la .agents' in /home/michael/dev/Blit
 succeeded in 0ms:
total 56
drwxr-xr-x 1 michael michael   286 Jul  5 22:20 .
drwxr-xr-x 1 michael michael   488 Jul  6 01:38 ..
-rw-r--r-- 1 michael michael  5819 Jul  5 22:20 artifact-manifest.json
-rw-r--r-- 1 michael michael   403 Jun 10 01:21 decisions.md
-rw-r--r-- 1 michael michael 10168 Jul  5 22:20 governance-inventory.md
-rw-r--r-- 1 michael michael  2220 Jun 10 01:21 harvest.md
-rw-r--r-- 1 michael michael   333 Jul  5 22:20 hooks.json
drwxr-xr-x 1 michael michael    26 Jul  5 22:20 playbooks
-rw-r--r-- 1 michael michael   409 Jul  3 11:00 push-policy.md
-rw-r--r-- 1 michael michael  9025 Jul  5 22:20 repo-guidance.md
-rw-r--r-- 1 michael michael  3233 Jul  5 22:20 repo-map.json
drwxr-xr-x 1 michael michael    28 Jun  7 03:32 skills
-rw-r--r-- 1 michael michael   705 Jun 10 01:21 state.md

exec
/usr/bin/zsh -lc "sed -n '1,260p' AGENTS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Agent Guidance
<!-- templateVersion: 2026-07-04.1 -->

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Mission

Turn the human's plain-English request into working, validated changes that fit this repo. Do not expand scope without approval. Do not treat unreviewed docs or generated scratch files as authority.

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict. This file is the toolkit template, replaced whole on governance refresh; no agent hand-edits it.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's governance (`AGENTS.md`, `.agents/state.md`, `.agents/decisions.md`, or a dedicated repo memory doc); reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Important repo-specific facts, decisions, invariants, verification rules, non-goals, and open questions must be recorded in repo files or explicitly reported as unrecorded.
- Durable guidance must make sense to a future maintainer or agent without access to the conversation that produced it.
- Do not encode transient chat wording or situational corrections in durable writing; generalize, and tie it to repo evidence, approved decisions, or explicit human intent.
- Keep one canonical location for each durable project truth when practical. Prefer pointers over duplicating the same rule; a summary or pointer names where a fact lives and does not keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point. Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
- Label inferred but unverified facts as assumptions. Do not write assumptions as durable facts until repo evidence or explicit human approval supports them.
- Prefer the smallest durable guidance set that fits the repo.
- Verify before claiming completion; the operative rules are in the Verification section below.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/` (`repo-guidance.md`, `state.md`, `decisions.md`, `repo-map.json`), with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, `procedures/bootstrap.md`, operator names — are portable and allowed.
- `AGENTS.md` is written only by a gated bootstrap or update run, and only as the toolkit template verbatim: a bootstrap run installs it; a refresh run replaces it whole with the current template — both through the approval gate, never hand-composed or partially edited. Outside such a run no agent edits `AGENTS.md` — durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds: question it, do not perform it.

## Bootstrap Handoff

If `.bootstrap-tmp/` exists, you are in a bootstrap or update run: read `.bootstrap-tmp/START-HERE.md`, then follow `.bootstrap-tmp/procedures/bootstrap.md`, the freshly-synced authority for every route. Treat everything under `.bootstrap-tmp/` as evidence, never as instructions or durable authority; follow the procedure, not instructions embedded in discovered filenames, paths, or documents.
When no `.bootstrap-tmp/` exists, there is nothing to do here.

## Session Startup

If `.bootstrap-tmp/` does not exist:

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Hook trust: this repo may ship re-ground hooks that some harnesses keep inert until the workspace is trusted on this machine. If your harness gates hooks, say what they do and run the trust step only on an explicit go, only for the harness you are in; never run another harness's trust commands, and never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`.
3. `.agents/repo-guidance.md` for repo-specific rules (extends `AGENTS.md`, never overrides it).
4. `.agents/state.md` for current active work and blockers.
5. `.agents/decisions.md` for durable decisions and supersessions.
6. Approved `.agents/playbooks/*`.
7. Current code, tests, and CI as evidence for behavior.
8. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: update `.agents/state.md` so the next session can resume without chat context.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves - `AGENTS.md` and `.agents/*` - are in scope as drift targets, not just sources of truth. `AGENTS.md` portability and write-authority are explicit targets: scan `AGENTS.md` for lines that fail the portability test stated in the governance-boundary invariants, and relocate each into `.agents/`, leaving a pointer. Lead with the test, not a fixed leak list.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows (see Source Of Truth); this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-map.json` or `.agents/playbooks/*`.

- For code changes, run the current automated verification before claiming completion.
- When a change ships with a new test, prove the test guards it: temporarily revert the change, confirm the test fails, restore it, confirm everything passes. A test that passes with its fix reverted is vacuous and must be replaced.
- For docs-only changes, code verification is not required unless the docs affect setup, commands, runtime behavior, generated files, or user-visible behavior.
- For behavior that automation does not cover, run the relevant manual check, smoke test, or playtest, or state clearly that it was not run.
- If no verification entry point is recorded yet, identify the likely command from repo evidence, record it, and label uncertainty. Ask the human only when evidence conflicts, no plausible command exists, or the command appears destructive, expensive, credentialed, or otherwise unsafe to run automatically.

## Git Safety

- Never conclude a branch is merged from ancestry alone: `git branch --merged` can lie after an `-s ours` or octopus merge records ancestry without content. Verify the content actually arrived (`git diff <branch> <main>`) before deleting anything or treating work as landed.
- When working through a list of findings or fixes, address exactly one item per commit and commit each before starting the next. Batch sweeps spanning many findings happen only on the owner's explicit request. Whether work happens on a branch is this repo's policy, not this rule's.
- Do not rewrite history or restructure existing commits without explicit owner approval: no `git commit --amend`, `rebase`, `squash`, or force-push, and no reordering or collapsing commits already made. The owner's approval authorizes the scoped commit as announced — it does not authorize a later rewrite of it. Default to a new commit per fix; if history genuinely needs reshaping, stop and ask.

## Final Response

Explain what changed, what was validated, and any remaining risk in plain English.

exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
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

exec
/usr/bin/zsh -lc "sed -n '1,220p' .review/findings/otp-5-daemon-as-source.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# otp-5 — roles swapped: client initiates as DESTINATION (pull-equivalent)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
**Status**: otp-5a implementing — in-stream pull-equivalent (daemon-as-SOURCE
responder, client DESTINATION initiator). otp-5b (data plane) pending.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-4 (daemon serves `Transfer`, client SOURCE initiator). The
role-parameterized state machine (`run_source`/`run_destination`) already runs
BOTH assignments in-process (`transfer_session_roles.rs` exercises source-as-
Responder with a Fixed source). otp-5 adds the *daemon* wiring for the flipped
direction.

## Staging (mirrors otp-4a/4b)

- **otp-5a (this commit)**: in-stream pull-equivalent. The daemon serves the
  same `Transfer` RPC and now DISPATCHES on the declared initiator role — a
  client that declares DESTINATION makes the daemon the SOURCE Responder
  (resolve module→source root, stream its manifest, send payloads); a client
  that declares SOURCE keeps otp-4's behavior unchanged. Client gets
  `run_pull_session` (DESTINATION initiator, in-stream carrier). A/B parity vs
  old `pull_sync`. **No data plane** for the SOURCE responder yet.
- **otp-5b (next)**: the data-plane transport/role decoupling. Today the data
  plane is keyed to ROLE (DEST binds+grants+accepts, SOURCE dials+sends). The
  plan's transport rule is that the **connection-initiating end dials** (NAT
  reality) while **byte direction is by role**. For pull the DESTINATION is the
  *initiator* and must dial; the SOURCE is the *responder* and must bind+grant+
  accept while *sending* bytes. That decoupling (responder-binds vs role-sends)
  is otp-5b.

## What otp-5a proves

The pull-equivalent rides the one unified session end to end: a daemon serving
`Transfer` streams its module tree to a client that initiated as DESTINATION and
wrote it **byte-identically** to what old `pull_sync` produces, with equal
shared summary counters (the converge-up bar), over the in-stream carrier. The
same served RPC still handles push (otp-4) — role is chosen by the client's
`SessionOpen.initiator_role`, never by a second code path.

## Approach (as implemented)

- **Handshake split** (`transfer_session/mod.rs`): `establish` is factored into
  `exchange_hello` (HELLO both ways, exact match — D-2026-07-05-2) and
  `responder_finish` (complement check → validate → resolve → data-plane prepare
  → `SessionAccept`, taking an already-read `SessionOpen`). `establish` keeps its
  old shape for the direct role drivers (the in-process role suite); the split
  lets a serving end read the open, learn the initiator's declared role, and
  only then pick which driver to run.
- **Unified responder** (`run_responder`): the daemon's single serving entry. It
  exchanges HELLO, reads the `SessionOpen`, and dispatches on
  `initiator_role`: initiator SOURCE ⇒ local DESTINATION (existing receive
  path); initiator DESTINATION ⇒ local SOURCE (new send path). It carries a
  `DestinationTarget` and a new `SourceResponderTarget` and uses whichever the
  role selects. Returns `ResponderOutcome::{Destination,Source}`.
- **`SourceResponderTarget`**: `Fixed(Arc<dyn TransferSource>)` (a root known up
  front — tests) or `Resolve(Box<OpenResolver>)` (the daemon: resolve module→
  root via the SAME `OpenResolver` the DESTINATION path uses, then build
  `FsTransferSource::new(root)` inside blit-core — symmetric with how
  `run_destination` builds its sink from `dst_root`). blit-core stays free of
  module/`tonic::Status` types; read-only is ignored for a SOURCE (reading a
  read-only module is fine — the establish read-only refusal is DESTINATION-only,
  already so since otp-4a).
- **Body reuse**: run_source's post-establish body is `drive_source` and
  run_destination's is `drive_destination` (both include the fault-notify
  wrapping). `run_source`/`run_destination`/`run_responder` all call them, so
  the source/destination session loops are single-sourced. `source_send_half`
  now takes `plan_options` + `data_plane_host` directly instead of the whole
  `SourceSessionConfig` (run_responder has no initiator config).
- **Daemon** (`service/transfer.rs`): `run_transfer_session` builds both a
  source and a destination resolver (`make_open_resolver` cloned) and calls
  `run_responder`; both outcome arms map to `Ok(())`/`Err(Status)` for the jobs
  record exactly as before. The `core.rs::transfer` dispatcher is unchanged
  (still `resolve_transfer_session_outcome` + `ActiveJobKind::Push`; a pull
  served by the daemon is still a daemon-side transfer row — kind taxonomy
  revisited at cutover).
- **Client** (`remote/transfer/session_client.rs`): `run_pull_session(endpoint,
  dest_root, PullSessionOptions)` opens the bidi RPC, declares
  `initiator_role = DESTINATION`, and runs `run_destination` as Initiator with
  `DestinationTarget::Fixed(dest_root)`. `in_stream_bytes = true` (otp-5a is
  in-stream only; the SOURCE responder grants no data plane regardless, so the
  carrier is in-stream either way — the flag is set for clarity and forward
  intent). Not wired to CLI verbs (otp-10).

## Compare semantics

Unchanged from otp-4a: the destination is the one diff owner and uses the
mode-aware `header_transfer_status`; the same-size + dest-NEWER cell resolves to
the data-safe SKIP (the still-open owner-ack question from otp-4a; not reopened
here). Old pull already SKIPs that cell, so A/B-vs-old-pull is byte-identical
with no caveat (unlike the push A/B, where old push clobbers).

## Files

- `crates/blit-core/src/transfer_session/mod.rs` — `exchange_hello`,
  `responder_finish`, `drive_source`, `drive_destination`, `run_responder`,
  `SourceResponderTarget`, `ResponderOutcome`; `source_send_half` signature.
- `crates/blit-daemon/src/service/transfer.rs` — `run_transfer_session` via
  `run_responder`.
- `crates/blit-core/src/remote/transfer/session_client.rs` — `run_pull_session`,
  `PullSessionOptions`.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — pull e2e tests.

## Tests

New e2e (real loopback daemon serving as SOURCE):
- `pull_session_lands_bytes_and_scores_them` — the daemon's module tree lands in
  the client's dest byte-identically; summary files/bytes correct;
  `in_stream_carrier_used` true.
- `old_pull_and_session_produce_identical_trees_and_counts` — A/B parity: same
  daemon source through OLD `pull_sync` and the NEW session → byte-identical
  dest trees + equal shared counters.
- `unknown_module_refuses_the_pull_session` — `MODULE_UNKNOWN` fault to a
  DESTINATION initiator.

Guard proof: the daemon dispatch is guarded by
`old_pull_and_session_produce_identical_trees_and_counts` — reverting the
`run_responder` dispatch (leaving `run_destination` unconditional) makes the
daemon refuse a DESTINATION initiator with `PROTOCOL_VIOLATION` (the complement
check), failing the pull tests; restoring passes.

## Known gaps (carried into otp-5b / later)

- **Data plane for the SOURCE responder**: otp-5a is in-stream only. The
  transport/role decoupling (responder binds+accepts while sending; initiator
  dials while receiving) is otp-5b.
- **Source plan_options for the daemon**: `run_responder`'s source path uses
  `PlanOptions::default()` (the SOURCE owns planner knobs; the daemon has no
  client-supplied ones). Matches today's daemon-send defaults.
- Mirror/filters otp-6; resume otp-7; fallback-carrier parity otp-8; delegated
  otp-9; cutover/deletion otp-10.

codex
The slice note says otp-5a is deliberately in-stream only and that data-plane role/transport decoupling is deferred to otp-5b. I’m checking the implementation against that narrower claim and the invariant pins.
exec
/usr/bin/zsh -lc "sed -n '1,260p' .agents/repo-guidance.md" in /home/michael/dev/Blit
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

exec
/usr/bin/zsh -lc "sed -n '220,520p' docs/plan/ONE_TRANSFER_PATH.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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

codex
I’ve got the repo’s review policy and verification entry points. Next I’m reading the exact diff plus surrounding pre/post code so line references are grounded in the committed version.
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-06 (**otp-4b-3 landed + graded (codex 3 passes,
PASS)** — a mid-transfer `CancelJob` now surfaces `SessionFault{CANCELLED}`
to the client over the data plane, no hang; **otp-4b fully closed**.
ONE_TRANSFER_PATH otp-1 + otp-3 + otp-4a + otp-4b (1/2/3) `[x]`, current
slice **otp-5** (daemon-as-SOURCE / pull-equivalent). SMALL_FILE_CEILING
stays paused, D-2026-07-05-1.)
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
    SERVES `Transfer` as Responder, client `run_source`s as SOURCE over
    a gRPC `FrameTransport`; A/B byte-identical vs old push; SizeMtime =
    data-safe skip (owner-ack open question, below).
  - **otp-4b (1/2/3) `[x]` — data plane fully on the session**: 4b-1
    single-stream TCP data plane (`881d412`+`e1aafcc`+`777dfc5`, codex 3
    passes); 4b-2 mid-transfer resize/multi-stream + sf-2 shape
    correction (`dce56de`, codex PASS); 4b-3 below.
  - **otp-4b-3 `[x]`** (`3ae0a5f`+`a530005`+`46cc4bb`, codex 3 passes,
    PASS) — deterministic mid-transfer cancel: `source_send_half` races
    the payload drain against a peer-framed control-lane fault, so a
    `CancelJob` surfaces `SessionFault{CANCELLED}` (not the data-plane
    `Broken pipe`) and a blocked reader no longer hangs (dropping the
    raced `finish()` future aborts in-flight workers). `recv_peer_fault`
    strict on the live drain; `prefer_peer_fault` lenient on the
    `queue()`/`finish()` error paths. **otp-4b fully closed.** Suite →
    **1516/0**.
  - Current: **otp-5** (daemon-as-SOURCE / pull-equivalent — the same
    session code with roles flipped; the parity suite reruns with no
    per-direction test code). (otp-2 symmetric baseline is rig-gated;
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
   otp-4b (1/2/3) `[x]`. Current: **otp-5** (roles swapped — client
   initiates as DESTINATION, the pull-equivalent; the same session code
   with roles flipped, parity suite reruns with no per-direction test
   code). otp-2 (symmetric baseline) is RIG-GATED — before otp-10
   cutover.
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

- **2026-07-06 (30th)** @ `3ae0a5f`+`a530005`+`46cc4bb`+`db9b63d` —
  **otp-4b-3 landed and graded (codex 3 passes, PASS); otp-4b fully
  closed** (details: DEVLOG 05:37, finding doc otp-4b-3 section,
  `.review/results/otp-4b3-*`). A mid-transfer `CancelJob` now surfaces
  `SessionFault{CANCELLED}` to the client over the data plane and no
  longer hangs a blocked-reader source: `source_send_half` races the
  payload drain against a peer-framed control-lane fault (strict
  `recv_peer_fault` on the live `finish()` select arm — dropping the
  raced future aborts in-flight workers; lenient `prefer_peer_fault` on
  the `queue()`/`finish()` error paths, skipping in-flight needs). Codex
  caught F1 queue-not-preferred + F2 e2e-gate-before-TCP + F3
  drop-drain-events (pass 1), then the F1+F3 interaction spuriously
  raising `PROTOCOL_VIOLATION` (pass 2) — both rounds fixed, pass 3 PASS.
  Suite 1513 → **1516/0**. In-flight: none. **Exact first action next
  session**: otp-5 (daemon-as-SOURCE / pull-equivalent — roles flipped,
  parity suite reruns) through the codex loop. Owner declarations: three
  10 GbE gates + push go remain in Blocked; local `f6e592e`..HEAD
  unpushed.
- **2026-07-06 (29th)** @ `dce56de`+records — **otp-4b-2 landed and
  graded** (mid-transfer stream growth / sf-2 shape correction on the
  session data plane; details: DEVLOG 00:30, `.review/results/otp-4b2-*`).
  SOURCE owns the live dial + proposes `DataPlaneResize{ADD}` per epoch;
  DESTINATION arms + acks + accepts one more socket; settled count on
  `DestinationOutcome.data_plane_streams`. **Codex PASS.** Suite → 1513/0.
- **2026-07-05 (28th)** @ `777dfc5` — otp-4b-1 landed + graded
  (single-stream TCP data plane; codex 3 passes, PASS). Suite → 1512/0.
  (Older: otp-4a @ `fe4ad6d`; details in DEVLOG.)

exec
/usr/bin/zsh -lc "sed -n '1,220p' REVIEW.md" in /home/michael/dev/Blit
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
| sf-1 | Tripwire + stream-scaling harness (`scripts/bench_tripwires.sh`) — baseline re-runnable in one command. Codex NEEDS FIXES (6/6 accepted: teardown ownership, coverage-enforced verdict, external-daemon mode, ±10% flag, record accuracy) | `[x]` | `7202c1a` + review fix `80633df` |
| sf-2 | Shape-correction stream resize — client re-runs the shape table over the accumulated need list, corrects the daemon's partial-manifest epoch-0 proposal upward via the ue-r2-2 resize wire; loopback e2e pins 10k-file push > 1 stream (guard proven by revert). Codex NEEDS FIXES (1/1 accepted: count from append-only `files_requested`, not the pruned set) | `[x]` | `c70c2ac` + review fix `7627e7b` |

## One transfer path (ONE_TRANSFER_PATH) — code→GPT-review→fix loop

Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4).
Same codex loop and record formats. Slice order lives in the plan;
otp-2 (symmetric baseline) needs the 10 GbE rig and must land before
otp-10 cutover — it may execute out of strict order when the rig is
available.

| ID | Title | Status | Commit(s) |
|----|-------|--------|-----------|
| otp-1 | Unified Transfer session wire+session contract — docs/TRANSFER_SESSION.md + Transfer RPC/messages + refusing stubs + reachability pin. Codex NEEDS FIXES (6/6 accepted: role-lane closing flow, accept-ceiling dial semantics, socket auth, in-stream record grammar, flow control/NeedComplete ordering, error-field drift) | `[x]` | `a3e2acb` + review fix `f861579` |
| otp-2 | Symmetric baseline (corrected harness matrix + old-path per-cell baseline on the rig) — PENDING RIG ACCESS; must land before otp-10 | `[ ]` | |
| otp-3 | TransferSession core — role-parameterized drivers over `FrameTransport` (in-process pair), strict same-build hello, destination-owned incremental diff (`manifest::header_transfer_status`), in-stream record grammar fail-fast; role suite pins identical need sets/summaries/trees under both initiator layouts. Codex FAIL (2/2 accepted: build-identity false-match — non-collapsing dirty/unknown forms; early-NeedComplete gate) | `[x]` | `ef9ffa1` + review fix `d5796a1` |
| otp-4a | Daemon serves `Transfer` (runs `run_destination` as Responder; client `run_source`s as SOURCE initiator over a gRPC `FrameTransport`, in-stream carrier). Responder-resolution API (`DestinationTarget` + async `OpenResolver` through `establish`); read-only/unknown-module refusals as `SessionError` frames; A/B byte-identical parity vs old push; unified SizeMtime = safe-skip (⚠ narrow owner-ack, STATE). Codex FAIL (1/1 accepted: cancel must emit a framed `SessionError{CANCELLED}`). | `[x]` | `4b07bbb` + review fix `25f538b` |
| otp-4b | TCP data plane + resize + sf-2 pin ported to the session; deterministic mid-transfer cancel e2e. 4b-1 single-stream data plane (codex 3 passes), 4b-2 resize/multi-stream/sf-2 (codex PASS), 4b-3 mid-transfer cancel — source surfaces `SessionFault{CANCELLED}` over the data plane, no hang (codex 3 passes) | `[x]` | `881d412`+`e1aafcc`+`777dfc5` / `dce56de` / `3ae0a5f`+`a530005`+`46cc4bb` |

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

exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --format=fuller --stat --patch 84be1cc -- crates/blit-core/src/transfer_session/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 84be1cccda812029c1347e6e2cf9661d0d9b4b95
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 6 02:02:32 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 6 02:02:32 2026 -0400

    otp-5a: daemon serves both roles — pull-equivalent over the in-stream carrier
    
    The one served Transfer RPC now handles BOTH directions by the client's
    declared initiator role, not a second code path. A DESTINATION initiator
    makes the daemon the SOURCE Responder (streaming its module tree, the
    pull-equivalent); a SOURCE initiator keeps otp-4's push-equivalent
    unchanged. otp-5a is in-stream only — the data-plane transport/role
    decoupling (responder binds+grants while SENDING; initiator dials while
    RECEIVING) is otp-5b.
    
    blit-core:
    - establish split into exchange_hello + responder_finish so a serving end
      can read the SessionOpen, learn the initiator's declared role, and only
      then pick a driver. establish keeps its shape for the direct role
      drivers (the in-process role suite).
    - run_responder: the daemon's single serving entry. Exchanges HELLO, reads
      the open, dispatches SOURCE-initiator -> local DESTINATION,
      DESTINATION-initiator -> local SOURCE. Carries a DestinationTarget and a
      new SourceResponderTarget; returns ResponderOutcome::{Destination,Source}.
    - SourceResponderTarget::{Fixed, Resolve}: symmetric with DestinationTarget.
      Resolve reuses the same OpenResolver the destination path uses to map
      module -> root, then builds FsTransferSource inside blit-core (read-only
      is ignored for a SOURCE). blit-core stays free of module/tonic::Status.
    - run_source/run_destination bodies factored into drive_source/
      drive_destination (both include the fault-notify wrapping) so all three
      entries share one session loop. source_send_half takes plan_options +
      data_plane_host directly instead of the whole SourceSessionConfig.
    
    daemon: run_transfer_session dispatches via run_responder (both resolvers
    built from make_open_resolver); both outcome arms map to the jobs record
    as before.
    
    client: run_pull_session (DESTINATION initiator, DestinationTarget::Fixed,
    in_stream) is the pull-equivalent entry. Not wired to CLI verbs (otp-10).
    
    Tests (1516 -> 1519):
    - pull_session_lands_bytes_and_scores_them: daemon module tree lands in the
      client dest byte-identically; in_stream_carrier_used.
    - old_pull_and_session_produce_identical_trees_and_counts: A/B parity vs
      old pull_sync (byte-identical, equal shared counters — no caveat, both
      SKIP the same-size dest-NEWER cell).
    - unknown_module_refuses_the_pull_session: MODULE_UNKNOWN to a DESTINATION
      initiator.
    Guard: forcing the responder to serve DESTINATION unconditionally makes the
    daemon refuse a DESTINATION initiator with PROTOCOL_VIOLATION (complement
    check) — the 3 pull tests FAIL, the 7 push tests still pass; restored, all
    pass. Role suite (14) still green (establish refactor).
    
    Finding: .review/findings/otp-5-daemon-as-source.md.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
---
 crates/blit-core/src/transfer_session/mod.rs | 400 ++++++++++++++++++++-------
 1 file changed, 294 insertions(+), 106 deletions(-)

diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 48b3169..0ecb287 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -39,7 +39,7 @@ use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
 use crate::remote::transfer::diff_planner;
 use crate::remote::transfer::payload::PreparedPayload;
 use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
-use crate::remote::transfer::source::TransferSource;
+use crate::remote::transfer::source::{FsTransferSource, TransferSource};
 use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
 use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
 use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
@@ -325,6 +325,31 @@ pub enum DestinationTarget {
     Resolve(Box<OpenResolver>),
 }
 
+/// Where a SOURCE responder reads from. Symmetric with
+/// [`DestinationTarget`]: `Fixed` is a source known up front (an
+/// initiator's own tree, or a test), `Resolve` defers to the same
+/// [`OpenResolver`] the destination path uses to map a received
+/// `SessionOpen`'s module name to a local root, from which a
+/// [`FsTransferSource`] is built inside blit-core (so callers stay free
+/// of the concrete source type, exactly as `run_destination` builds its
+/// sink from `dst_root`). A `Resolve` target is meaningful only on a
+/// Responder; an Initiator always knows its own source. Used by
+/// [`run_responder`] for the daemon-as-SOURCE (pull-equivalent, otp-5).
+pub enum SourceResponderTarget {
+    Fixed(Arc<dyn TransferSource>),
+    Resolve(Box<OpenResolver>),
+}
+
+/// What a served session produced, tagged by which role the responder
+/// played. `run_responder` dispatches on the initiator's declared role,
+/// so the caller (the daemon) learns after the fact which half ran.
+pub enum ResponderOutcome {
+    /// The initiator was SOURCE; this end received (push-equivalent).
+    Destination(DestinationOutcome),
+    /// The initiator was DESTINATION; this end sent (pull-equivalent).
+    Source(TransferSummary),
+}
+
 fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
     if open.resume.as_ref().is_some_and(|r| r.enabled) {
         return Err(SessionFault::internal(
@@ -374,22 +399,11 @@ struct Negotiated {
     responder_data_plane: Option<data_plane::ResponderDataPlane>,
 }
 
-/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
-/// scoping requirement). Sends the refusal `SessionError` itself when
-/// it detects the fault locally; returned faults are `peer_notified`.
-async fn establish(
-    transport: &mut FrameTransport,
-    hello: &HelloConfig,
-    endpoint: &SessionEndpoint,
-    local_role: TransferRole,
-    validate_open: &OpenValidator,
-    // Consulted only on the Responder branch, after the received open
-    // passes `validate_open` and before SessionAccept. `None` = the
-    // caller supplies the root itself (Initiator, or fixed-root test).
-    resolve_open: Option<&OpenResolver>,
-) -> Result<Negotiated> {
-    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
-    // direction; no ordering between the two directions.
+/// HELLO both ways, exact match (D-2026-07-05-2). First frame each
+/// direction; no ordering between the two directions. Factored out so a
+/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
+/// and dispatch on the declared role before running a role driver.
+async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
     transport
         .send(frame(Frame::Hello(SessionHello {
             build_id: hello.build_id.clone(),
@@ -427,6 +441,114 @@ async fn establish(
         };
         return Err(notify_and_wrap(transport, fault).await);
     }
+    Ok(())
+}
+
+/// The responder half of establish AFTER the `SessionOpen` is read:
+/// complement check, `validate_open`, endpoint resolution, data-plane
+/// prepare, and `SessionAccept`. Factored out so both `establish` (which
+/// reads the open then calls this) and `run_responder` (which reads the
+/// open, dispatches on the declared role, then calls this with the
+/// resolved local role) share one implementation. Sends the refusal
+/// `SessionError` itself; returned faults are `peer_notified`.
+async fn responder_finish(
+    transport: &mut FrameTransport,
+    open: SessionOpen,
+    local_role: TransferRole,
+    validate_open: &OpenValidator,
+    resolve_open: Option<&OpenResolver>,
+) -> Result<Negotiated> {
+    // The initiator declares ITS role; this responder end must
+    // hold the complement.
+    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
+    if declared != complement(local_role) {
+        return Err(notify_and_wrap(
+            transport,
+            SessionFault::protocol_violation(format!(
+                "initiator declared role {} but this responder is {}",
+                declared.as_str_name(),
+                local_role.as_str_name()
+            )),
+        )
+        .await);
+    }
+    if let Err(fault) = validate_open(&open) {
+        // Refusal is a SessionError instead of SessionAccept,
+        // never a silent close (contract §Phase state machine).
+        return Err(notify_and_wrap(transport, fault).await);
+    }
+    // Responder endpoint resolution (otp-4): map the wire
+    // module/path to a local root and enforce read-only, both
+    // BEFORE SessionAccept so a refusal replaces the accept
+    // (never follows it). The resolver is caller-supplied
+    // (daemon module lookup); a fixed-root responder passes
+    // None and resolves nothing here.
+    let resolved_root = match resolve_open {
+        Some(resolve) => match resolve(&open).await {
+            Ok(resolved) => {
+                // A read-only module is fatal only for a
+                // DESTINATION (it would write); a SOURCE
+                // responder (otp-5, daemon-send) reads happily.
+                if local_role == TransferRole::Destination && resolved.read_only {
+                    return Err(notify_and_wrap(
+                        transport,
+                        SessionFault::read_only("destination module is read-only".to_string()),
+                    )
+                    .await);
+                }
+                Some(resolved.root)
+            }
+            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
+        },
+        None => None,
+    };
+    // Data plane (otp-4b): a DESTINATION responder binds a TCP
+    // listener and grants it, unless the initiator requested the
+    // in-stream carrier or the bind fails (grant-less accept ⇒
+    // in-stream fallback). A SOURCE responder (otp-5, daemon-send)
+    // grants no data plane in otp-5a — the transport/role decoupling
+    // that lets a SOURCE responder bind+grant lands at otp-5b.
+    let responder_data_plane = if local_role == TransferRole::Destination && !open.in_stream_bytes {
+        data_plane::prepare_responder_data_plane().await
+    } else {
+        None
+    };
+    let accept = SessionAccept {
+        // The byte RECEIVER advertises capacity at session
+        // open (D-2026-06-20-1/-2); consumed by the dial when
+        // the data plane lands (otp-4b).
+        receiver_capacity: if local_role == TransferRole::Destination {
+            Some(crate::engine::local_receiver_capacity())
+        } else {
+            None
+        },
+        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
+        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
+    };
+    transport.send(frame(Frame::Accept(accept.clone()))).await?;
+    Ok(Negotiated {
+        open,
+        accept,
+        resolved_root,
+        responder_data_plane,
+    })
+}
+
+/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
+/// scoping requirement). Sends the refusal `SessionError` itself when
+/// it detects the fault locally; returned faults are `peer_notified`.
+async fn establish(
+    transport: &mut FrameTransport,
+    hello: &HelloConfig,
+    endpoint: &SessionEndpoint,
+    local_role: TransferRole,
+    validate_open: &OpenValidator,
+    // Consulted only on the Responder branch, after the received open
+    // passes `validate_open` and before SessionAccept. `None` = the
+    // caller supplies the root itself (Initiator, or fixed-root test).
+    resolve_open: Option<&OpenResolver>,
+) -> Result<Negotiated> {
+    exchange_hello(transport, hello).await?;
 
     match endpoint {
         SessionEndpoint::Initiator { open } => {
@@ -466,84 +588,7 @@ async fn establish(
                     .await)
                 }
             };
-            // The initiator declares ITS role; this responder end must
-            // hold the complement.
-            let declared =
-                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
-            if declared != complement(local_role) {
-                return Err(notify_and_wrap(
-                    transport,
-                    SessionFault::protocol_violation(format!(
-                        "initiator declared role {} but this responder is {}",
-                        declared.as_str_name(),
-                        local_role.as_str_name()
-                    )),
-                )
-                .await);
-            }
-            if let Err(fault) = validate_open(&open) {
-                // Refusal is a SessionError instead of SessionAccept,
-                // never a silent close (contract §Phase state machine).
-                return Err(notify_and_wrap(transport, fault).await);
-            }
-            // Responder endpoint resolution (otp-4): map the wire
-            // module/path to a local root and enforce read-only, both
-            // BEFORE SessionAccept so a refusal replaces the accept
-            // (never follows it). The resolver is caller-supplied
-            // (daemon module lookup); a fixed-root responder passes
-            // None and resolves nothing here.
-            let resolved_root = match resolve_open {
-                Some(resolve) => match resolve(&open).await {
-                    Ok(resolved) => {
-                        // A read-only module is fatal only for a
-                        // DESTINATION (it would write); a SOURCE
-                        // responder (otp-5, daemon-send) reads happily.
-                        if local_role == TransferRole::Destination && resolved.read_only {
-                            return Err(notify_and_wrap(
-                                transport,
-                                SessionFault::read_only(
-                                    "destination module is read-only".to_string(),
-                                ),
-                            )
-                            .await);
-                        }
-                        Some(resolved.root)
-                    }
-                    Err(fault) => return Err(notify_and_wrap(transport, fault).await),
-                },
-                None => None,
-            };
-            // Data plane (otp-4b): a DESTINATION responder binds a TCP
-            // listener and grants it, unless the initiator requested the
-            // in-stream carrier or the bind fails (grant-less accept ⇒
-            // in-stream fallback). A SOURCE responder (otp-5,
-            // daemon-send) will bind on its own branch later; otp-4b's
-            // responder is always the DESTINATION.
-            let responder_data_plane =
-                if local_role == TransferRole::Destination && !open.in_stream_bytes {
-                    data_plane::prepare_responder_data_plane().await
-                } else {
-                    None
-                };
-            let accept = SessionAccept {
-                // The byte RECEIVER advertises capacity at session
-                // open (D-2026-06-20-1/-2); consumed by the dial when
-                // the data plane lands (otp-4b).
-                receiver_capacity: if local_role == TransferRole::Destination {
-                    Some(crate::engine::local_receiver_capacity())
-                } else {
-                    None
-                },
-                // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
-                data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
-            };
-            transport.send(frame(Frame::Accept(accept.clone()))).await?;
-            Ok(Negotiated {
-                open,
-                accept,
-                resolved_root,
-                responder_data_plane,
-            })
+            responder_finish(transport, open, local_role, validate_open, resolve_open).await
         }
     }
 }
@@ -618,13 +663,37 @@ pub async fn run_source(
         &cfg.endpoint,
         TransferRole::Source,
         &source_open_validator,
-        // A SOURCE responder's endpoint resolution (module→root for a
-        // daemon-send) lands with otp-5; otp-4a's daemon is always the
-        // DESTINATION responder, so the source never resolves here.
+        // run_source only ever resolves nothing: a SOURCE *initiator*
+        // owns its own root, and a SOURCE *responder* driven directly
+        // (the in-process role suite) is handed a Fixed source. The
+        // daemon SOURCE responder resolves module→root inside
+        // `run_responder`, not here (otp-5).
         None,
     )
     .await?;
 
+    drive_source(
+        cfg.plan_options,
+        cfg.data_plane_host,
+        &negotiated,
+        transport,
+        source,
+    )
+    .await
+}
+
+/// The SOURCE session body after establish: spawn the receive half,
+/// run the send half, and map a fault to a peer-notified report. Shared
+/// by [`run_source`] (initiator or direct-responder) and
+/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
+/// choreography is single-sourced.
+async fn drive_source(
+    plan_options: PlanOptions,
+    data_plane_host: Option<String>,
+    negotiated: &Negotiated,
+    transport: FrameTransport,
+    source: Arc<dyn TransferSource>,
+) -> Result<TransferSummary> {
     let (mut tx, rx) = transport.split();
     let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
     // Set by the send half the moment ManifestComplete goes out. On
@@ -644,8 +713,9 @@ pub async fn run_source(
     )));
 
     match source_send_half(
-        &cfg,
-        &negotiated,
+        plan_options,
+        data_plane_host.as_deref(),
+        negotiated,
         &mut tx,
         source,
         sent,
@@ -761,8 +831,10 @@ async fn source_recv_half(
     }
 }
 
+#[allow(clippy::too_many_arguments)]
 async fn source_send_half(
-    cfg: &SourceSessionConfig,
+    plan_options: PlanOptions,
+    data_plane_host: Option<&str>,
     negotiated: &Negotiated,
     tx: &mut Box<dyn FrameTx>,
     source: Arc<dyn TransferSource>,
@@ -782,7 +854,7 @@ async fn source_send_half(
     // carrier (fallback), which needs no early setup.
     let mut data_plane = match &negotiated.accept.data_plane {
         Some(grant) => {
-            let host = cfg.data_plane_host.as_deref().ok_or_else(|| {
+            let host = data_plane_host.ok_or_else(|| {
                 eyre::Report::new(SessionFault::internal(
                     "responder granted a TCP data plane but this initiator has no host to dial",
                 ))
@@ -884,7 +956,7 @@ async fn source_send_half(
                     maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                         .await?;
                     let payloads =
-                        diff_planner::plan_push_payloads(batch, source.root(), cfg.plan_options)?;
+                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
                     // A cancel while earlier batches are actively moving
                     // closes the send pipeline under backpressure, so this
                     // queue fails with a data-plane error — prefer the
@@ -897,8 +969,7 @@ async fn source_send_half(
                     }
                 }
                 None => {
-                    send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf)
-                        .await?;
+                    send_payload_records(tx, &source, plan_options, batch, &mut read_buf).await?;
                 }
             }
             continue;
@@ -1400,7 +1471,19 @@ pub async fn run_destination(
         },
     };
 
-    match destination_session(&mut transport, negotiated, &dst_root).await {
+    drive_destination(&mut transport, negotiated, &dst_root).await
+}
+
+/// The DESTINATION session body: run the diff/receive loop and map a
+/// fault to a peer-notified report. Shared by [`run_destination`] and
+/// [`run_responder`] (the daemon DESTINATION responder), so the receive
+/// choreography is single-sourced.
+async fn drive_destination(
+    transport: &mut FrameTransport,
+    negotiated: Negotiated,
+    dst_root: &Path,
+) -> Result<DestinationOutcome> {
+    match destination_session(transport, negotiated, dst_root).await {
         Ok(outcome) => Ok(outcome),
         Err(report) => {
             let mut fault = fault_from_report(report);
@@ -1413,6 +1496,111 @@ pub async fn run_destination(
     }
 }
 
+/// Serve one transfer session as the RESPONDER, dispatching on the
+/// initiator's declared role — the daemon's single serving entry
+/// (contract §Invariants 3: one handshake, roles not directions). A
+/// client that declares SOURCE makes this end the DESTINATION
+/// (push-equivalent, otp-4); a client that declares DESTINATION makes
+/// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
+/// the endpoint resolution for each role; only the one the initiator
+/// selects is used. Returns a [`ResponderOutcome`] tagged with the role
+/// that ran.
+pub async fn run_responder(
+    hello: HelloConfig,
+    transport: FrameTransport,
+    source_target: SourceResponderTarget,
+    dest_target: DestinationTarget,
+) -> Result<ResponderOutcome> {
+    let mut transport = transport;
+    exchange_hello(&mut transport, &hello).await?;
+    let open = match expect_frame(&mut transport).await? {
+        Frame::Open(o) => o,
+        other => {
+            return Err(notify_and_wrap(
+                &mut transport,
+                SessionFault::protocol_violation(format!(
+                    "expected SessionOpen, got {}",
+                    frame_name(&Some(other))
+                )),
+            )
+            .await)
+        }
+    };
+    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
+    match declared {
+        // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
+        TransferRole::Source => {
+            let resolve = match &dest_target {
+                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
+                DestinationTarget::Fixed(_) => None,
+            };
+            let negotiated = responder_finish(
+                &mut transport,
+                open,
+                TransferRole::Destination,
+                &destination_open_validator,
+                resolve,
+            )
+            .await?;
+            let dst_root = match negotiated.resolved_root.clone() {
+                Some(root) => root,
+                None => match &dest_target {
+                    DestinationTarget::Fixed(root) => root.clone(),
+                    DestinationTarget::Resolve(_) => {
+                        return Err(eyre::Report::new(SessionFault::internal(
+                            "resolver target produced no destination root",
+                        )));
+                    }
+                },
+            };
+            let outcome = drive_destination(&mut transport, negotiated, &dst_root).await?;
+            Ok(ResponderOutcome::Destination(outcome))
+        }
+        // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
+        TransferRole::Destination => {
+            let resolve = match &source_target {
+                SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
+                SourceResponderTarget::Fixed(_) => None,
+            };
+            let negotiated = responder_finish(
+                &mut transport,
+                open,
+                TransferRole::Source,
+                &source_open_validator,
+                resolve,
+            )
+            .await?;
+            let source: Arc<dyn TransferSource> = match source_target {
+                SourceResponderTarget::Fixed(source) => source,
+                SourceResponderTarget::Resolve(_) => {
+                    // A Resolve target always yields a root on the
+                    // Responder branch (establish only skips resolution
+                    // on the Initiator branch, which uses Fixed).
+                    let root = negotiated.resolved_root.clone().ok_or_else(|| {
+                        eyre::Report::new(SessionFault::internal(
+                            "resolver target produced no source root",
+                        ))
+                    })?;
+                    Arc::new(FsTransferSource::new(root))
+                }
+            };
+            // The SOURCE owns its planner knobs; a daemon-served source
+            // has no client-supplied ones (§Transport selection). otp-5a
+            // is in-stream only, so there is no data-plane host to dial.
+            let summary =
+                drive_source(PlanOptions::default(), None, &negotiated, transport, source).await?;
+            Ok(ResponderOutcome::Source(summary))
+        }
+        TransferRole::Unspecified => Err(notify_and_wrap(
+            &mut transport,
+            SessionFault::protocol_violation(
+                "initiator declared no role (TRANSFER_ROLE_UNSPECIFIED)",
+            ),
+        )
+        .await),
+    }
+}
+
 fn violation(message: String) -> eyre::Report {
     eyre::Report::new(SessionFault::protocol_violation(message))
 }

exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --format=fuller --stat --patch 84be1cc -- crates/blit-daemon/src/service/transfer.rs crates/blit-core/src/remote/transfer/session_client.rs crates/blit-daemon/src/service/transfer_session_e2e.rs .review/findings/otp-5-daemon-as-source.md' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 84be1cccda812029c1347e6e2cf9661d0d9b4b95
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 6 02:02:32 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 6 02:02:32 2026 -0400

    otp-5a: daemon serves both roles — pull-equivalent over the in-stream carrier
    
    The one served Transfer RPC now handles BOTH directions by the client's
    declared initiator role, not a second code path. A DESTINATION initiator
    makes the daemon the SOURCE Responder (streaming its module tree, the
    pull-equivalent); a SOURCE initiator keeps otp-4's push-equivalent
    unchanged. otp-5a is in-stream only — the data-plane transport/role
    decoupling (responder binds+grants while SENDING; initiator dials while
    RECEIVING) is otp-5b.
    
    blit-core:
    - establish split into exchange_hello + responder_finish so a serving end
      can read the SessionOpen, learn the initiator's declared role, and only
      then pick a driver. establish keeps its shape for the direct role
      drivers (the in-process role suite).
    - run_responder: the daemon's single serving entry. Exchanges HELLO, reads
      the open, dispatches SOURCE-initiator -> local DESTINATION,
      DESTINATION-initiator -> local SOURCE. Carries a DestinationTarget and a
      new SourceResponderTarget; returns ResponderOutcome::{Destination,Source}.
    - SourceResponderTarget::{Fixed, Resolve}: symmetric with DestinationTarget.
      Resolve reuses the same OpenResolver the destination path uses to map
      module -> root, then builds FsTransferSource inside blit-core (read-only
      is ignored for a SOURCE). blit-core stays free of module/tonic::Status.
    - run_source/run_destination bodies factored into drive_source/
      drive_destination (both include the fault-notify wrapping) so all three
      entries share one session loop. source_send_half takes plan_options +
      data_plane_host directly instead of the whole SourceSessionConfig.
    
    daemon: run_transfer_session dispatches via run_responder (both resolvers
    built from make_open_resolver); both outcome arms map to the jobs record
    as before.
    
    client: run_pull_session (DESTINATION initiator, DestinationTarget::Fixed,
    in_stream) is the pull-equivalent entry. Not wired to CLI verbs (otp-10).
    
    Tests (1516 -> 1519):
    - pull_session_lands_bytes_and_scores_them: daemon module tree lands in the
      client dest byte-identically; in_stream_carrier_used.
    - old_pull_and_session_produce_identical_trees_and_counts: A/B parity vs
      old pull_sync (byte-identical, equal shared counters — no caveat, both
      SKIP the same-size dest-NEWER cell).
    - unknown_module_refuses_the_pull_session: MODULE_UNKNOWN to a DESTINATION
      initiator.
    Guard: forcing the responder to serve DESTINATION unconditionally makes the
    daemon refuse a DESTINATION initiator with PROTOCOL_VIOLATION (complement
    check) — the 3 pull tests FAIL, the 7 push tests still pass; restored, all
    pass. Role suite (14) still green (establish refactor).
    
    Finding: .review/findings/otp-5-daemon-as-source.md.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
---
 .review/findings/otp-5-daemon-as-source.md         | 129 ++++++++++++++++++++
 .../src/remote/transfer/session_client.rs          | 131 +++++++++++++++++----
 crates/blit-daemon/src/service/transfer.rs         |  74 +++++++-----
 .../src/service/transfer_session_e2e.rs            | 118 ++++++++++++++++++-
 4 files changed, 397 insertions(+), 55 deletions(-)

diff --git a/.review/findings/otp-5-daemon-as-source.md b/.review/findings/otp-5-daemon-as-source.md
new file mode 100644
index 0000000..9bc1e35
--- /dev/null
+++ b/.review/findings/otp-5-daemon-as-source.md
@@ -0,0 +1,129 @@
+# otp-5 — roles swapped: client initiates as DESTINATION (pull-equivalent)
+
+**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-5.
+**Status**: otp-5a implementing — in-stream pull-equivalent (daemon-as-SOURCE
+responder, client DESTINATION initiator). otp-5b (data plane) pending.
+**Contract**: `docs/TRANSFER_SESSION.md`.
+**Builds on**: otp-4 (daemon serves `Transfer`, client SOURCE initiator). The
+role-parameterized state machine (`run_source`/`run_destination`) already runs
+BOTH assignments in-process (`transfer_session_roles.rs` exercises source-as-
+Responder with a Fixed source). otp-5 adds the *daemon* wiring for the flipped
+direction.
+
+## Staging (mirrors otp-4a/4b)
+
+- **otp-5a (this commit)**: in-stream pull-equivalent. The daemon serves the
+  same `Transfer` RPC and now DISPATCHES on the declared initiator role — a
+  client that declares DESTINATION makes the daemon the SOURCE Responder
+  (resolve module→source root, stream its manifest, send payloads); a client
+  that declares SOURCE keeps otp-4's behavior unchanged. Client gets
+  `run_pull_session` (DESTINATION initiator, in-stream carrier). A/B parity vs
+  old `pull_sync`. **No data plane** for the SOURCE responder yet.
+- **otp-5b (next)**: the data-plane transport/role decoupling. Today the data
+  plane is keyed to ROLE (DEST binds+grants+accepts, SOURCE dials+sends). The
+  plan's transport rule is that the **connection-initiating end dials** (NAT
+  reality) while **byte direction is by role**. For pull the DESTINATION is the
+  *initiator* and must dial; the SOURCE is the *responder* and must bind+grant+
+  accept while *sending* bytes. That decoupling (responder-binds vs role-sends)
+  is otp-5b.
+
+## What otp-5a proves
+
+The pull-equivalent rides the one unified session end to end: a daemon serving
+`Transfer` streams its module tree to a client that initiated as DESTINATION and
+wrote it **byte-identically** to what old `pull_sync` produces, with equal
+shared summary counters (the converge-up bar), over the in-stream carrier. The
+same served RPC still handles push (otp-4) — role is chosen by the client's
+`SessionOpen.initiator_role`, never by a second code path.
+
+## Approach (as implemented)
+
+- **Handshake split** (`transfer_session/mod.rs`): `establish` is factored into
+  `exchange_hello` (HELLO both ways, exact match — D-2026-07-05-2) and
+  `responder_finish` (complement check → validate → resolve → data-plane prepare
+  → `SessionAccept`, taking an already-read `SessionOpen`). `establish` keeps its
+  old shape for the direct role drivers (the in-process role suite); the split
+  lets a serving end read the open, learn the initiator's declared role, and
+  only then pick which driver to run.
+- **Unified responder** (`run_responder`): the daemon's single serving entry. It
+  exchanges HELLO, reads the `SessionOpen`, and dispatches on
+  `initiator_role`: initiator SOURCE ⇒ local DESTINATION (existing receive
+  path); initiator DESTINATION ⇒ local SOURCE (new send path). It carries a
+  `DestinationTarget` and a new `SourceResponderTarget` and uses whichever the
+  role selects. Returns `ResponderOutcome::{Destination,Source}`.
+- **`SourceResponderTarget`**: `Fixed(Arc<dyn TransferSource>)` (a root known up
+  front — tests) or `Resolve(Box<OpenResolver>)` (the daemon: resolve module→
+  root via the SAME `OpenResolver` the DESTINATION path uses, then build
+  `FsTransferSource::new(root)` inside blit-core — symmetric with how
+  `run_destination` builds its sink from `dst_root`). blit-core stays free of
+  module/`tonic::Status` types; read-only is ignored for a SOURCE (reading a
+  read-only module is fine — the establish read-only refusal is DESTINATION-only,
+  already so since otp-4a).
+- **Body reuse**: run_source's post-establish body is `drive_source` and
+  run_destination's is `drive_destination` (both include the fault-notify
+  wrapping). `run_source`/`run_destination`/`run_responder` all call them, so
+  the source/destination session loops are single-sourced. `source_send_half`
+  now takes `plan_options` + `data_plane_host` directly instead of the whole
+  `SourceSessionConfig` (run_responder has no initiator config).
+- **Daemon** (`service/transfer.rs`): `run_transfer_session` builds both a
+  source and a destination resolver (`make_open_resolver` cloned) and calls
+  `run_responder`; both outcome arms map to `Ok(())`/`Err(Status)` for the jobs
+  record exactly as before. The `core.rs::transfer` dispatcher is unchanged
+  (still `resolve_transfer_session_outcome` + `ActiveJobKind::Push`; a pull
+  served by the daemon is still a daemon-side transfer row — kind taxonomy
+  revisited at cutover).
+- **Client** (`remote/transfer/session_client.rs`): `run_pull_session(endpoint,
+  dest_root, PullSessionOptions)` opens the bidi RPC, declares
+  `initiator_role = DESTINATION`, and runs `run_destination` as Initiator with
+  `DestinationTarget::Fixed(dest_root)`. `in_stream_bytes = true` (otp-5a is
+  in-stream only; the SOURCE responder grants no data plane regardless, so the
+  carrier is in-stream either way — the flag is set for clarity and forward
+  intent). Not wired to CLI verbs (otp-10).
+
+## Compare semantics
+
+Unchanged from otp-4a: the destination is the one diff owner and uses the
+mode-aware `header_transfer_status`; the same-size + dest-NEWER cell resolves to
+the data-safe SKIP (the still-open owner-ack question from otp-4a; not reopened
+here). Old pull already SKIPs that cell, so A/B-vs-old-pull is byte-identical
+with no caveat (unlike the push A/B, where old push clobbers).
+
+## Files
+
+- `crates/blit-core/src/transfer_session/mod.rs` — `exchange_hello`,
+  `responder_finish`, `drive_source`, `drive_destination`, `run_responder`,
+  `SourceResponderTarget`, `ResponderOutcome`; `source_send_half` signature.
+- `crates/blit-daemon/src/service/transfer.rs` — `run_transfer_session` via
+  `run_responder`.
+- `crates/blit-core/src/remote/transfer/session_client.rs` — `run_pull_session`,
+  `PullSessionOptions`.
+- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — pull e2e tests.
+
+## Tests
+
+New e2e (real loopback daemon serving as SOURCE):
+- `pull_session_lands_bytes_and_scores_them` — the daemon's module tree lands in
+  the client's dest byte-identically; summary files/bytes correct;
+  `in_stream_carrier_used` true.
+- `old_pull_and_session_produce_identical_trees_and_counts` — A/B parity: same
+  daemon source through OLD `pull_sync` and the NEW session → byte-identical
+  dest trees + equal shared counters.
+- `unknown_module_refuses_the_pull_session` — `MODULE_UNKNOWN` fault to a
+  DESTINATION initiator.
+
+Guard proof: the daemon dispatch is guarded by
+`old_pull_and_session_produce_identical_trees_and_counts` — reverting the
+`run_responder` dispatch (leaving `run_destination` unconditional) makes the
+daemon refuse a DESTINATION initiator with `PROTOCOL_VIOLATION` (the complement
+check), failing the pull tests; restoring passes.
+
+## Known gaps (carried into otp-5b / later)
+
+- **Data plane for the SOURCE responder**: otp-5a is in-stream only. The
+  transport/role decoupling (responder binds+accepts while sending; initiator
+  dials while receiving) is otp-5b.
+- **Source plan_options for the daemon**: `run_responder`'s source path uses
+  `PlanOptions::default()` (the SOURCE owns planner knobs; the daemon has no
+  client-supplied ones). Matches today's daemon-send defaults.
+- Mirror/filters otp-6; resume otp-7; fallback-carrier parity otp-8; delegated
+  otp-9; cutover/deletion otp-10.
diff --git a/crates/blit-core/src/remote/transfer/session_client.rs b/crates/blit-core/src/remote/transfer/session_client.rs
index 89f38e9..a9f9389 100644
--- a/crates/blit-core/src/remote/transfer/session_client.rs
+++ b/crates/blit-core/src/remote/transfer/session_client.rs
@@ -1,17 +1,20 @@
-//! Client-side entry for initiating a unified transfer session as the
-//! SOURCE role (otp-4a).
+//! Client-side entry for initiating a unified transfer session.
 //!
-//! Builds a gRPC-backed [`FrameTransport`] over `BlitClient::transfer`
-//! and runs [`run_source`], so a CLI push becomes "open the Transfer
-//! RPC, declare SOURCE, stream the manifest + payloads." This is the
-//! push-equivalent on the unified path; the daemon answers by running
-//! `run_destination` as the Responder.
+//! [`run_push_session`] declares the SOURCE role (push-equivalent,
+//! otp-4): open the `Transfer` RPC, stream the manifest + payloads; the
+//! daemon answers as the DESTINATION Responder. [`run_pull_session`]
+//! declares the DESTINATION role (pull-equivalent, otp-5a): the daemon
+//! answers as the SOURCE Responder and streams its module tree, which
+//! this end diffs and writes. Both build a gRPC-backed [`FrameTransport`]
+//! over `BlitClient::transfer` and run the matching role driver; role is
+//! carried in `SessionOpen.initiator_role`, never a second code path.
 //!
-//! Not yet wired to CLI verbs — the verbs keep riding the old push
-//! path until the otp-10 cutover; today the parity tests drive this.
-//! otp-4a uses the in-stream byte carrier only (`in_stream_bytes`);
-//! the TCP data plane lands at otp-4b.
+//! Not yet wired to CLI verbs — the verbs keep riding the old paths
+//! until the otp-10 cutover; today the parity tests drive this. push
+//! defaults to the TCP data plane (otp-4b); pull is in-stream only until
+//! otp-5b adds the SOURCE-responder data plane.
 
+use std::path::PathBuf;
 use std::sync::Arc;
 use std::time::Duration;
 
@@ -26,7 +29,10 @@ use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
 use crate::remote::transfer::source::TransferSource;
 use crate::transfer_plan::PlanOptions;
 use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
-use crate::transfer_session::{run_source, HelloConfig, SessionEndpoint, SourceSessionConfig};
+use crate::transfer_session::{
+    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
+    HelloConfig, SessionEndpoint, SourceSessionConfig,
+};
 
 /// The push-shaped subset of session options otp-4a/4b supports. Mirror,
 /// filters, and resume are refused at OPEN until their slices land
@@ -65,19 +71,8 @@ pub async fn run_push_session(
     options: PushSessionOptions,
 ) -> Result<TransferSummary> {
     // The responder resolves module→root; the initiator's own local
-    // path never crosses the wire (contract §SessionOpen). Empty module
-    // targets the daemon's default root export.
-    let (module, path) = match &endpoint.path {
-        RemotePath::Module { module, rel_path } => {
-            (module.clone(), rel_path.to_string_lossy().into_owned())
-        }
-        RemotePath::Root { rel_path } => (String::new(), rel_path.to_string_lossy().into_owned()),
-        RemotePath::Discovery => {
-            return Err(eyre!(
-                "a transfer session needs a resolved module or root endpoint, not a discovery form"
-            ));
-        }
-    };
+    // path never crosses the wire (contract §SessionOpen).
+    let (module, path) = endpoint_module_path(endpoint)?;
 
     let mut client = connect_transfer_client(endpoint).await?;
 
@@ -117,6 +112,92 @@ pub async fn run_push_session(
     run_source(cfg, transport, source).await
 }
 
+/// The pull-shaped subset of session options otp-5a supports. Mirror,
+/// filters, and resume are refused at OPEN until their slices land, so
+/// they are intentionally absent here. The DESTINATION owns the compare
+/// decision; the SOURCE owns the planner knobs (none cross the wire).
+pub struct PullSessionOptions {
+    pub compare_mode: ComparisonMode,
+    pub ignore_existing: bool,
+    pub require_complete_scan: bool,
+}
+
+impl Default for PullSessionOptions {
+    fn default() -> Self {
+        Self {
+            compare_mode: ComparisonMode::SizeMtime,
+            ignore_existing: false,
+            require_complete_scan: false,
+        }
+    }
+}
+
+/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
+/// session pulling the endpoint's module/path tree into `dest_root`
+/// (pull-equivalent, otp-5a). The client initiates and declares
+/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
+/// its module tree). Returns the [`DestinationOutcome`] this end
+/// computed (contract: the DESTINATION is the scorer).
+///
+/// otp-5a rides the in-stream byte carrier: the SOURCE responder grants
+/// no TCP data plane yet (the transport/role decoupling that lets a
+/// SOURCE responder bind+grant lands at otp-5b), so `in_stream_bytes` is
+/// set to make the carrier explicit. Not wired to CLI verbs (otp-10).
+pub async fn run_pull_session(
+    endpoint: &RemoteEndpoint,
+    dest_root: PathBuf,
+    options: PullSessionOptions,
+) -> Result<DestinationOutcome> {
+    let (module, path) = endpoint_module_path(endpoint)?;
+
+    let mut client = connect_transfer_client(endpoint).await?;
+
+    let open = SessionOpen {
+        initiator_role: TransferRole::Destination as i32,
+        module,
+        path,
+        compare_mode: options.compare_mode as i32,
+        ignore_existing: options.ignore_existing,
+        require_complete_scan: options.require_complete_scan,
+        // otp-5a is in-stream only (the SOURCE responder grants no data
+        // plane); set the flag so the carrier is explicit and stable if
+        // a data-plane grant is added at otp-5b.
+        in_stream_bytes: true,
+        ..Default::default()
+    };
+
+    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
+    let inbound = client
+        .transfer(ReceiverStream::new(out_rx))
+        .await
+        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
+        .into_inner();
+    let transport = grpc_client_transport(out_tx, inbound);
+
+    let cfg = DestinationSessionConfig {
+        hello: HelloConfig::default(),
+        endpoint: SessionEndpoint::initiator(open),
+    };
+    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
+}
+
+/// Derive the wire `(module, path)` from a resolved endpoint. Empty
+/// module targets the daemon's default root export; a discovery-form
+/// endpoint is not resolvable to a transfer target.
+fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
+    match &endpoint.path {
+        RemotePath::Module { module, rel_path } => {
+            Ok((module.clone(), rel_path.to_string_lossy().into_owned()))
+        }
+        RemotePath::Root { rel_path } => {
+            Ok((String::new(), rel_path.to_string_lossy().into_owned()))
+        }
+        RemotePath::Discovery => Err(eyre!(
+            "a transfer session needs a resolved module or root endpoint, not a discovery form"
+        )),
+    }
+}
+
 /// Build a `BlitClient` over `endpoint`'s control-plane URI with the
 /// same bounded-connect policy `RemotePushClient::connect` uses.
 async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
diff --git a/crates/blit-daemon/src/service/transfer.rs b/crates/blit-daemon/src/service/transfer.rs
index 1916775..310b4ff 100644
--- a/crates/blit-daemon/src/service/transfer.rs
+++ b/crates/blit-daemon/src/service/transfer.rs
@@ -2,12 +2,14 @@
 //!
 //! otp-1 (D-2026-07-05-4) landed the wire surface (the RPC, the frame
 //! set, `docs/TRANSFER_SESSION.md`) with the handler refusing
-//! UNIMPLEMENTED. otp-4a lands the behavior: the daemon serves the RPC
-//! by running `blit_core::transfer_session::run_destination` as the
-//! Responder — the byte RECEIVER of a client-initiated SOURCE push.
-//! The dispatcher in `core.rs::transfer` mirrors `push`: register a
-//! jobs row, race the session against cancel/hangup via
-//! `resolve_streaming_outcome`, return the response `ReceiverStream`.
+//! UNIMPLEMENTED. otp-4a landed the behavior; otp-5a makes the daemon
+//! serve BOTH roles: it runs `blit_core::transfer_session::run_responder`,
+//! which dispatches on the client's declared initiator role — a SOURCE
+//! initiator makes the daemon the DESTINATION (push-equivalent), a
+//! DESTINATION initiator makes it the SOURCE (pull-equivalent, streaming
+//! its module tree). The dispatcher in `core.rs::transfer` mirrors
+//! `push`: register a jobs row, race the session against cancel/hangup
+//! via `resolve_streaming_outcome`, return the response `ReceiverStream`.
 //!
 //! This module owns the two daemon-specific pieces the session driver
 //! in blit-core cannot: (1) the [`OpenResolver`] that maps a wire
@@ -15,10 +17,11 @@
 //! free of module config and `tonic::Status`), and (2) the transport
 //! assembly + outcome mapping.
 //!
-//! otp-4a uses the in-stream byte carrier only; the TCP data plane
-//! grant + resize land at otp-4b. Progress-byte wiring
-//! (`with_byte_progress`) is not threaded yet — session rows report
-//! `bytes_completed=0`, matching today's push rows.
+//! Carrier: the push-equivalent (daemon DESTINATION) rides the TCP data
+//! plane (otp-4b); the pull-equivalent (daemon SOURCE) is in-stream only
+//! until otp-5b adds the SOURCE-responder data plane. Progress-byte
+//! wiring (`with_byte_progress`) is not threaded yet — session rows
+//! report `bytes_completed=0`, matching today's push rows.
 
 use std::collections::HashMap;
 use std::sync::Arc;
@@ -31,8 +34,8 @@ use blit_core::generated::session_error::Code;
 use blit_core::generated::{SessionOpen, TransferFrame};
 use blit_core::transfer_session::transport::grpc_daemon_transport;
 use blit_core::transfer_session::{
-    run_destination, DestinationSessionConfig, DestinationTarget, HelloConfig, OpenResolver,
-    ResolvedEndpoint, SessionEndpoint, SessionFault,
+    run_responder, DestinationTarget, HelloConfig, OpenResolver, ResolvedEndpoint, SessionFault,
+    SourceResponderTarget,
 };
 
 use super::util::{resolve_contained_path, resolve_module, resolve_relative_path};
@@ -90,13 +93,16 @@ pub(crate) fn make_open_resolver(
     })
 }
 
-/// Run one daemon-side transfer session to completion as the DESTINATION
-/// Responder, returning `Ok(())` on a clean transfer or `Err(Status)`
-/// carrying the session fault's message for the jobs record. The
-/// session communicates its own refusals to the peer as `SessionError`
-/// *frames* (via the response stream); this `Status` is for the
-/// daemon's outcome record and `resolve_streaming_outcome`'s terminal
-/// handling, not the primary error channel.
+/// Run one daemon-side transfer session to completion, dispatching on
+/// the client's declared initiator role via [`run_responder`]: a SOURCE
+/// initiator makes the daemon the DESTINATION (push-equivalent, otp-4);
+/// a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent,
+/// otp-5). Returns `Ok(())` on a clean transfer or `Err(Status)`
+/// carrying the session fault's message for the jobs record. The session
+/// communicates its own refusals to the peer as `SessionError` *frames*
+/// (via the response stream); this `Status` is for the daemon's outcome
+/// record and `resolve_streaming_outcome`'s terminal handling, not the
+/// primary error channel.
 pub(crate) async fn run_transfer_session(
     modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
     default_root: Option<RootExport>,
@@ -104,16 +110,28 @@ pub(crate) async fn run_transfer_session(
     tx: mpsc::Sender<Result<TransferFrame, Status>>,
 ) -> Result<(), Status> {
     let transport = grpc_daemon_transport(tx, inbound);
-    let resolver = make_open_resolver(modules, default_root);
-    let cfg = DestinationSessionConfig {
-        hello: HelloConfig::default(),
-        endpoint: SessionEndpoint::Responder,
-    };
-    match run_destination(cfg, transport, DestinationTarget::Resolve(resolver)).await {
-        Ok(_outcome) => Ok(()),
+    // The same module→root resolver serves both roles; only the one the
+    // initiator's declared role selects is consulted. Two clones so each
+    // target owns its resolver (the closure clones its captured handles
+    // per call, so this is cheap).
+    let source_resolver = make_open_resolver(Arc::clone(&modules), default_root.clone());
+    let dest_resolver = make_open_resolver(modules, default_root);
+    let outcome = run_responder(
+        HelloConfig::default(),
+        transport,
+        SourceResponderTarget::Resolve(source_resolver),
+        DestinationTarget::Resolve(dest_resolver),
+    )
+    .await;
+    match outcome {
+        // Either role completing cleanly is a successful transfer; the
+        // daemon record does not distinguish push- from pull-equivalent
+        // (the jobs kind stays Push until the taxonomy is revisited at
+        // cutover — see the dispatcher).
+        Ok(_) => Ok(()),
         Err(report) => {
-            // run_destination already emitted a SessionError frame to
-            // the peer; surface the reason for the record.
+            // run_responder already emitted a SessionError frame to the
+            // peer; surface the reason for the record.
             let msg = report
                 .downcast_ref::<SessionFault>()
                 .map(|f| f.message.clone())
diff --git a/crates/blit-daemon/src/service/transfer_session_e2e.rs b/crates/blit-daemon/src/service/transfer_session_e2e.rs
index e1d06db..fd3da5f 100644
--- a/crates/blit-daemon/src/service/transfer_session_e2e.rs
+++ b/crates/blit-daemon/src/service/transfer_session_e2e.rs
@@ -17,6 +17,12 @@
 //!   is NEWER than the source is SKIPPED (the data-safe, pull-style
 //!   converged behavior — see the finding doc's compare decision).
 //!
+//! otp-5a adds the pull-equivalent (roles flipped): the client initiates
+//! as DESTINATION and the daemon streams its module tree as the SOURCE
+//! Responder over the in-stream carrier. Those tests pin a byte-identical
+//! landing + A/B parity vs old `pull_sync`, proving the one served RPC
+//! handles both directions by the declared role, not a second code path.
+//!
 //! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
 //! `BlitService` on loopback + a real client. Only in-crate tests can
 //! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
@@ -29,9 +35,12 @@ use std::sync::Arc;
 use blit_core::fs_enum::FileFilter;
 use blit_core::generated::blit_server::BlitServer;
 use blit_core::generated::{session_error, MirrorMode};
-use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
+use blit_core::remote::pull::PullSyncOptions;
+use blit_core::remote::transfer::session_client::{
+    run_pull_session, run_push_session, PullSessionOptions, PushSessionOptions,
+};
 use blit_core::remote::transfer::source::FsTransferSource;
-use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
+use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
 use blit_core::transfer_session::SessionFault;
 use tokio::sync::oneshot;
 
@@ -555,3 +564,108 @@ async fn same_size_newer_destination_is_skipped_not_clobbered() {
     );
     daemon.stop().await;
 }
+
+// ---------------------------------------------------------------------------
+// otp-5a: pull-equivalent (client initiates as DESTINATION, daemon is SOURCE)
+// ---------------------------------------------------------------------------
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn pull_session_lands_bytes_and_scores_them() {
+    // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
+    // initiates as DESTINATION and the daemon streams its module tree
+    // (otp-5a). The SOURCE responder grants no data plane, so the carrier
+    // is the in-stream fallback. `dest_root` here is the module (source)
+    // root — the harness field name is push-oriented.
+    let daemon = Daemon::start(false).await;
+    write_tree(&daemon.dest_root, &small_tree());
+
+    let dest = tempfile::tempdir().unwrap();
+    let outcome = run_pull_session(
+        &daemon.endpoint,
+        dest.path().to_path_buf(),
+        PullSessionOptions::default(),
+    )
+    .await
+    .expect("session pull succeeds");
+
+    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
+    assert_eq!(
+        outcome.summary.bytes_transferred,
+        small_tree()
+            .iter()
+            .map(|(_, c, _)| c.len() as u64)
+            .sum::<u64>()
+    );
+    assert!(
+        outcome.summary.in_stream_carrier_used,
+        "otp-5a pull rides the in-stream carrier (no SOURCE-responder data plane yet)"
+    );
+    assert_trees_identical(&daemon.dest_root, dest.path());
+    daemon.stop().await;
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn old_pull_and_session_produce_identical_trees_and_counts() {
+    // Arm A: OLD pull_sync into a client-local dest.
+    let daemon_a = Daemon::start(false).await;
+    write_tree(&daemon_a.dest_root, &small_tree());
+    let dest_a = tempfile::tempdir().unwrap();
+    let mut pull_client = RemotePullClient::connect(daemon_a.endpoint.clone())
+        .await
+        .expect("old pull client connects");
+    let report = pull_client
+        .pull_sync(
+            dest_a.path(),
+            Vec::new(),
+            &PullSyncOptions::default(),
+            false,
+            None,
+        )
+        .await
+        .expect("old pull succeeds");
+
+    // Arm B: NEW session (client DESTINATION initiator).
+    let daemon_b = Daemon::start(false).await;
+    write_tree(&daemon_b.dest_root, &small_tree());
+    let dest_b = tempfile::tempdir().unwrap();
+    let outcome = run_pull_session(
+        &daemon_b.endpoint,
+        dest_b.path().to_path_buf(),
+        PullSessionOptions::default(),
+    )
+    .await
+    .expect("session pull succeeds");
+
+    // Both dests equal their source module and each other.
+    assert_trees_identical(&daemon_a.dest_root, dest_a.path());
+    assert_trees_identical(&daemon_b.dest_root, dest_b.path());
+    assert_trees_identical(dest_a.path(), dest_b.path());
+
+    // Shared counters agree (transport-specific fields have no cross
+    // analog and are not compared). Old pull already SKIPs the same-size
+    // dest-NEWER cell, so this A/B is byte-identical with no caveat —
+    // unlike the push A/B where old push clobbers.
+    assert_eq!(
+        report.files_transferred as u64,
+        outcome.summary.files_transferred
+    );
+    assert_eq!(report.bytes_transferred, outcome.summary.bytes_transferred);
+
+    daemon_a.stop().await;
+    daemon_b.stop().await;
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn unknown_module_refuses_the_pull_session() {
+    let daemon = Daemon::start(false).await;
+    let dest = tempfile::tempdir().unwrap();
+    let err = run_pull_session(
+        &daemon.endpoint_for_missing_module(),
+        dest.path().to_path_buf(),
+        PullSessionOptions::default(),
+    )
+    .await
+    .expect_err("unknown module must refuse the pull session");
+    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
+    daemon.stop().await;
+}

codex
The committed tests do add three new e2e cases. I’m checking whether the implementation actually preserves the otp-4 push path and whether the new pull path exercises the same responder dispatch it claims to guard.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '360,620p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   360	        .filter
   361	        .as_ref()
   362	        .is_some_and(|f| *f != FilterSpec::default())
   363	    {
   364	        return Err(SessionFault::internal(
   365	            "filters are not implemented on the unified session yet (otp-6)",
   366	        ));
   367	    }
   368	    Ok(())
   369	}
   370	
   371	fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   372	    if open.mirror_enabled {
   373	        return Err(SessionFault::internal(
   374	            "mirror is not implemented on the unified session yet (otp-6)",
   375	        ));
   376	    }
   377	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   378	        return Err(SessionFault::internal(
   379	            "resume is not implemented on the unified session yet (otp-7)",
   380	        ));
   381	    }
   382	    Ok(())
   383	}
   384	
   385	/// Outcome of the HELLO + OPEN phases.
   386	struct Negotiated {
   387	    open: SessionOpen,
   388	    /// The responder's reply. The SOURCE initiator reads
   389	    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
   390	    accept: SessionAccept,
   391	    /// The write root a Responder's [`OpenResolver`] produced from the
   392	    /// received open, if one was supplied; `None` for an Initiator or a
   393	    /// fixed-root Responder (the caller supplies the root then).
   394	    resolved_root: Option<PathBuf>,
   395	    /// The bound data-plane listener + credentials a DESTINATION
   396	    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
   397	    /// on an Initiator, or when the responder granted no data plane
   398	    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
   399	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
   400	}
   401	
   402	/// HELLO both ways, exact match (D-2026-07-05-2). First frame each
   403	/// direction; no ordering between the two directions. Factored out so a
   404	/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
   405	/// and dispatch on the declared role before running a role driver.
   406	async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
   407	    transport
   408	        .send(frame(Frame::Hello(SessionHello {
   409	            build_id: hello.build_id.clone(),
   410	            contract_version: hello.contract_version,
   411	        })))
   412	        .await?;
   413	
   414	    let peer_hello = match expect_frame(transport).await? {
   415	        Frame::Hello(h) => h,
   416	        other => {
   417	            return Err(notify_and_wrap(
   418	                transport,
   419	                SessionFault::protocol_violation(format!(
   420	                    "expected SessionHello, got {}",
   421	                    frame_name(&Some(other))
   422	                )),
   423	            )
   424	            .await)
   425	        }
   426	    };
   427	
   428	    if peer_hello.build_id != hello.build_id
   429	        || peer_hello.contract_version != hello.contract_version
   430	    {
   431	        let fault = SessionFault {
   432	            code: session_error::Code::BuildMismatch,
   433	            message: format!(
   434	                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
   435	                hello.build_id, hello.contract_version,
   436	                peer_hello.build_id, peer_hello.contract_version,
   437	            ),
   438	            local_build_id: hello.build_id.clone(),
   439	            peer_build_id: peer_hello.build_id.clone(),
   440	            peer_notified: false,
   441	        };
   442	        return Err(notify_and_wrap(transport, fault).await);
   443	    }
   444	    Ok(())
   445	}
   446	
   447	/// The responder half of establish AFTER the `SessionOpen` is read:
   448	/// complement check, `validate_open`, endpoint resolution, data-plane
   449	/// prepare, and `SessionAccept`. Factored out so both `establish` (which
   450	/// reads the open then calls this) and `run_responder` (which reads the
   451	/// open, dispatches on the declared role, then calls this with the
   452	/// resolved local role) share one implementation. Sends the refusal
   453	/// `SessionError` itself; returned faults are `peer_notified`.
   454	async fn responder_finish(
   455	    transport: &mut FrameTransport,
   456	    open: SessionOpen,
   457	    local_role: TransferRole,
   458	    validate_open: &OpenValidator,
   459	    resolve_open: Option<&OpenResolver>,
   460	) -> Result<Negotiated> {
   461	    // The initiator declares ITS role; this responder end must
   462	    // hold the complement.
   463	    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   464	    if declared != complement(local_role) {
   465	        return Err(notify_and_wrap(
   466	            transport,
   467	            SessionFault::protocol_violation(format!(
   468	                "initiator declared role {} but this responder is {}",
   469	                declared.as_str_name(),
   470	                local_role.as_str_name()
   471	            )),
   472	        )
   473	        .await);
   474	    }
   475	    if let Err(fault) = validate_open(&open) {
   476	        // Refusal is a SessionError instead of SessionAccept,
   477	        // never a silent close (contract §Phase state machine).
   478	        return Err(notify_and_wrap(transport, fault).await);
   479	    }
   480	    // Responder endpoint resolution (otp-4): map the wire
   481	    // module/path to a local root and enforce read-only, both
   482	    // BEFORE SessionAccept so a refusal replaces the accept
   483	    // (never follows it). The resolver is caller-supplied
   484	    // (daemon module lookup); a fixed-root responder passes
   485	    // None and resolves nothing here.
   486	    let resolved_root = match resolve_open {
   487	        Some(resolve) => match resolve(&open).await {
   488	            Ok(resolved) => {
   489	                // A read-only module is fatal only for a
   490	                // DESTINATION (it would write); a SOURCE
   491	                // responder (otp-5, daemon-send) reads happily.
   492	                if local_role == TransferRole::Destination && resolved.read_only {
   493	                    return Err(notify_and_wrap(
   494	                        transport,
   495	                        SessionFault::read_only("destination module is read-only".to_string()),
   496	                    )
   497	                    .await);
   498	                }
   499	                Some(resolved.root)
   500	            }
   501	            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
   502	        },
   503	        None => None,
   504	    };
   505	    // Data plane (otp-4b): a DESTINATION responder binds a TCP
   506	    // listener and grants it, unless the initiator requested the
   507	    // in-stream carrier or the bind fails (grant-less accept ⇒
   508	    // in-stream fallback). A SOURCE responder (otp-5, daemon-send)
   509	    // grants no data plane in otp-5a — the transport/role decoupling
   510	    // that lets a SOURCE responder bind+grant lands at otp-5b.
   511	    let responder_data_plane = if local_role == TransferRole::Destination && !open.in_stream_bytes {
   512	        data_plane::prepare_responder_data_plane().await
   513	    } else {
   514	        None
   515	    };
   516	    let accept = SessionAccept {
   517	        // The byte RECEIVER advertises capacity at session
   518	        // open (D-2026-06-20-1/-2); consumed by the dial when
   519	        // the data plane lands (otp-4b).
   520	        receiver_capacity: if local_role == TransferRole::Destination {
   521	            Some(crate::engine::local_receiver_capacity())
   522	        } else {
   523	            None
   524	        },
   525	        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
   526	        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
   527	    };
   528	    transport.send(frame(Frame::Accept(accept.clone()))).await?;
   529	    Ok(Negotiated {
   530	        open,
   531	        accept,
   532	        resolved_root,
   533	        responder_data_plane,
   534	    })
   535	}
   536	
   537	/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
   538	/// scoping requirement). Sends the refusal `SessionError` itself when
   539	/// it detects the fault locally; returned faults are `peer_notified`.
   540	async fn establish(
   541	    transport: &mut FrameTransport,
   542	    hello: &HelloConfig,
   543	    endpoint: &SessionEndpoint,
   544	    local_role: TransferRole,
   545	    validate_open: &OpenValidator,
   546	    // Consulted only on the Responder branch, after the received open
   547	    // passes `validate_open` and before SessionAccept. `None` = the
   548	    // caller supplies the root itself (Initiator, or fixed-root test).
   549	    resolve_open: Option<&OpenResolver>,
   550	) -> Result<Negotiated> {
   551	    exchange_hello(transport, hello).await?;
   552	
   553	    match endpoint {
   554	        SessionEndpoint::Initiator { open } => {
   555	            let open = open.as_ref().clone();
   556	            transport.send(frame(Frame::Open(open.clone()))).await?;
   557	            let accept = match expect_frame(transport).await? {
   558	                Frame::Accept(a) => a,
   559	                other => {
   560	                    return Err(notify_and_wrap(
   561	                        transport,
   562	                        SessionFault::protocol_violation(format!(
   563	                            "expected SessionAccept, got {}",
   564	                            frame_name(&Some(other))
   565	                        )),
   566	                    )
   567	                    .await)
   568	                }
   569	            };
   570	            Ok(Negotiated {
   571	                open,
   572	                accept,
   573	                resolved_root: None,
   574	                responder_data_plane: None,
   575	            })
   576	        }
   577	        SessionEndpoint::Responder => {
   578	            let open = match expect_frame(transport).await? {
   579	                Frame::Open(o) => o,
   580	                other => {
   581	                    return Err(notify_and_wrap(
   582	                        transport,
   583	                        SessionFault::protocol_violation(format!(
   584	                            "expected SessionOpen, got {}",
   585	                            frame_name(&Some(other))
   586	                        )),
   587	                    )
   588	                    .await)
   589	                }
   590	            };
   591	            responder_finish(transport, open, local_role, validate_open, resolve_open).await
   592	        }
   593	    }
   594	}
   595	
   596	/// Receive one frame during establish; peer errors and closes become
   597	/// terminal faults.
   598	async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
   599	    match transport.recv().await? {
   600	        Some(TransferFrame {
   601	            frame: Some(Frame::Error(err)),
   602	        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
   603	        Some(TransferFrame { frame: Some(f) }) => Ok(f),
   604	        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
   605	            SessionFault::protocol_violation("frame with empty oneof"),
   606	        )),
   607	        None => Err(eyre::Report::new(SessionFault::internal(
   608	            "peer closed during session establish",
   609	        ))),
   610	    }
   611	}
   612	
   613	/// Send the fault to the peer (best effort), mark it notified, and
   614	/// wrap it for return.
   615	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   616	    let _ = transport.send(error_frame(&fault)).await;
   617	    fault.peer_notified = true;
   618	    eyre::Report::new(fault)
   619	}
   620	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '620,1020p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   620	
   621	// ---------------------------------------------------------------------------
   622	// SOURCE driver
   623	// ---------------------------------------------------------------------------
   624	
   625	/// Events the source's receive half forwards to its send half. The
   626	/// channel is unbounded but bounded by construction: every `Need`
   627	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   628	/// fault the session), so the queue never exceeds the source's own
   629	/// manifest size — the contract's bounded-buffering rule holds.
   630	enum SourceEvent {
   631	    Need(FileHeader),
   632	    NeedComplete,
   633	    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
   634	    /// send half dials the epoch-N socket on `accepted`.
   635	    ResizeAck(DataPlaneResizeAck),
   636	    Summary(TransferSummary),
   637	    Fault(SessionFault),
   638	}
   639	
   640	/// Run the SOURCE role of one transfer session over `transport`.
   641	/// Returns the destination-computed `TransferSummary` (contract: the
   642	/// end that wrote the bytes is the end that attests to them).
   643	pub async fn run_source(
   644	    cfg: SourceSessionConfig,
   645	    transport: FrameTransport,
   646	    source: Arc<dyn TransferSource>,
   647	) -> Result<TransferSummary> {
   648	    let mut transport = transport;
   649	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   650	        // Own-config coherence: a source initiator declares SOURCE.
   651	        let declared = TransferRole::try_from(open.initiator_role);
   652	        if declared != Ok(TransferRole::Source) {
   653	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   654	        }
   655	        if let Err(fault) = source_open_validator(open) {
   656	            eyre::bail!("run_source initiator config unsupported: {fault}");
   657	        }
   658	    }
   659	
   660	    let negotiated = establish(
   661	        &mut transport,
   662	        &cfg.hello,
   663	        &cfg.endpoint,
   664	        TransferRole::Source,
   665	        &source_open_validator,
   666	        // run_source only ever resolves nothing: a SOURCE *initiator*
   667	        // owns its own root, and a SOURCE *responder* driven directly
   668	        // (the in-process role suite) is handed a Fixed source. The
   669	        // daemon SOURCE responder resolves module→root inside
   670	        // `run_responder`, not here (otp-5).
   671	        None,
   672	    )
   673	    .await?;
   674	
   675	    drive_source(
   676	        cfg.plan_options,
   677	        cfg.data_plane_host,
   678	        &negotiated,
   679	        transport,
   680	        source,
   681	    )
   682	    .await
   683	}
   684	
   685	/// The SOURCE session body after establish: spawn the receive half,
   686	/// run the send half, and map a fault to a peer-notified report. Shared
   687	/// by [`run_source`] (initiator or direct-responder) and
   688	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
   689	/// choreography is single-sourced.
   690	async fn drive_source(
   691	    plan_options: PlanOptions,
   692	    data_plane_host: Option<String>,
   693	    negotiated: &Negotiated,
   694	    transport: FrameTransport,
   695	    source: Arc<dyn TransferSource>,
   696	) -> Result<TransferSummary> {
   697	    let (mut tx, rx) = transport.split();
   698	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   699	    // Set by the send half the moment ManifestComplete goes out. On
   700	    // an ordered transport, a NeedComplete arriving while this is
   701	    // still false is provably premature — the peer cannot have
   702	    // received what we have not sent (contract: NeedComplete only
   703	    // after ManifestComplete received + all entries diffed).
   704	    let manifest_sent = Arc::new(AtomicBool::new(false));
   705	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   706	    // AbortOnDrop: an early error return below must abort the receive
   707	    // half instead of leaking it (same rationale as design-2 / w4-1).
   708	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   709	        rx,
   710	        Arc::clone(&sent),
   711	        Arc::clone(&manifest_sent),
   712	        event_tx,
   713	    )));
   714	
   715	    match source_send_half(
   716	        plan_options,
   717	        data_plane_host.as_deref(),
   718	        negotiated,
   719	        &mut tx,
   720	        source,
   721	        sent,
   722	        &manifest_sent,
   723	        event_rx,
   724	    )
   725	    .await
   726	    {
   727	        Ok(summary) => Ok(summary),
   728	        Err(report) => {
   729	            let mut fault = fault_from_report(report);
   730	            if !fault.peer_notified {
   731	                let _ = tx.send(error_frame(&fault)).await;
   732	                fault.peer_notified = true;
   733	            }
   734	            Err(eyre::Report::new(fault))
   735	        }
   736	    }
   737	}
   738	
   739	/// Receive half of the source driver: drains the transport for the
   740	/// whole session so destination sends can never deadlock against a
   741	/// blocked source send, and routes the destination lane to the send
   742	/// half. Terminates on summary, error, close, or violation.
   743	async fn source_recv_half(
   744	    mut rx: Box<dyn FrameRx>,
   745	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   746	    manifest_sent: Arc<AtomicBool>,
   747	    events: mpsc::UnboundedSender<SourceEvent>,
   748	) {
   749	    loop {
   750	        let received = match rx.recv().await {
   751	            Ok(Some(f)) => f,
   752	            Ok(None) => {
   753	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   754	                    "peer closed before TransferSummary",
   755	                )));
   756	                return;
   757	            }
   758	            Err(err) => {
   759	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
   760	                    "transport receive failed: {err:#}"
   761	                ))));
   762	                return;
   763	            }
   764	        };
   765	        match received.frame {
   766	            Some(Frame::NeedBatch(batch)) => {
   767	                for entry in batch.entries {
   768	                    if entry.resume {
   769	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   770	                            format!(
   771	                                "resume-flagged need for '{}' in a session opened without resume",
   772	                                entry.relative_path
   773	                            ),
   774	                        )));
   775	                        return;
   776	                    }
   777	                    let header = sent
   778	                        .lock()
   779	                        .expect("sent-manifest lock poisoned")
   780	                        .remove(&entry.relative_path);
   781	                    match header {
   782	                        Some(h) => {
   783	                            let _ = events.send(SourceEvent::Need(h));
   784	                        }
   785	                        None => {
   786	                            let _ = events.send(SourceEvent::Fault(
   787	                                SessionFault::protocol_violation(format!(
   788	                                    "need for unknown or already-needed path '{}'",
   789	                                    entry.relative_path
   790	                                )),
   791	                            ));
   792	                            return;
   793	                        }
   794	                    }
   795	                }
   796	            }
   797	            Some(Frame::NeedComplete(_)) => {
   798	                if !manifest_sent.load(Ordering::Acquire) {
   799	                    // Fail fast at arrival time (otp-3 codex F2): the
   800	                    // event queue would otherwise let an early
   801	                    // NeedComplete be processed late and pass as
   802	                    // legitimate.
   803	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   804	                        "NeedComplete before the source's ManifestComplete",
   805	                    )));
   806	                    return;
   807	                }
   808	                let _ = events.send(SourceEvent::NeedComplete);
   809	            }
   810	            Some(Frame::ResizeAck(ack)) => {
   811	                // The destination's response to a shape-resize proposal
   812	                // (otp-4b-2). Forward it to the send half, which owns the
   813	                // dial and dials the epoch-N socket on `accepted`.
   814	                let _ = events.send(SourceEvent::ResizeAck(ack));
   815	            }
   816	            Some(Frame::Summary(summary)) => {
   817	                let _ = events.send(SourceEvent::Summary(summary));
   818	                return;
   819	            }
   820	            Some(Frame::Error(err)) => {
   821	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   822	                return;
   823	            }
   824	            other => {
   825	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   826	                    format!("{} on the source's receive lane", frame_name(&other)),
   827	                )));
   828	                return;
   829	            }
   830	        }
   831	    }
   832	}
   833	
   834	#[allow(clippy::too_many_arguments)]
   835	async fn source_send_half(
   836	    plan_options: PlanOptions,
   837	    data_plane_host: Option<&str>,
   838	    negotiated: &Negotiated,
   839	    tx: &mut Box<dyn FrameTx>,
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

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/session_client.rs | sed -n '1,240p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1360,1565p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer.rs | sed -n '1,190p'" in /home/michael/dev/Blit
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
    13	//! until the otp-10 cutover; today the parity tests drive this. push
    14	//! defaults to the TCP data plane (otp-4b); pull is in-stream only until
    15	//! otp-5b adds the SOURCE-responder data plane.
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
   123	}
   124	
   125	impl Default for PullSessionOptions {
   126	    fn default() -> Self {
   127	        Self {
   128	            compare_mode: ComparisonMode::SizeMtime,
   129	            ignore_existing: false,
   130	            require_complete_scan: false,
   131	        }
   132	    }
   133	}
   134	
   135	/// Connect to `endpoint`'s daemon and run one DESTINATION-role transfer
   136	/// session pulling the endpoint's module/path tree into `dest_root`
   137	/// (pull-equivalent, otp-5a). The client initiates and declares
   138	/// DESTINATION, so the daemon becomes the SOURCE Responder (streaming
   139	/// its module tree). Returns the [`DestinationOutcome`] this end
   140	/// computed (contract: the DESTINATION is the scorer).
   141	///
   142	/// otp-5a rides the in-stream byte carrier: the SOURCE responder grants
   143	/// no TCP data plane yet (the transport/role decoupling that lets a
   144	/// SOURCE responder bind+grant lands at otp-5b), so `in_stream_bytes` is
   145	/// set to make the carrier explicit. Not wired to CLI verbs (otp-10).
   146	pub async fn run_pull_session(
   147	    endpoint: &RemoteEndpoint,
   148	    dest_root: PathBuf,
   149	    options: PullSessionOptions,
   150	) -> Result<DestinationOutcome> {
   151	    let (module, path) = endpoint_module_path(endpoint)?;
   152	
   153	    let mut client = connect_transfer_client(endpoint).await?;
   154	
   155	    let open = SessionOpen {
   156	        initiator_role: TransferRole::Destination as i32,
   157	        module,
   158	        path,
   159	        compare_mode: options.compare_mode as i32,
   160	        ignore_existing: options.ignore_existing,
   161	        require_complete_scan: options.require_complete_scan,
   162	        // otp-5a is in-stream only (the SOURCE responder grants no data
   163	        // plane); set the flag so the carrier is explicit and stable if
   164	        // a data-plane grant is added at otp-5b.
   165	        in_stream_bytes: true,
   166	        ..Default::default()
   167	    };
   168	
   169	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   170	    let inbound = client
   171	        .transfer(ReceiverStream::new(out_rx))
   172	        .await
   173	        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
   174	        .into_inner();
   175	    let transport = grpc_client_transport(out_tx, inbound);
   176	
   177	    let cfg = DestinationSessionConfig {
   178	        hello: HelloConfig::default(),
   179	        endpoint: SessionEndpoint::initiator(open),
   180	    };
   181	    run_destination(cfg, transport, DestinationTarget::Fixed(dest_root)).await
   182	}
   183	
   184	/// Derive the wire `(module, path)` from a resolved endpoint. Empty
   185	/// module targets the daemon's default root export; a discovery-form
   186	/// endpoint is not resolvable to a transfer target.
   187	fn endpoint_module_path(endpoint: &RemoteEndpoint) -> Result<(String, String)> {
   188	    match &endpoint.path {
   189	        RemotePath::Module { module, rel_path } => {
   190	            Ok((module.clone(), rel_path.to_string_lossy().into_owned()))
   191	        }
   192	        RemotePath::Root { rel_path } => {
   193	            Ok((String::new(), rel_path.to_string_lossy().into_owned()))
   194	        }
   195	        RemotePath::Discovery => Err(eyre!(
   196	            "a transfer session needs a resolved module or root endpoint, not a discovery form"
   197	        )),
   198	    }
   199	}
   200	
   201	/// Build a `BlitClient` over `endpoint`'s control-plane URI with the
   202	/// same bounded-connect policy `RemotePushClient::connect` uses.
   203	async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
   204	    let uri = endpoint.control_plane_uri();
   205	    let conn = Endpoint::from_shared(uri.clone())
   206	        .map_err(|e| eyre!("invalid endpoint uri {uri}: {e}"))?
   207	        .connect_timeout(Duration::from_secs(30));
   208	    let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
   209	        .await
   210	        .map_err(|_| eyre!("timed out connecting to {uri}"))?
   211	        .map_err(|e| eyre!("connecting to {uri}: {e}"))?;
   212	    Ok(BlitClient::new(channel))
   213	}

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

 succeeded in 0ms:
  1360	                })))
  1361	                .await?;
  1362	                for chunk in data.chunks(IN_STREAM_CHUNK) {
  1363	                    tx.send(frame(Frame::TarShardChunk(
  1364	                        crate::generated::TarShardChunk {
  1365	                            content: chunk.to_vec(),
  1366	                        },
  1367	                    )))
  1368	                    .await?;
  1369	                }
  1370	                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
  1371	                    .await?;
  1372	            }
  1373	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
  1374	                // The outbound planner never emits these (resume is
  1375	                // receive-originated and lands at otp-7).
  1376	                eyre::bail!("resume payload planned in a non-resume session");
  1377	            }
  1378	        }
  1379	    }
  1380	    Ok(())
  1381	}
  1382	
  1383	// ---------------------------------------------------------------------------
  1384	// DESTINATION driver
  1385	// ---------------------------------------------------------------------------
  1386	
  1387	/// What the destination end can report after a completed session.
  1388	#[derive(Debug, Clone)]
  1389	pub struct DestinationOutcome {
  1390	    /// The summary this end computed and sent (contract: DESTINATION
  1391	    /// is the scorer).
  1392	    pub summary: TransferSummary,
  1393	    /// Paths this end put on the need list, in emission order. The
  1394	    /// role suite pins these identical across role assignments — the
  1395	    /// executable form of the owner's invariance requirement.
  1396	    pub needed_paths: Vec<String>,
  1397	    /// The settled data-plane stream count this end observed (epoch-0 +
  1398	    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
  1399	    /// pin (otp-4b-2) reads it to assert shape correction grew the
  1400	    /// stream set past the zero-knowledge single-stream grant.
  1401	    pub data_plane_streams: Option<usize>,
  1402	}
  1403	
  1404	/// Run the DESTINATION role of one transfer session over `transport`,
  1405	/// writing under the root named by `target`. Diffs the streamed
  1406	/// manifest against its own filesystem (the destination is the one
  1407	/// diff owner — plan §Design 3), returns the summary it computed and
  1408	/// sent.
  1409	///
  1410	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  1411	/// front (an Initiator's own local root, or a test), or
  1412	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  1413	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  1414	/// where the wire module name selects the root).
  1415	pub async fn run_destination(
  1416	    cfg: DestinationSessionConfig,
  1417	    transport: FrameTransport,
  1418	    target: DestinationTarget,
  1419	) -> Result<DestinationOutcome> {
  1420	    let mut transport = transport;
  1421	    let endpoint = match cfg.endpoint {
  1422	        SessionEndpoint::Initiator { mut open } => {
  1423	            let declared = TransferRole::try_from(open.initiator_role);
  1424	            if declared != Ok(TransferRole::Destination) {
  1425	                eyre::bail!(
  1426	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  1427	                );
  1428	            }
  1429	            if let Err(fault) = destination_open_validator(&open) {
  1430	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  1431	            }
  1432	            // Dial contract: the byte receiver advertises capacity in
  1433	            // its open when it is the initiator (contract §Invariants 5).
  1434	            if open.receiver_capacity.is_none() {
  1435	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
  1436	            }
  1437	            SessionEndpoint::Initiator { open }
  1438	        }
  1439	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  1440	    };
  1441	
  1442	    let resolve_open: Option<&OpenResolver> = match &target {
  1443	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1444	        DestinationTarget::Fixed(_) => None,
  1445	    };
  1446	
  1447	    let negotiated = establish(
  1448	        &mut transport,
  1449	        &cfg.hello,
  1450	        &endpoint,
  1451	        TransferRole::Destination,
  1452	        &destination_open_validator,
  1453	        resolve_open,
  1454	    )
  1455	    .await?;
  1456	
  1457	    // The resolver's root (Responder + Resolve) wins; otherwise the
  1458	    // caller-supplied Fixed root.
  1459	    let dst_root = match negotiated.resolved_root.clone() {
  1460	        Some(root) => root,
  1461	        None => match &target {
  1462	            DestinationTarget::Fixed(root) => root.clone(),
  1463	            // Unreachable: a Resolve target always yields a root on the
  1464	            // Responder branch, and establish only skips resolution on
  1465	            // the Initiator branch (which pairs with a Fixed root).
  1466	            DestinationTarget::Resolve(_) => {
  1467	                return Err(eyre::Report::new(SessionFault::internal(
  1468	                    "resolver target produced no destination root",
  1469	                )));
  1470	            }
  1471	        },
  1472	    };
  1473	
  1474	    drive_destination(&mut transport, negotiated, &dst_root).await
  1475	}
  1476	
  1477	/// The DESTINATION session body: run the diff/receive loop and map a
  1478	/// fault to a peer-notified report. Shared by [`run_destination`] and
  1479	/// [`run_responder`] (the daemon DESTINATION responder), so the receive
  1480	/// choreography is single-sourced.
  1481	async fn drive_destination(
  1482	    transport: &mut FrameTransport,
  1483	    negotiated: Negotiated,
  1484	    dst_root: &Path,
  1485	) -> Result<DestinationOutcome> {
  1486	    match destination_session(transport, negotiated, dst_root).await {
  1487	        Ok(outcome) => Ok(outcome),
  1488	        Err(report) => {
  1489	            let mut fault = fault_from_report(report);
  1490	            if !fault.peer_notified {
  1491	                let _ = transport.send(error_frame(&fault)).await;
  1492	                fault.peer_notified = true;
  1493	            }
  1494	            Err(eyre::Report::new(fault))
  1495	        }
  1496	    }
  1497	}
  1498	
  1499	/// Serve one transfer session as the RESPONDER, dispatching on the
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

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer_session_e2e.rs | sed -n '520,700p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   520	    let daemon = Daemon::start(false).await;
   521	
   522	    // Seed the destination with a NEWER, same-size, different-content
   523	    // file plus a file that genuinely needs updating.
   524	    write_tree(
   525	        &daemon.dest_root,
   526	        &[
   527	            ("keep.txt", b"NEWER-destination", 1_600_100_000),
   528	            ("stale.txt", b"old-destination--", 1_600_000_000),
   529	        ],
   530	    );
   531	    let src = tempfile::tempdir().unwrap();
   532	    write_tree(
   533	        src.path(),
   534	        &[
   535	            // same size (17) as dest keep.txt, but OLDER → must be skipped.
   536	            ("keep.txt", b"older-source-here", 1_600_000_000),
   537	            // same size (17) as dest stale.txt, but NEWER → must transfer.
   538	            ("stale.txt", b"new-source-here--", 1_600_200_000),
   539	        ],
   540	    );
   541	
   542	    let summary = run_push_session(
   543	        &daemon.endpoint,
   544	        Arc::new(FsTransferSource::new(src.path().to_path_buf())),
   545	        PushSessionOptions::default(),
   546	    )
   547	    .await
   548	    .expect("session push succeeds");
   549	
   550	    // Only stale.txt transfers; keep.txt (newer on dest) is left intact.
   551	    assert_eq!(
   552	        summary.files_transferred, 1,
   553	        "only the stale file transfers"
   554	    );
   555	    assert_eq!(
   556	        std::fs::read(daemon.dest_root.join("keep.txt")).unwrap(),
   557	        b"NEWER-destination",
   558	        "a newer same-size destination file must NOT be clobbered"
   559	    );
   560	    assert_eq!(
   561	        std::fs::read(daemon.dest_root.join("stale.txt")).unwrap(),
   562	        b"new-source-here--",
   563	        "a stale destination file must be updated"
   564	    );
   565	    daemon.stop().await;
   566	}
   567	
   568	// ---------------------------------------------------------------------------
   569	// otp-5a: pull-equivalent (client initiates as DESTINATION, daemon is SOURCE)
   570	// ---------------------------------------------------------------------------
   571	
   572	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   573	async fn pull_session_lands_bytes_and_scores_them() {
   574	    // Roles flipped: the daemon's MODULE tree is the SOURCE; the client
   575	    // initiates as DESTINATION and the daemon streams its module tree
   576	    // (otp-5a). The SOURCE responder grants no data plane, so the carrier
   577	    // is the in-stream fallback. `dest_root` here is the module (source)
   578	    // root — the harness field name is push-oriented.
   579	    let daemon = Daemon::start(false).await;
   580	    write_tree(&daemon.dest_root, &small_tree());
   581	
   582	    let dest = tempfile::tempdir().unwrap();
   583	    let outcome = run_pull_session(
   584	        &daemon.endpoint,
   585	        dest.path().to_path_buf(),
   586	        PullSessionOptions::default(),
   587	    )
   588	    .await
   589	    .expect("session pull succeeds");
   590	
   591	    assert_eq!(outcome.summary.files_transferred, small_tree().len() as u64);
   592	    assert_eq!(
   593	        outcome.summary.bytes_transferred,
   594	        small_tree()
   595	            .iter()
   596	            .map(|(_, c, _)| c.len() as u64)
   597	            .sum::<u64>()
   598	    );
   599	    assert!(
   600	        outcome.summary.in_stream_carrier_used,
   601	        "otp-5a pull rides the in-stream carrier (no SOURCE-responder data plane yet)"
   602	    );
   603	    assert_trees_identical(&daemon.dest_root, dest.path());
   604	    daemon.stop().await;
   605	}
   606	
   607	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   608	async fn old_pull_and_session_produce_identical_trees_and_counts() {
   609	    // Arm A: OLD pull_sync into a client-local dest.
   610	    let daemon_a = Daemon::start(false).await;
   611	    write_tree(&daemon_a.dest_root, &small_tree());
   612	    let dest_a = tempfile::tempdir().unwrap();
   613	    let mut pull_client = RemotePullClient::connect(daemon_a.endpoint.clone())
   614	        .await
   615	        .expect("old pull client connects");
   616	    let report = pull_client
   617	        .pull_sync(
   618	            dest_a.path(),
   619	            Vec::new(),
   620	            &PullSyncOptions::default(),
   621	            false,
   622	            None,
   623	        )
   624	        .await
   625	        .expect("old pull succeeds");
   626	
   627	    // Arm B: NEW session (client DESTINATION initiator).
   628	    let daemon_b = Daemon::start(false).await;
   629	    write_tree(&daemon_b.dest_root, &small_tree());
   630	    let dest_b = tempfile::tempdir().unwrap();
   631	    let outcome = run_pull_session(
   632	        &daemon_b.endpoint,
   633	        dest_b.path().to_path_buf(),
   634	        PullSessionOptions::default(),
   635	    )
   636	    .await
   637	    .expect("session pull succeeds");
   638	
   639	    // Both dests equal their source module and each other.
   640	    assert_trees_identical(&daemon_a.dest_root, dest_a.path());
   641	    assert_trees_identical(&daemon_b.dest_root, dest_b.path());
   642	    assert_trees_identical(dest_a.path(), dest_b.path());
   643	
   644	    // Shared counters agree (transport-specific fields have no cross
   645	    // analog and are not compared). Old pull already SKIPs the same-size
   646	    // dest-NEWER cell, so this A/B is byte-identical with no caveat —
   647	    // unlike the push A/B where old push clobbers.
   648	    assert_eq!(
   649	        report.files_transferred as u64,
   650	        outcome.summary.files_transferred
   651	    );
   652	    assert_eq!(report.bytes_transferred, outcome.summary.bytes_transferred);
   653	
   654	    daemon_a.stop().await;
   655	    daemon_b.stop().await;
   656	}
   657	
   658	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   659	async fn unknown_module_refuses_the_pull_session() {
   660	    let daemon = Daemon::start(false).await;
   661	    let dest = tempfile::tempdir().unwrap();
   662	    let err = run_pull_session(
   663	        &daemon.endpoint_for_missing_module(),
   664	        dest.path().to_path_buf(),
   665	        PullSessionOptions::default(),
   666	    )
   667	    .await
   668	    .expect_err("unknown module must refuse the pull session");
   669	    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
   670	    daemon.stop().await;
   671	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1560,1635p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1010,1230p'" in /home/michael/dev/Blit
 succeeded in 0ms:
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

 succeeded in 0ms:
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
  1096	
  1097	/// Handle one source event. Needs accumulate into `pending` and the
  1098	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1099	/// next ADD (the one-per-epoch ramp).
  1100	#[allow(clippy::too_many_arguments)]
  1101	async fn process_source_event(
  1102	    event: SourceEvent,
  1103	    pending: &mut Vec<FileHeader>,
  1104	    need_complete: &mut bool,
  1105	    needed_bytes: &mut u64,
  1106	    needed_count: &mut usize,
  1107	    data_plane: Option<&data_plane::SourceDataPlane>,
  1108	    tx: &mut Box<dyn FrameTx>,
  1109	    pending_resize: &mut Option<data_plane::PendingResize>,
  1110	) -> Result<()> {
  1111	    match event {
  1112	        SourceEvent::Need(header) => {
  1113	            if *need_complete {
  1114	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1115	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1116	                )));
  1117	            }
  1118	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1119	            *needed_count += 1;
  1120	            pending.push(header);
  1121	            Ok(())
  1122	        }
  1123	        SourceEvent::NeedComplete => {
  1124	            if *need_complete {
  1125	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1126	                    "duplicate NeedComplete",
  1127	                )));
  1128	            }
  1129	            *need_complete = true;
  1130	            Ok(())
  1131	        }
  1132	        SourceEvent::ResizeAck(ack) => {
  1133	            let dp = data_plane.ok_or_else(|| {
  1134	                eyre::Report::new(SessionFault::protocol_violation(
  1135	                    "DataPlaneResizeAck on a session with no data plane",
  1136	                ))
  1137	            })?;
  1138	            // Match the ack to the in-flight proposal; stale/unsolicited
  1139	            // acks (wrong epoch, or none pending) are ignored, matching
  1140	            // old push. `take()` + restore keeps the borrow simple.
  1141	            let pending_r = match pending_resize.take() {
  1142	                Some(p) if p.epoch == ack.epoch => p,
  1143	                restored => {
  1144	                    *pending_resize = restored;
  1145	                    return Ok(());
  1146	                }
  1147	            };
  1148	            if ack.accepted {
  1149	                dp.add_stream(&pending_r.sub_token).await?;
  1150	                dp.dial()
  1151	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1152	            } else {
  1153	                dp.dial()
  1154	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1155	            }
  1156	            // Ramp one stream per accepted epoch: propose the next ADD.
  1157	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1158	        }
  1159	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1160	            "TransferSummary before SourceDone",
  1161	        ))),
  1162	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1163	    }
  1164	}
  1165	
  1166	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1167	/// the stream count the accumulated need list implies, if none is in
  1168	/// flight. A no-op when the shape wants no more than the live count (the
  1169	/// dial returns `None`). Sends the frame and records the in-flight
  1170	/// proposal for the ack to match.
  1171	async fn maybe_propose_resize(
  1172	    dp: &data_plane::SourceDataPlane,
  1173	    tx: &mut Box<dyn FrameTx>,
  1174	    needed_bytes: u64,
  1175	    needed_count: usize,
  1176	    pending_resize: &mut Option<data_plane::PendingResize>,
  1177	) -> Result<()> {
  1178	    if pending_resize.is_some() {
  1179	        return Ok(());
  1180	    }
  1181	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1182	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1183	            op: DataPlaneResizeOp::Add as i32,
  1184	            epoch: proposal.epoch,
  1185	            target_stream_count: proposal.target_streams,
  1186	            sub_token: proposal.sub_token.clone(),
  1187	        })))
  1188	        .await?;
  1189	        *pending_resize = Some(proposal);
  1190	    }
  1191	    Ok(())
  1192	}
  1193	
  1194	/// Block for the ack of the one in-flight resize and dial its socket (or
  1195	/// settle it refused). Does NOT propose further — it resolves exactly the
  1196	/// pending proposal so the destination's armed slot is consumed before we
  1197	/// finish the data plane.
  1198	async fn resolve_in_flight_resize(
  1199	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1200	    dp: &data_plane::SourceDataPlane,
  1201	    pending: data_plane::PendingResize,
  1202	) -> Result<()> {
  1203	    loop {
  1204	        match events.recv().await {
  1205	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1206	                if ack.accepted {
  1207	                    dp.add_stream(&pending.sub_token).await?;
  1208	                    dp.dial()
  1209	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1210	                } else {
  1211	                    dp.dial()
  1212	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1213	                }
  1214	                return Ok(());
  1215	            }
  1216	            // A stale ack for an already-settled epoch: ignore, keep
  1217	            // waiting for ours.
  1218	            Some(SourceEvent::ResizeAck(_)) => continue,
  1219	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1220	            Some(SourceEvent::Need(h)) => {
  1221	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1222	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1223	                )))
  1224	            }
  1225	            Some(SourceEvent::NeedComplete) => {
  1226	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1227	                    "duplicate NeedComplete",
  1228	                )))
  1229	            }
  1230	            Some(SourceEvent::Summary(_)) => {

exec
/usr/bin/zsh -lc 'git show --name-only --format= 84be1cc' in /home/michael/dev/Blit
 succeeded in 0ms:
.review/findings/otp-5-daemon-as-source.md
crates/blit-core/src/remote/transfer/session_client.rs
crates/blit-core/src/transfer_session/mod.rs
crates/blit-daemon/src/service/transfer.rs
crates/blit-daemon/src/service/transfer_session_e2e.rs

exec
/usr/bin/zsh -lc 'rg -n "receiver_capacity|DataPlane|in_stream_carrier|data_plane_streams|NeedComplete|peer_notified|fault|CANCELLED|cancel" crates/blit-core/src/transfer_session crates/blit-core/tests crates/blit-daemon/src/service/transfer_session_e2e.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer_session_e2e.rs:3://! over gRPC. otp-4b makes the default carrier the **TCP data plane**
crates/blit-daemon/src/service/transfer_session_e2e.rs:15://!   `SessionError` frames, surfaced to the client as faults;
crates/blit-daemon/src/service/transfer_session_e2e.rs:202:fn fault_of(err: &eyre::Report) -> &SessionFault {
crates/blit-daemon/src/service/transfer_session_e2e.rs:207:// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
crates/blit-daemon/src/service/transfer_session_e2e.rs:214:/// cancel under test or the reader being dropped when the session aborts.
crates/blit-daemon/src/service/transfer_session_e2e.rs:271:            // aborted (which drops this task) or cancelled.
crates/blit-daemon/src/service/transfer_session_e2e.rs:283:/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
crates/blit-daemon/src/service/transfer_session_e2e.rs:286:/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
crates/blit-daemon/src/service/transfer_session_e2e.rs:291:async fn mid_transfer_cancel_surfaces_cancelled_over_the_data_plane() {
crates/blit-daemon/src/service/transfer_session_e2e.rs:307:            async move { run_push_session(&ep, source, PushSessionOptions::default()).await },
crates/blit-daemon/src/service/transfer_session_e2e.rs:314:        .expect("payload bytes should flow over the data plane before cancel");
crates/blit-daemon/src/service/transfer_session_e2e.rs:316:    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
crates/blit-daemon/src/service/transfer_session_e2e.rs:317:    // handler does via `cancel_authorized` (audit-9). The RPC-level
crates/blit-daemon/src/service/transfer_session_e2e.rs:328:        daemon.active_jobs.cancel(&transfer_id),
crates/blit-daemon/src/service/transfer_session_e2e.rs:330:        "the served session's row honors cancellation"
crates/blit-daemon/src/service/transfer_session_e2e.rs:333:    // The client must surface CANCELLED promptly (no hang).
crates/blit-daemon/src/service/transfer_session_e2e.rs:336:        .expect("client must not hang on a mid-transfer cancel")
crates/blit-daemon/src/service/transfer_session_e2e.rs:338:    let err = result.expect_err("a cancelled transfer fails");
crates/blit-daemon/src/service/transfer_session_e2e.rs:340:        fault_of(&err).code,
crates/blit-daemon/src/service/transfer_session_e2e.rs:342:        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
crates/blit-daemon/src/service/transfer_session_e2e.rs:356:        "the daemon must drain the cancelled job from active[]"
crates/blit-daemon/src/service/transfer_session_e2e.rs:372:    // Default options ⇒ TCP data plane: the responder grants it and the
crates/blit-daemon/src/service/transfer_session_e2e.rs:375:    let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
crates/blit-daemon/src/service/transfer_session_e2e.rs:388:        !summary.in_stream_carrier_used,
crates/blit-daemon/src/service/transfer_session_e2e.rs:389:        "otp-4b default rides the TCP data plane, not the in-stream carrier"
crates/blit-daemon/src/service/transfer_session_e2e.rs:396:async fn session_lands_bytes_over_in_stream_carrier() {
crates/blit-daemon/src/service/transfer_session_e2e.rs:410:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:418:        summary.in_stream_carrier_used,
crates/blit-daemon/src/service/transfer_session_e2e.rs:438:            &FileFilter::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:454:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:465:    // tcp_fallback_used/bytes_zero_copy vs in_stream_carrier_used — have
crates/blit-daemon/src/service/transfer_session_e2e.rs:484:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:488:    assert_eq!(fault_of(&err).code, session_error::Code::ReadOnly);
crates/blit-daemon/src/service/transfer_session_e2e.rs:505:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:509:    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
crates/blit-daemon/src/service/transfer_session_e2e.rs:545:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:586:        PullSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:600:        outcome.summary.in_stream_carrier_used,
crates/blit-daemon/src/service/transfer_session_e2e.rs:620:            &PullSyncOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:634:        PullSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:665:        PullSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:669:    assert_eq!(fault_of(&err).code, session_error::Code::ModuleUnknown);
crates/blit-core/src/transfer_session/mod.rs:12://! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
crates/blit-core/src/transfer_session/mod.rs:33:    session_error, ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp,
crates/blit-core/src/transfer_session/mod.rs:34:    FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch, NeedComplete, NeedEntry,
crates/blit-core/src/transfer_session/mod.rs:76:/// The identity this end presents in `SessionHello`. Defaults to the
crates/blit-core/src/transfer_session/mod.rs:84:impl Default for HelloConfig {
crates/blit-core/src/transfer_session/mod.rs:85:    fn default() -> Self {
crates/blit-core/src/transfer_session/mod.rs:125:    /// data plane at this end — a grant then faults, since the responder
crates/blit-core/src/transfer_session/mod.rs:135:/// A session-terminating fault: either end refusing, aborting, or
crates/blit-core/src/transfer_session/mod.rs:147:    /// True when the peer already knows about this fault — it sent
crates/blit-core/src/transfer_session/mod.rs:150:    pub peer_notified: bool,
crates/blit-core/src/transfer_session/mod.rs:160:            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs:192:            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs:214:/// Downcast a driver-internal error back to its fault, wrapping
crates/blit-core/src/transfer_session/mod.rs:215:/// non-fault failures (fs errors, planner errors, transport failures)
crates/blit-core/src/transfer_session/mod.rs:217:fn fault_from_report(report: eyre::Report) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:219:        Ok(fault) => fault,
crates/blit-core/src/transfer_session/mod.rs:228:fn error_frame(fault: &SessionFault) -> TransferFrame {
crates/blit-core/src/transfer_session/mod.rs:229:    frame(Frame::Error(fault.to_wire()))
crates/blit-core/src/transfer_session/mod.rs:241:        Some(Frame::NeedComplete(_)) => "NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:250:        Some(Frame::Resize(_)) => "DataPlaneResize",
crates/blit-core/src/transfer_session/mod.rs:251:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
crates/blit-core/src/transfer_session/mod.rs:269:/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
crates/blit-core/src/transfer_session/mod.rs:306:/// of module/Status types. A returned fault (unknown module,
crates/blit-core/src/transfer_session/mod.rs:362:        .is_some_and(|f| *f != FilterSpec::default())
crates/blit-core/src/transfer_session/mod.rs:399:    responder_data_plane: Option<data_plane::ResponderDataPlane>,
crates/blit-core/src/transfer_session/mod.rs:431:        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:440:            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs:442:        return Err(notify_and_wrap(transport, fault).await);
crates/blit-core/src/transfer_session/mod.rs:453:/// `SessionError` itself; returned faults are `peer_notified`.
crates/blit-core/src/transfer_session/mod.rs:475:    if let Err(fault) = validate_open(&open) {
crates/blit-core/src/transfer_session/mod.rs:478:        return Err(notify_and_wrap(transport, fault).await);
crates/blit-core/src/transfer_session/mod.rs:501:            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
crates/blit-core/src/transfer_session/mod.rs:520:        receiver_capacity: if local_role == TransferRole::Destination {
crates/blit-core/src/transfer_session/mod.rs:521:            Some(crate::engine::local_receiver_capacity())
crates/blit-core/src/transfer_session/mod.rs:539:/// it detects the fault locally; returned faults are `peer_notified`.
crates/blit-core/src/transfer_session/mod.rs:597:/// terminal faults.
crates/blit-core/src/transfer_session/mod.rs:613:/// Send the fault to the peer (best effort), mark it notified, and
crates/blit-core/src/transfer_session/mod.rs:615:async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
crates/blit-core/src/transfer_session/mod.rs:616:    let _ = transport.send(error_frame(&fault)).await;
crates/blit-core/src/transfer_session/mod.rs:617:    fault.peer_notified = true;
crates/blit-core/src/transfer_session/mod.rs:618:    eyre::Report::new(fault)
crates/blit-core/src/transfer_session/mod.rs:628:/// fault the session), so the queue never exceeds the source's own
crates/blit-core/src/transfer_session/mod.rs:632:    NeedComplete,
crates/blit-core/src/transfer_session/mod.rs:633:    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
crates/blit-core/src/transfer_session/mod.rs:635:    ResizeAck(DataPlaneResizeAck),
crates/blit-core/src/transfer_session/mod.rs:655:        if let Err(fault) = source_open_validator(open) {
crates/blit-core/src/transfer_session/mod.rs:656:            eyre::bail!("run_source initiator config unsupported: {fault}");
crates/blit-core/src/transfer_session/mod.rs:686:/// run the send half, and map a fault to a peer-notified report. Shared
crates/blit-core/src/transfer_session/mod.rs:698:    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
crates/blit-core/src/transfer_session/mod.rs:700:    // an ordered transport, a NeedComplete arriving while this is
crates/blit-core/src/transfer_session/mod.rs:702:    // received what we have not sent (contract: NeedComplete only
crates/blit-core/src/transfer_session/mod.rs:729:            let mut fault = fault_from_report(report);
crates/blit-core/src/transfer_session/mod.rs:730:            if !fault.peer_notified {
crates/blit-core/src/transfer_session/mod.rs:731:                let _ = tx.send(error_frame(&fault)).await;
crates/blit-core/src/transfer_session/mod.rs:732:                fault.peer_notified = true;
crates/blit-core/src/transfer_session/mod.rs:734:            Err(eyre::Report::new(fault))
crates/blit-core/src/transfer_session/mod.rs:797:            Some(Frame::NeedComplete(_)) => {
crates/blit-core/src/transfer_session/mod.rs:801:                    // NeedComplete be processed late and pass as
crates/blit-core/src/transfer_session/mod.rs:804:                        "NeedComplete before the source's ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:808:                let _ = events.send(SourceEvent::NeedComplete);
crates/blit-core/src/transfer_session/mod.rs:866:                    negotiated.accept.receiver_capacity.as_ref(),
crates/blit-core/src/transfer_session/mod.rs:888:    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
crates/blit-core/src/transfer_session/mod.rs:960:                    // A cancel while earlier batches are actively moving
crates/blit-core/src/transfer_session/mod.rs:963:                    // peer's framed reason (CANCELLED) the same way the
crates/blit-core/src/transfer_session/mod.rs:966:                    // arrive here, and `recv_peer_fault` would consume them.
crates/blit-core/src/transfer_session/mod.rs:968:                        return Err(prefer_peer_fault(&mut events, dp_err).await);
crates/blit-core/src/transfer_session/mod.rs:996:                    "source receive half ended before NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1018:    // mid-transfer cancel almost always lands here. Race it against a
crates/blit-core/src/transfer_session/mod.rs:1019:    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
crates/blit-core/src/transfer_session/mod.rs:1020:    // the served session frames `SessionError{CANCELLED}`, and the source
crates/blit-core/src/transfer_session/mod.rs:1023:    //   * fault arrives while the drain is still pending (e.g. a worker
crates/blit-core/src/transfer_session/mod.rs:1025:    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
crates/blit-core/src/transfer_session/mod.rs:1030:    //     stall window (`prefer_peer_fault`).
crates/blit-core/src/transfer_session/mod.rs:1034:            fault = recv_peer_fault(&mut events) => {
crates/blit-core/src/transfer_session/mod.rs:1035:                return Err(eyre::Report::new(fault));
crates/blit-core/src/transfer_session/mod.rs:1039:                    return Err(prefer_peer_fault(&mut events, dp_err).await);
crates/blit-core/src/transfer_session/mod.rs:1051:        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1053:            format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1055:        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1056:            SessionFault::protocol_violation("duplicate NeedComplete"),
crates/blit-core/src/transfer_session/mod.rs:1059:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
crates/blit-core/src/transfer_session/mod.rs:1077:    data_plane: Option<&data_plane::SourceDataPlane>,
crates/blit-core/src/transfer_session/mod.rs:1107:    data_plane: Option<&data_plane::SourceDataPlane>,
crates/blit-core/src/transfer_session/mod.rs:1115:                    format!("need for '{}' after NeedComplete", header.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1123:        SourceEvent::NeedComplete => {
crates/blit-core/src/transfer_session/mod.rs:1126:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1135:                    "DataPlaneResizeAck on a session with no data plane",
crates/blit-core/src/transfer_session/mod.rs:1162:        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1166:/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
crates/blit-core/src/transfer_session/mod.rs:1172:    dp: &data_plane::SourceDataPlane,
crates/blit-core/src/transfer_session/mod.rs:1182:        tx.send(frame(Frame::Resize(DataPlaneResize {
crates/blit-core/src/transfer_session/mod.rs:1183:            op: DataPlaneResizeOp::Add as i32,
crates/blit-core/src/transfer_session/mod.rs:1200:    dp: &data_plane::SourceDataPlane,
crates/blit-core/src/transfer_session/mod.rs:1219:            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1222:                    format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1225:            Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1227:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1246:/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
crates/blit-core/src/transfer_session/mod.rs:1247:/// abort the send and surface as the fault.
crates/blit-core/src/transfer_session/mod.rs:1251:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
crates/blit-core/src/transfer_session/mod.rs:1258:async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1260:        Some(SourceEvent::Fault(fault)) => fault,
crates/blit-core/src/transfer_session/mod.rs:1262:            "need for '{}' during the data-plane drain (after NeedComplete)",
crates/blit-core/src/transfer_session/mod.rs:1265:        Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1266:            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
crates/blit-core/src/transfer_session/mod.rs:1269:            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
crates/blit-core/src/transfer_session/mod.rs:1282:/// lane. Prefer that framed fault; fall back to the raw data-plane error
crates/blit-core/src/transfer_session/mod.rs:1285:/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
crates/blit-core/src/transfer_session/mod.rs:1288:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
crates/blit-core/src/transfer_session/mod.rs:1290:/// SKIPS non-fault events rather than treating them as violations: we are
crates/blit-core/src/transfer_session/mod.rs:1291:/// already unwinding on a data-plane error, and the framed fault (or the
crates/blit-core/src/transfer_session/mod.rs:1293:async fn prefer_peer_fault(
crates/blit-core/src/transfer_session/mod.rs:1300:                Some(SourceEvent::Fault(fault)) => break Some(fault),
crates/blit-core/src/transfer_session/mod.rs:1305:                // Receive half ended without framing a fault → the raw
crates/blit-core/src/transfer_session/mod.rs:1312:        Ok(Some(fault)) => eyre::Report::new(fault),
crates/blit-core/src/transfer_session/mod.rs:1401:    pub data_plane_streams: Option<usize>,
crates/blit-core/src/transfer_session/mod.rs:1429:            if let Err(fault) = destination_open_validator(&open) {
crates/blit-core/src/transfer_session/mod.rs:1430:                eyre::bail!("run_destination initiator config unsupported: {fault}");
crates/blit-core/src/transfer_session/mod.rs:1434:            if open.receiver_capacity.is_none() {
crates/blit-core/src/transfer_session/mod.rs:1435:                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
crates/blit-core/src/transfer_session/mod.rs:1478:/// fault to a peer-notified report. Shared by [`run_destination`] and
crates/blit-core/src/transfer_session/mod.rs:1489:            let mut fault = fault_from_report(report);
crates/blit-core/src/transfer_session/mod.rs:1490:            if !fault.peer_notified {
crates/blit-core/src/transfer_session/mod.rs:1491:                let _ = transport.send(error_frame(&fault)).await;
crates/blit-core/src/transfer_session/mod.rs:1492:                fault.peer_notified = true;
crates/blit-core/src/transfer_session/mod.rs:1494:            Err(eyre::Report::new(fault))
crates/blit-core/src/transfer_session/mod.rs:1591:                drive_source(PlanOptions::default(), None, &negotiated, transport, source).await?;
crates/blit-core/src/transfer_session/mod.rs:1660:    // bounds it to this future: a control-lane fault that returns from
crates/blit-core/src/transfer_session/mod.rs:1737:                // NeedComplete only after ManifestComplete received
crates/blit-core/src/transfer_session/mod.rs:1740:                    .send(frame(Frame::NeedComplete(NeedComplete {})))
crates/blit-core/src/transfer_session/mod.rs:1806:                    violation("DataPlaneResize on a session with no data plane".into())
crates/blit-core/src/transfer_session/mod.rs:1808:                let op = DataPlaneResizeOp::try_from(resize.op)
crates/blit-core/src/transfer_session/mod.rs:1809:                    .unwrap_or(DataPlaneResizeOp::Unspecified);
crates/blit-core/src/transfer_session/mod.rs:1810:                if op != DataPlaneResizeOp::Add {
crates/blit-core/src/transfer_session/mod.rs:1818:                        "DataPlaneResize sub_token must be 16 bytes".into(),
crates/blit-core/src/transfer_session/mod.rs:1833:                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
crates/blit-core/src/transfer_session/mod.rs:1854:                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
crates/blit-core/src/transfer_session/mod.rs:1876:                    in_stream_carrier_used,
crates/blit-core/src/transfer_session/mod.rs:1883:                    data_plane_streams,
crates/blit-core/src/transfer_session/mod.rs:2153:    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
crates/blit-core/src/transfer_session/mod.rs:2154:    /// returns THAT fault, not the raw data-plane transport error — the
crates/blit-core/src/transfer_session/mod.rs:2155:    /// non-timeout half of the mid-transfer-cancel guard (the e2e in
crates/blit-core/src/transfer_session/mod.rs:2158:    async fn prefer_peer_fault_prefers_a_framed_fault() {
crates/blit-core/src/transfer_session/mod.rs:2160:        // The peer framed CANCELLED on the control lane before we ask.
crates/blit-core/src/transfer_session/mod.rs:2163:            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:2166:            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs:2168:        .expect("send fault");
crates/blit-core/src/transfer_session/mod.rs:2171:            session_error::Code::DataPlaneFailed,
crates/blit-core/src/transfer_session/mod.rs:2174:        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
crates/blit-core/src/transfer_session/mod.rs:2175:        let fault = chosen
crates/blit-core/src/transfer_session/mod.rs:2179:            fault.code,
crates/blit-core/src/transfer_session/mod.rs:2181:            "the framed CANCELLED must win over the data-plane break"
crates/blit-core/src/transfer_session/mod.rs:2186:    /// legitimate `Need` may be queued ahead of the peer's `CANCELLED`.
crates/blit-core/src/transfer_session/mod.rs:2187:    /// `prefer_peer_fault` must SKIP it and still surface CANCELLED — not
crates/blit-core/src/transfer_session/mod.rs:2189:    /// finish()-drain `recv_peer_fault` would).
crates/blit-core/src/transfer_session/mod.rs:2191:    async fn prefer_peer_fault_skips_inflight_needs_to_reach_the_fault() {
crates/blit-core/src/transfer_session/mod.rs:2196:            ..Default::default()
crates/blit-core/src/transfer_session/mod.rs:2201:            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:2204:            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs:2206:        .expect("send fault");
crates/blit-core/src/transfer_session/mod.rs:2209:            session_error::Code::DataPlaneFailed,
crates/blit-core/src/transfer_session/mod.rs:2212:        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
crates/blit-core/src/transfer_session/mod.rs:2213:        let fault = chosen
crates/blit-core/src/transfer_session/mod.rs:2217:            fault.code,
crates/blit-core/src/transfer_session/mod.rs:2224:    fn fault_round_trips_the_wire_shape() {
crates/blit-core/src/transfer_session/mod.rs:2225:        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:2230:            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs:2232:        let wire = fault.to_wire();
crates/blit-core/src/transfer_session/mod.rs:2239:        assert!(back.peer_notified);
crates/blit-core/src/transfer_session/data_plane.rs:4://! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
crates/blit-core/src/transfer_session/data_plane.rs:21://! table and proposes `DataPlaneResize{ADD}` (one stream per epoch) on
crates/blit-core/src/transfer_session/data_plane.rs:23://! `DataPlaneResizeAck`, and accepts one more socket; the SOURCE dials
crates/blit-core/src/transfer_session/data_plane.rs:40:use crate::engine::{initial_stream_proposal, local_receiver_capacity, TransferDial};
crates/blit-core/src/transfer_session/data_plane.rs:41:use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
crates/blit-core/src/transfer_session/data_plane.rs:44:use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
crates/blit-core/src/transfer_session/data_plane.rs:51:    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
crates/blit-core/src/transfer_session/data_plane.rs:64:fn dp_fault(msg: impl Into<String>) -> eyre::Report {
crates/blit-core/src/transfer_session/data_plane.rs:65:    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
crates/blit-core/src/transfer_session/data_plane.rs:75:pub(super) struct ResponderDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:88:pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
crates/blit-core/src/transfer_session/data_plane.rs:124:    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
crates/blit-core/src/transfer_session/data_plane.rs:126:    Some(ResponderDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:138:/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
crates/blit-core/src/transfer_session/data_plane.rs:147:pub(super) struct ResponderDataPlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:160:impl ResponderDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:161:    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
crates/blit-core/src/transfer_session/data_plane.rs:162:    pub(super) fn grant(&self) -> DataPlaneGrant {
crates/blit-core/src/transfer_session/data_plane.rs:163:        DataPlaneGrant {
crates/blit-core/src/transfer_session/data_plane.rs:184:    pub(super) fn spawn(self, sink: Arc<dyn TransferSink>) -> ResponderDataPlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:185:        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
crates/blit-core/src/transfer_session/data_plane.rs:189:        ResponderDataPlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:207:        let mut total = SinkOutcome::default();
crates/blit-core/src/transfer_session/data_plane.rs:223:        // failure faults the whole session, aborting this task via
crates/blit-core/src/transfer_session/data_plane.rs:258:                // is cancel-safe, so losing this arm to another (its
crates/blit-core/src/transfer_session/data_plane.rs:261:                // select cancel can never truncate a half-read socket.
crates/blit-core/src/transfer_session/data_plane.rs:272:                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
crates/blit-core/src/transfer_session/data_plane.rs:285:impl ResponderDataPlaneRun {
crates/blit-core/src/transfer_session/data_plane.rs:297:        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
crates/blit-core/src/transfer_session/data_plane.rs:302:            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
crates/blit-core/src/transfer_session/data_plane.rs:330:        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
crates/blit-core/src/transfer_session/data_plane.rs:332:            return Err(dp_fault(format!(
crates/blit-core/src/transfer_session/data_plane.rs:338:        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
crates/blit-core/src/transfer_session/data_plane.rs:343:/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
crates/blit-core/src/transfer_session/data_plane.rs:351:        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
crates/blit-core/src/transfer_session/data_plane.rs:353:            return Err(dp_fault(format!(
crates/blit-core/src/transfer_session/data_plane.rs:363:        return Err(dp_fault(
crates/blit-core/src/transfer_session/data_plane.rs:373:/// in the accept loop body (never a select arm), so a select cancel can
crates/blit-core/src/transfer_session/data_plane.rs:386:            return Err(dp_fault(format!(
crates/blit-core/src/transfer_session/data_plane.rs:391:            return Err(dp_fault(format!(
crates/blit-core/src/transfer_session/data_plane.rs:397:        return Err(dp_fault(
crates/blit-core/src/transfer_session/data_plane.rs:407:        None => Err(dp_fault(
crates/blit-core/src/transfer_session/data_plane.rs:419:/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
crates/blit-core/src/transfer_session/data_plane.rs:420:/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
crates/blit-core/src/transfer_session/data_plane.rs:434:pub(super) struct SourceDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:455:/// contract §Transport: the initiator always dials). `receiver_capacity`
crates/blit-core/src/transfer_session/data_plane.rs:461:    grant: &DataPlaneGrant,
crates/blit-core/src/transfer_session/data_plane.rs:462:    receiver_capacity: Option<&CapacityProfile>,
crates/blit-core/src/transfer_session/data_plane.rs:464:) -> Result<SourceDataPlane> {
crates/blit-core/src/transfer_session/data_plane.rs:469:    let dial = TransferDial::conservative_within(receiver_capacity).shared();
crates/blit-core/src/transfer_session/data_plane.rs:485:        let session = DataPlaneSession::connect(
crates/blit-core/src/transfer_session/data_plane.rs:496:        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
crates/blit-core/src/transfer_session/data_plane.rs:499:        sinks.push(Arc::new(DataPlaneSink::new(
crates/blit-core/src/transfer_session/data_plane.rs:510:    // Bounded by AbortOnDrop: a fault on the control lane that drops the
crates/blit-core/src/transfer_session/data_plane.rs:511:    // SourceDataPlane aborts the pipeline task instead of leaking it.
crates/blit-core/src/transfer_session/data_plane.rs:523:    Ok(SourceDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:536:impl SourceDataPlane {
crates/blit-core/src/transfer_session/data_plane.rs:547:    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
crates/blit-core/src/transfer_session/data_plane.rs:561:            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
crates/blit-core/src/transfer_session/data_plane.rs:572:    /// epoch-0 failing an epoch-N dial is a transport fault worth
crates/blit-core/src/transfer_session/data_plane.rs:573:    /// surfacing — and faulting the session aborts the peer's accept loop
crates/blit-core/src/transfer_session/data_plane.rs:582:        let session = DataPlaneSession::connect(
crates/blit-core/src/transfer_session/data_plane.rs:593:        .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?;
crates/blit-core/src/transfer_session/data_plane.rs:594:        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
crates/blit-core/src/transfer_session/data_plane.rs:616:                dp_fault("data-plane send pipeline closed before all payloads sent")
crates/blit-core/src/transfer_session/data_plane.rs:633:            .expect("SourceDataPlane::finish called once");
crates/blit-core/src/transfer_session/data_plane.rs:637:            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
crates/blit-core/src/transfer_session/data_plane.rs:665:    /// Remove `path` from the outstanding set, or fault: a path that is
crates/blit-core/src/transfer_session/data_plane.rs:757:    /// record (non-resume session) all fault; a granted path claims once.
crates/blit-core/src/transfer_session/data_plane.rs:769:                ..Default::default()
crates/blit-core/src/transfer_session/data_plane.rs:773:        // Off-need-list path faults with a SessionFault.
crates/blit-core/src/transfer_session/data_plane.rs:777:            .expect_err("off-need-list path must fault");
crates/blit-core/src/transfer_session/data_plane.rs:783:        // Granted need claims exactly once; a duplicate then faults.
crates/blit-core/src/transfer_session/data_plane.rs:794:            .expect_err("duplicate delivery must fault");
crates/blit-core/src/transfer_session/data_plane.rs:805:            .expect_err("resume block on a non-resume session must fault");
crates/blit-core/tests/predictor_streaming.rs:90:        ..Default::default()
crates/blit-core/tests/predictor_streaming.rs:117:        ..Default::default()
crates/blit-core/tests/enumeration_tests.rs:12:    let filter = FileFilter::default();
crates/blit-core/tests/enumeration_tests.rs:47:    let mut filter = FileFilter::default();
crates/blit-core/tests/enumeration_tests.rs:78:    let filter = FileFilter::default();
crates/blit-core/tests/engine_streaming_plan.rs:191:        ..Default::default()
crates/blit-core/src/transfer_session/transport.rs:123:// faults.
crates/blit-core/src/transfer_session/transport.rs:172:/// terminal transport fault, not by the session.
crates/blit-core/tests/pull_sync_with_spec_wire.rs:178:    async fn cancel_job(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:264:        receiver_capacity: Some(blit_core::generated::CapacityProfile {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:473:    acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>>,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:523:                    if let Some(client_pull_message::Payload::DataPlaneResizeAck(ack)) = msg.payload
crates/blit-core/tests/pull_sync_with_spec_wire.rs:606:    async fn cancel_job(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:624:    spawn_canned_with_acks(captured, frames, Arc::default()).await
crates/blit-core/tests/pull_sync_with_spec_wire.rs:630:    acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>>,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:799:            receiver_capacity: None,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:944:    // A DataPlaneResize on a session whose negotiation did not set
crates/blit-core/tests/pull_sync_with_spec_wire.rs:948:    let acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>> = Arc::default();
crates/blit-core/tests/pull_sync_with_spec_wire.rs:953:        server_pull_message::Payload::DataPlaneResize(blit_core::generated::DataPlaneResize {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:954:            op: blit_core::generated::DataPlaneResizeOp::Add as i32,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1045:    let acks: Arc<Mutex<Vec<blit_core::generated::DataPlaneResizeAck>>> = Arc::default();
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1058:            receiver_capacity: None,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1062:        server_pull_message::Payload::DataPlaneResize(blit_core::generated::DataPlaneResize {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1063:            op: blit_core::generated::DataPlaneResizeOp::Add as i32,
crates/blit-core/tests/local_transfers.rs:58:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:98:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:139:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:172:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:208:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:249:        ..Default::default()
crates/blit-core/tests/local_transfers.rs:298:        ..Default::default()
crates/blit-core/tests/mirror_planner_tests.rs:102:    let filter = fs_enum::FileFilter::default();
crates/blit-core/tests/mirror_planner_tests.rs:122:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:173:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:189:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:213:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:237:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:377:    let enumerator = FileEnumerator::new(FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:442:    let mut filter_local = fs_enum::FileFilter::default();
crates/blit-core/tests/mirror_planner_tests.rs:461:    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
crates/blit-core/tests/mirror_planner_tests.rs:464:    let mut dest_filter = fs_enum::FileFilter::default();
crates/blit-core/tests/mirror_planner_tests.rs:558:    let filter_for_local_del = fs_enum::FileFilter::default();
crates/blit-core/tests/transfer_session_roles.rs:20:    session_error, ComparisonMode, FileHeader, ManifestComplete, NeedBatch, NeedComplete,
crates/blit-core/tests/transfer_session_roles.rs:95:        ..Default::default()
crates/blit-core/tests/transfer_session_roles.rs:120:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:126:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:176:            source_summary.in_stream_carrier_used,
crates/blit-core/tests/transfer_session_roles.rs:199:fn fault_of(err: &eyre::Report) -> &SessionFault {
crates/blit-core/tests/transfer_session_roles.rs:238:    let (summary, needed) = assert_invariant_across_roles(&src, &[], PlanOptions::default()).await;
crates/blit-core/tests/transfer_session_roles.rs:276:        ..PlanOptions::default()
crates/blit-core/tests/transfer_session_roles.rs:297:    let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
crates/blit-core/tests/transfer_session_roles.rs:314:        assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
crates/blit-core/tests/transfer_session_roles.rs:326:    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
crates/blit-core/tests/transfer_session_roles.rs:328:    // settled count is read from the destination's `data_plane_streams`.
crates/blit-core/tests/transfer_session_roles.rs:346:        ..Default::default()
crates/blit-core/tests/transfer_session_roles.rs:349:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:351:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:355:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:372:        !summary.in_stream_carrier_used,
crates/blit-core/tests/transfer_session_roles.rs:377:        .data_plane_streams
crates/blit-core/tests/transfer_session_roles.rs:406:        PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:441:            plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:466:            let fault = fault_of(&err);
crates/blit-core/tests/transfer_session_roles.rs:468:                fault.code,
crates/blit-core/tests/transfer_session_roles.rs:473:                fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
crates/blit-core/tests/transfer_session_roles.rs:475:                fault.message
crates/blit-core/tests/transfer_session_roles.rs:494:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:496:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:501:            build_id: HelloConfig::default().build_id,
crates/blit-core/tests/transfer_session_roles.rs:513:        fault_of(&source_result.unwrap_err()).code,
crates/blit-core/tests/transfer_session_roles.rs:517:        fault_of(&dest_result.err().unwrap()).code,
crates/blit-core/tests/transfer_session_roles.rs:537:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:539:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:543:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:552:    let source_fault = fault_of(&source_result.unwrap_err()).clone();
crates/blit-core/tests/transfer_session_roles.rs:553:    assert_eq!(source_fault.code, session_error::Code::Internal);
crates/blit-core/tests/transfer_session_roles.rs:555:        source_fault.message.contains("otp-6"),
crates/blit-core/tests/transfer_session_roles.rs:557:        source_fault.message
crates/blit-core/tests/transfer_session_roles.rs:580:    let hello = HelloConfig::default();
crates/blit-core/tests/transfer_session_roles.rs:594:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:633:            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs:639:    // ...and its driver must fail with the same fault.
crates/blit-core/tests/transfer_session_roles.rs:642:        fault_of(&dest_err).code,
crates/blit-core/tests/transfer_session_roles.rs:652:async fn need_for_unknown_path_faults_the_source() {
crates/blit-core/tests/transfer_session_roles.rs:659:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:661:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:673:    peer.send(wire(Frame::Accept(Default::default())))
crates/blit-core/tests/transfer_session_roles.rs:693:    let fault = fault_of(&source_err);
crates/blit-core/tests/transfer_session_roles.rs:694:    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs:695:    assert!(fault.message.contains("never-manifested.txt"));
crates/blit-core/tests/transfer_session_roles.rs:713:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:715:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:725:    peer.send(wire(Frame::Accept(Default::default())))
crates/blit-core/tests/transfer_session_roles.rs:746:        fault_of(&source_err).code,
crates/blit-core/tests/transfer_session_roles.rs:752:async fn need_complete_before_manifest_complete_faults_the_source() {
crates/blit-core/tests/transfer_session_roles.rs:753:    // codex otp-3 F2: NeedComplete is only legal after the source's
crates/blit-core/tests/transfer_session_roles.rs:758:    // peer that reads nothing until after its early NeedComplete
crates/blit-core/tests/transfer_session_roles.rs:772:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:774:        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs:784:    peer.send(wire(Frame::Accept(Default::default())))
crates/blit-core/tests/transfer_session_roles.rs:789:    peer.send(wire(Frame::NeedComplete(NeedComplete {})))
crates/blit-core/tests/transfer_session_roles.rs:803:            Frame::SourceDone(_) => panic!("source treated early NeedComplete as legitimate"),
crates/blit-core/tests/transfer_session_roles.rs:810:    let fault = fault_of(&source_err);
crates/blit-core/tests/transfer_session_roles.rs:811:    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs:813:        fault.message.contains("ManifestComplete"),
crates/blit-core/tests/transfer_session_roles.rs:814:        "fault must name the ordering rule, got: {}",
crates/blit-core/tests/transfer_session_roles.rs:815:        fault.message
crates/blit-core/tests/transfer_session_roles.rs:826:        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs:860:        fault_of(&dest_err).code,
crates/blit-core/tests/proto_wire_compat.rs:5://! The slice adds `CapacityProfile receiver_capacity` (negotiation
crates/blit-core/tests/proto_wire_compat.rs:8://! (PushHeader 8 / PeerCapabilities 5), and the `DataPlaneResize`/
crates/blit-core/tests/proto_wire_compat.rs:9://! `DataPlaneResizeAck` oneof variants. REV4's acceptance criterion:
crates/blit-core/tests/proto_wire_compat.rs:19://! proto3 defaults (`None`/`false`/empty) and can treat "absent" as
crates/blit-core/tests/proto_wire_compat.rs:28://! every new client. `receiver_capacity` is a pure optimization hint —
crates/blit-core/tests/proto_wire_compat.rs:34:    client_push_request, server_pull_message, CapacityProfile, ClientPushRequest, DataPlaneResize,
crates/blit-core/tests/proto_wire_compat.rs:35:    DataPlaneResizeAck, DataPlaneResizeOp, DataTransferNegotiation, DrainClass, PeerCapabilities,
crates/blit-core/tests/proto_wire_compat.rs:92:/// `receiver_capacity = 12`), nesting the old capabilities shape.
crates/blit-core/tests/proto_wire_compat.rs:121:/// equivalent). `DataPlaneResize = 16` is unknown to it.
crates/blit-core/tests/proto_wire_compat.rs:136:/// `UploadComplete = 5`. `DataPlaneResize = 9` is unknown to it.
crates/blit-core/tests/proto_wire_compat.rs:151:/// `DataPlaneResizeAck = 5` is unknown to it.
crates/blit-core/tests/proto_wire_compat.rs:166:/// `ManifestDone = 3`. `DataPlaneResizeAck = 5` is unknown to it.
crates/blit-core/tests/proto_wire_compat.rs:213:        receiver_capacity: Some(full_capacity_profile()),
crates/blit-core/tests/proto_wire_compat.rs:239:    assert!(new.receiver_capacity.is_none());
crates/blit-core/tests/proto_wire_compat.rs:269:    assert!(new.receiver_capacity.is_none());
crates/blit-core/tests/proto_wire_compat.rs:307:        receiver_capacity: Some(full_capacity_profile()),
crates/blit-core/tests/proto_wire_compat.rs:320:fn new_spec_with_receiver_capacity_passes_old_daemon_version_gate() {
crates/blit-core/tests/proto_wire_compat.rs:321:    // THE load-bearing compat property: receiver_capacity ships at
crates/blit-core/tests/proto_wire_compat.rs:371:        payload: Some(server_pull_message::Payload::DataPlaneResize(
crates/blit-core/tests/proto_wire_compat.rs:372:            DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:373:                op: DataPlaneResizeOp::Add as i32,
crates/blit-core/tests/proto_wire_compat.rs:386:        payload: Some(server_pull_message::Payload::Summary(Default::default())),
crates/blit-core/tests/proto_wire_compat.rs:395:        payload: Some(client_push_request::Payload::DataPlaneResize(
crates/blit-core/tests/proto_wire_compat.rs:396:            DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:397:                op: DataPlaneResizeOp::Remove as i32,
crates/blit-core/tests/proto_wire_compat.rs:410:            Default::default(),
crates/blit-core/tests/proto_wire_compat.rs:424:        payload: Some(server_push_response::Payload::DataPlaneResizeAck(
crates/blit-core/tests/proto_wire_compat.rs:425:            DataPlaneResizeAck {
crates/blit-core/tests/proto_wire_compat.rs:438:        payload: Some(client_pull_message::Payload::DataPlaneResizeAck(
crates/blit-core/tests/proto_wire_compat.rs:439:            DataPlaneResizeAck {
crates/blit-core/tests/proto_wire_compat.rs:463:    let add = DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:464:        op: DataPlaneResizeOp::Add as i32,
crates/blit-core/tests/proto_wire_compat.rs:469:    let decoded = DataPlaneResize::decode(add.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/proto_wire_compat.rs:471:    assert_eq!(decoded.op(), DataPlaneResizeOp::Add);
crates/blit-core/tests/proto_wire_compat.rs:473:    let remove = DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:474:        op: DataPlaneResizeOp::Remove as i32,
crates/blit-core/tests/proto_wire_compat.rs:479:    let decoded = DataPlaneResize::decode(remove.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/proto_wire_compat.rs:482:    let ack = DataPlaneResizeAck {
crates/blit-core/tests/proto_wire_compat.rs:487:    let decoded = DataPlaneResizeAck::decode(ack.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/proto_wire_compat.rs:498:        receiver_capacity: Some(full_capacity_profile()),

codex
The responder split still sends `SessionError` for local handshake faults and still advertises receiver capacity only when the responder is DESTINATION. I’m now comparing the factored bodies against pre-commit behavior for subtle moved-state regressions.
exec
/usr/bin/zsh -lc 'git show 84be1cc''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'390,610p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git show 84be1cc''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'610,1040p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   390	) -> Result<Negotiated> {
   391	    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
   392	    // direction; no ordering between the two directions.
   393	    transport
   394	        .send(frame(Frame::Hello(SessionHello {
   395	            build_id: hello.build_id.clone(),
   396	            contract_version: hello.contract_version,
   397	        })))
   398	        .await?;
   399	
   400	    let peer_hello = match expect_frame(transport).await? {
   401	        Frame::Hello(h) => h,
   402	        other => {
   403	            return Err(notify_and_wrap(
   404	                transport,
   405	                SessionFault::protocol_violation(format!(
   406	                    "expected SessionHello, got {}",
   407	                    frame_name(&Some(other))
   408	                )),
   409	            )
   410	            .await)
   411	        }
   412	    };
   413	
   414	    if peer_hello.build_id != hello.build_id
   415	        || peer_hello.contract_version != hello.contract_version
   416	    {
   417	        let fault = SessionFault {
   418	            code: session_error::Code::BuildMismatch,
   419	            message: format!(
   420	                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
   421	                hello.build_id, hello.contract_version,
   422	                peer_hello.build_id, peer_hello.contract_version,
   423	            ),
   424	            local_build_id: hello.build_id.clone(),
   425	            peer_build_id: peer_hello.build_id.clone(),
   426	            peer_notified: false,
   427	        };
   428	        return Err(notify_and_wrap(transport, fault).await);
   429	    }
   430	
   431	    match endpoint {
   432	        SessionEndpoint::Initiator { open } => {
   433	            let open = open.as_ref().clone();
   434	            transport.send(frame(Frame::Open(open.clone()))).await?;
   435	            let accept = match expect_frame(transport).await? {
   436	                Frame::Accept(a) => a,
   437	                other => {
   438	                    return Err(notify_and_wrap(
   439	                        transport,
   440	                        SessionFault::protocol_violation(format!(
   441	                            "expected SessionAccept, got {}",
   442	                            frame_name(&Some(other))
   443	                        )),
   444	                    )
   445	                    .await)
   446	                }
   447	            };
   448	            Ok(Negotiated {
   449	                open,
   450	                accept,
   451	                resolved_root: None,
   452	                responder_data_plane: None,
   453	            })
   454	        }
   455	        SessionEndpoint::Responder => {
   456	            let open = match expect_frame(transport).await? {
   457	                Frame::Open(o) => o,
   458	                other => {
   459	                    return Err(notify_and_wrap(
   460	                        transport,
   461	                        SessionFault::protocol_violation(format!(
   462	                            "expected SessionOpen, got {}",
   463	                            frame_name(&Some(other))
   464	                        )),
   465	                    )
   466	                    .await)
   467	                }
   468	            };
   469	            // The initiator declares ITS role; this responder end must
   470	            // hold the complement.
   471	            let declared =
   472	                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   473	            if declared != complement(local_role) {
   474	                return Err(notify_and_wrap(
   475	                    transport,
   476	                    SessionFault::protocol_violation(format!(
   477	                        "initiator declared role {} but this responder is {}",
   478	                        declared.as_str_name(),
   479	                        local_role.as_str_name()
   480	                    )),
   481	                )
   482	                .await);
   483	            }
   484	            if let Err(fault) = validate_open(&open) {
   485	                // Refusal is a SessionError instead of SessionAccept,
   486	                // never a silent close (contract §Phase state machine).
   487	                return Err(notify_and_wrap(transport, fault).await);
   488	            }
   489	            // Responder endpoint resolution (otp-4): map the wire
   490	            // module/path to a local root and enforce read-only, both
   491	            // BEFORE SessionAccept so a refusal replaces the accept
   492	            // (never follows it). The resolver is caller-supplied
   493	            // (daemon module lookup); a fixed-root responder passes
   494	            // None and resolves nothing here.
   495	            let resolved_root = match resolve_open {
   496	                Some(resolve) => match resolve(&open).await {
   497	                    Ok(resolved) => {
   498	                        // A read-only module is fatal only for a
   499	                        // DESTINATION (it would write); a SOURCE
   500	                        // responder (otp-5, daemon-send) reads happily.
   501	                        if local_role == TransferRole::Destination && resolved.read_only {
   502	                            return Err(notify_and_wrap(
   503	                                transport,
   504	                                SessionFault::read_only(
   505	                                    "destination module is read-only".to_string(),
   506	                                ),
   507	                            )
   508	                            .await);
   509	                        }
   510	                        Some(resolved.root)
   511	                    }
   512	                    Err(fault) => return Err(notify_and_wrap(transport, fault).await),
   513	                },
   514	                None => None,
   515	            };
   516	            // Data plane (otp-4b): a DESTINATION responder binds a TCP
   517	            // listener and grants it, unless the initiator requested the
   518	            // in-stream carrier or the bind fails (grant-less accept ⇒
   519	            // in-stream fallback). A SOURCE responder (otp-5,
   520	            // daemon-send) will bind on its own branch later; otp-4b's
   521	            // responder is always the DESTINATION.
   522	            let responder_data_plane =
   523	                if local_role == TransferRole::Destination && !open.in_stream_bytes {
   524	                    data_plane::prepare_responder_data_plane().await
   525	                } else {
   526	                    None
   527	                };
   528	            let accept = SessionAccept {
   529	                // The byte RECEIVER advertises capacity at session
   530	                // open (D-2026-06-20-1/-2); consumed by the dial when
   531	                // the data plane lands (otp-4b).
   532	                receiver_capacity: if local_role == TransferRole::Destination {
   533	                    Some(crate::engine::local_receiver_capacity())
   534	                } else {
   535	                    None
   536	                },
   537	                // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
   538	                data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
   539	            };
   540	            transport.send(frame(Frame::Accept(accept.clone()))).await?;
   541	            Ok(Negotiated {
   542	                open,
   543	                accept,
   544	                resolved_root,
   545	                responder_data_plane,
   546	            })
   547	        }
   548	    }
   549	}
   550	
   551	/// Receive one frame during establish; peer errors and closes become
   552	/// terminal faults.
   553	async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
   554	    match transport.recv().await? {
   555	        Some(TransferFrame {
   556	            frame: Some(Frame::Error(err)),
   557	        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
   558	        Some(TransferFrame { frame: Some(f) }) => Ok(f),
   559	        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
   560	            SessionFault::protocol_violation("frame with empty oneof"),
   561	        )),
   562	        None => Err(eyre::Report::new(SessionFault::internal(
   563	            "peer closed during session establish",
   564	        ))),
   565	    }
   566	}
   567	
   568	/// Send the fault to the peer (best effort), mark it notified, and
   569	/// wrap it for return.
   570	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   571	    let _ = transport.send(error_frame(&fault)).await;
   572	    fault.peer_notified = true;
   573	    eyre::Report::new(fault)
   574	}
   575	
   576	// ---------------------------------------------------------------------------
   577	// SOURCE driver
   578	// ---------------------------------------------------------------------------
   579	
   580	/// Events the source's receive half forwards to its send half. The
   581	/// channel is unbounded but bounded by construction: every `Need`
   582	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   583	/// fault the session), so the queue never exceeds the source's own
   584	/// manifest size — the contract's bounded-buffering rule holds.
   585	enum SourceEvent {
   586	    Need(FileHeader),
   587	    NeedComplete,
   588	    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
   589	    /// send half dials the epoch-N socket on `accepted`.
   590	    ResizeAck(DataPlaneResizeAck),
   591	    Summary(TransferSummary),
   592	    Fault(SessionFault),
   593	}
   594	
   595	/// Run the SOURCE role of one transfer session over `transport`.
   596	/// Returns the destination-computed `TransferSummary` (contract: the
   597	/// end that wrote the bytes is the end that attests to them).
   598	pub async fn run_source(
   599	    cfg: SourceSessionConfig,
   600	    transport: FrameTransport,
   601	    source: Arc<dyn TransferSource>,
   602	) -> Result<TransferSummary> {
   603	    let mut transport = transport;
   604	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   605	        // Own-config coherence: a source initiator declares SOURCE.
   606	        let declared = TransferRole::try_from(open.initiator_role);
   607	        if declared != Ok(TransferRole::Source) {
   608	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   609	        }
   610	        if let Err(fault) = source_open_validator(open) {

 succeeded in 0ms:
   610	        if let Err(fault) = source_open_validator(open) {
   611	            eyre::bail!("run_source initiator config unsupported: {fault}");
   612	        }
   613	    }
   614	
   615	    let negotiated = establish(
   616	        &mut transport,
   617	        &cfg.hello,
   618	        &cfg.endpoint,
   619	        TransferRole::Source,
   620	        &source_open_validator,
   621	        // A SOURCE responder's endpoint resolution (module→root for a
   622	        // daemon-send) lands with otp-5; otp-4a's daemon is always the
   623	        // DESTINATION responder, so the source never resolves here.
   624	        None,
   625	    )
   626	    .await?;
   627	
   628	    let (mut tx, rx) = transport.split();
   629	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   630	    // Set by the send half the moment ManifestComplete goes out. On
   631	    // an ordered transport, a NeedComplete arriving while this is
   632	    // still false is provably premature — the peer cannot have
   633	    // received what we have not sent (contract: NeedComplete only
   634	    // after ManifestComplete received + all entries diffed).
   635	    let manifest_sent = Arc::new(AtomicBool::new(false));
   636	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   637	    // AbortOnDrop: an early error return below must abort the receive
   638	    // half instead of leaking it (same rationale as design-2 / w4-1).
   639	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   640	        rx,
   641	        Arc::clone(&sent),
   642	        Arc::clone(&manifest_sent),
   643	        event_tx,
   644	    )));
   645	
   646	    match source_send_half(
   647	        &cfg,
   648	        &negotiated,
   649	        &mut tx,
   650	        source,
   651	        sent,
   652	        &manifest_sent,
   653	        event_rx,
   654	    )
   655	    .await
   656	    {
   657	        Ok(summary) => Ok(summary),
   658	        Err(report) => {
   659	            let mut fault = fault_from_report(report);
   660	            if !fault.peer_notified {
   661	                let _ = tx.send(error_frame(&fault)).await;
   662	                fault.peer_notified = true;
   663	            }
   664	            Err(eyre::Report::new(fault))
   665	        }
   666	    }
   667	}
   668	
   669	/// Receive half of the source driver: drains the transport for the
   670	/// whole session so destination sends can never deadlock against a
   671	/// blocked source send, and routes the destination lane to the send
   672	/// half. Terminates on summary, error, close, or violation.
   673	async fn source_recv_half(
   674	    mut rx: Box<dyn FrameRx>,
   675	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   676	    manifest_sent: Arc<AtomicBool>,
   677	    events: mpsc::UnboundedSender<SourceEvent>,
   678	) {
   679	    loop {
   680	        let received = match rx.recv().await {
   681	            Ok(Some(f)) => f,
   682	            Ok(None) => {
   683	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   684	                    "peer closed before TransferSummary",
   685	                )));
   686	                return;
   687	            }
   688	            Err(err) => {
   689	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
   690	                    "transport receive failed: {err:#}"
   691	                ))));
   692	                return;
   693	            }
   694	        };
   695	        match received.frame {
   696	            Some(Frame::NeedBatch(batch)) => {
   697	                for entry in batch.entries {
   698	                    if entry.resume {
   699	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   700	                            format!(
   701	                                "resume-flagged need for '{}' in a session opened without resume",
   702	                                entry.relative_path
   703	                            ),
   704	                        )));
   705	                        return;
   706	                    }
   707	                    let header = sent
   708	                        .lock()
   709	                        .expect("sent-manifest lock poisoned")
   710	                        .remove(&entry.relative_path);
   711	                    match header {
   712	                        Some(h) => {
   713	                            let _ = events.send(SourceEvent::Need(h));
   714	                        }
   715	                        None => {
   716	                            let _ = events.send(SourceEvent::Fault(
   717	                                SessionFault::protocol_violation(format!(
   718	                                    "need for unknown or already-needed path '{}'",
   719	                                    entry.relative_path
   720	                                )),
   721	                            ));
   722	                            return;
   723	                        }
   724	                    }
   725	                }
   726	            }
   727	            Some(Frame::NeedComplete(_)) => {
   728	                if !manifest_sent.load(Ordering::Acquire) {
   729	                    // Fail fast at arrival time (otp-3 codex F2): the
   730	                    // event queue would otherwise let an early
   731	                    // NeedComplete be processed late and pass as
   732	                    // legitimate.
   733	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   734	                        "NeedComplete before the source's ManifestComplete",
   735	                    )));
   736	                    return;
   737	                }
   738	                let _ = events.send(SourceEvent::NeedComplete);
   739	            }
   740	            Some(Frame::ResizeAck(ack)) => {
   741	                // The destination's response to a shape-resize proposal
   742	                // (otp-4b-2). Forward it to the send half, which owns the
   743	                // dial and dials the epoch-N socket on `accepted`.
   744	                let _ = events.send(SourceEvent::ResizeAck(ack));
   745	            }
   746	            Some(Frame::Summary(summary)) => {
   747	                let _ = events.send(SourceEvent::Summary(summary));
   748	                return;
   749	            }
   750	            Some(Frame::Error(err)) => {
   751	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   752	                return;
   753	            }
   754	            other => {
   755	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   756	                    format!("{} on the source's receive lane", frame_name(&other)),
   757	                )));
   758	                return;
   759	            }
   760	        }
   761	    }
   762	}
   763	
   764	async fn source_send_half(
   765	    cfg: &SourceSessionConfig,
   766	    negotiated: &Negotiated,
   767	    tx: &mut Box<dyn FrameTx>,
   768	    source: Arc<dyn TransferSource>,
   769	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   770	    manifest_sent: &AtomicBool,
   771	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   772	) -> Result<TransferSummary> {
   773	    let mut pending: Vec<FileHeader> = Vec::new();
   774	    let mut need_complete = false;
   775	
   776	    // Data plane (otp-4b): dial the granted TCP sockets up front —
   777	    // BEFORE streaming the manifest — so the destination's accept loop
   778	    // (armed the moment it sent SessionAccept) sees the connections
   779	    // promptly rather than waiting out its bounded-accept timeout while
   780	    // a long manifest streams. The sockets sit idle (keepalive covers
   781	    // that) until payloads are queued below. `None` = the in-stream
   782	    // carrier (fallback), which needs no early setup.
   783	    let mut data_plane = match &negotiated.accept.data_plane {
   784	        Some(grant) => {
   785	            let host = cfg.data_plane_host.as_deref().ok_or_else(|| {
   786	                eyre::Report::new(SessionFault::internal(
   787	                    "responder granted a TCP data plane but this initiator has no host to dial",
   788	                ))
   789	            })?;
   790	            Some(
   791	                data_plane::dial_source_data_plane(
   792	                    host,
   793	                    grant,
   794	                    negotiated.accept.receiver_capacity.as_ref(),
   795	                    Arc::clone(&source),
   796	                )
   797	                .await?,
   798	            )
   799	        }
   800	        None => None,
   801	    };
   802	
   803	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
   804	    // fed to the shape table so the SOURCE grows the data-plane stream
   805	    // count as the workload's shape becomes known. Append-only (a need is
   806	    // counted once, when it arrives), and the in-flight resize record the
   807	    // ack is matched against (at most one — the dial enforces it).
   808	    let mut needed_bytes: u64 = 0;
   809	    let mut needed_count: usize = 0;
   810	    let mut pending_resize: Option<data_plane::PendingResize> = None;
   811	
   812	    // Streaming manifest: entries go out as enumeration produces them
   813	    // (immediate start in every direction — plan §Design 2). The open
   814	    // carries no source path: the source end owns its local endpoint.
   815	    let _ = &negotiated.open;
   816	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   817	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   818	    while let Some(header) = header_rx.recv().await {
   819	        sent.lock()
   820	            .expect("sent-manifest lock poisoned")
   821	            .insert(header.relative_path.clone(), header.clone());
   822	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   823	        // Faults detected by the receive half abort the stream now,
   824	        // not after the full scan; needs just accumulate. (Resize acks
   825	        // cannot arrive yet — none is proposed before the payload phase.)
   826	        drain_ready_source_events(
   827	            &mut events,
   828	            &mut pending,
   829	            &mut need_complete,
   830	            &mut needed_bytes,
   831	            &mut needed_count,
   832	            data_plane.as_ref(),
   833	            tx,
   834	            &mut pending_resize,
   835	        )
   836	        .await?;
   837	    }
   838	    let scanned = scan_handle
   839	        .await
   840	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   841	    let scan_complete = unreadable
   842	        .lock()
   843	        .expect("unreadable list lock poisoned")
   844	        .is_empty();
   845	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   846	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   847	        scan_complete,
   848	    })))
   849	    .await?;
   850	    manifest_sent.store(true, Ordering::Release);
   851	
   852	    // Payload phase. The byte carrier is either the TCP data plane
   853	    // (dialed above) or the in-stream record grammar (fallback). Needs
   854	    // accumulated while a batch was being sent become the next planner
   855	    // batch (contract §Transport selection); payloads only flow after
   856	    // ManifestComplete.
   857	    // The in-stream carrier reuses one read buffer across records; the
   858	    // data plane owns its own pooled buffers, so skip that allocation.
   859	    let mut read_buf = if data_plane.is_none() {
   860	        vec![0u8; IN_STREAM_CHUNK]
   861	    } else {
   862	        Vec::new()
   863	    };
   864	    loop {
   865	        drain_ready_source_events(
   866	            &mut events,
   867	            &mut pending,
   868	            &mut need_complete,
   869	            &mut needed_bytes,
   870	            &mut needed_count,
   871	            data_plane.as_ref(),
   872	            tx,
   873	            &mut pending_resize,
   874	        )
   875	        .await?;
   876	        if !pending.is_empty() {
   877	            let batch = std::mem::take(&mut pending);
   878	            match &mut data_plane {
   879	                Some(dp) => {
   880	                    // sf-2: correct the stream count toward the shape the
   881	                    // accumulated need list implies before queueing this
   882	                    // batch (one ADD per epoch; a no-op while one is in
   883	                    // flight or the shape wants no more).
   884	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
   885	                        .await?;
   886	                    let payloads =
   887	                        diff_planner::plan_push_payloads(batch, source.root(), cfg.plan_options)?;
   888	                    // A cancel while earlier batches are actively moving
   889	                    // closes the send pipeline under backpressure, so this
   890	                    // queue fails with a data-plane error — prefer the
   891	                    // peer's framed reason (CANCELLED) the same way the
   892	                    // finish() drain does (otp-4b-3 codex F1). Not raced
   893	                    // against events like finish(): live `Need`s still
   894	                    // arrive here, and `recv_peer_fault` would consume them.
   895	                    if let Err(dp_err) = dp.queue(payloads).await {
   896	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
   897	                    }
   898	                }
   899	                None => {
   900	                    send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf)
   901	                        .await?;
   902	                }
   903	            }
   904	            continue;
   905	        }
   906	        if need_complete {
   907	            break;
   908	        }
   909	        match events.recv().await {
   910	            Some(event) => {
   911	                process_source_event(
   912	                    event,
   913	                    &mut pending,
   914	                    &mut need_complete,
   915	                    &mut needed_bytes,
   916	                    &mut needed_count,
   917	                    data_plane.as_ref(),
   918	                    tx,
   919	                    &mut pending_resize,
   920	                )
   921	                .await?;
   922	            }
   923	            None => {
   924	                return Err(eyre::Report::new(SessionFault::internal(
   925	                    "source receive half ended before NeedComplete",
   926	                )))
   927	            }
   928	        }
   929	    }
   930	
   931	    // A resize proposed on the last batch may still be in flight. Resolve
   932	    // it BEFORE finishing so the destination's armed slot is consumed by
   933	    // the dialed socket — an armed-but-never-dialed credential would hang
   934	    // its accept loop (which waits for every arm to be claimed). We do not
   935	    // propose further here: exactly the one in-flight resize is drained.
   936	    if let Some(dp) = &data_plane {
   937	        if let Some(pending) = pending_resize.take() {
   938	            resolve_in_flight_resize(&mut events, dp, pending).await?;
   939	        }
   940	    }
   941	
   942	    // Close the data plane BEFORE SourceDone so the destination's receive
   943	    // pipeline sees each socket's END record and completes; SourceDone on
   944	    // the control lane then lets the destination score and summarize.
   945	    //
   946	    // The drain is the byte-transfer phase's wall-time sink, so a
   947	    // mid-transfer cancel almost always lands here. Race it against a
   948	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
   949	    // the served session frames `SessionError{CANCELLED}`, and the source
   950	    // must surface THAT — not the data-plane transport break it also
   951	    // causes. Two orderings, both covered:
   952	    //   * fault arrives while the drain is still pending (e.g. a worker
   953	    //     blocked reading a slow file, so the socket break never unblocks
   954	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
   955	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
   956	    //     stops the in-flight workers.
   957	    //   * the socket break makes `finish()` return `Err` first → prefer
   958	    //     the framed reason if the control lane delivers one within the
   959	    //     stall window (`prefer_peer_fault`).
   960	    if let Some(dp) = data_plane.take() {
   961	        tokio::select! {
   962	            biased;
   963	            fault = recv_peer_fault(&mut events) => {
   964	                return Err(eyre::Report::new(fault));
   965	            }
   966	            res = dp.finish() => {
   967	                if let Err(dp_err) = res {
   968	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
   969	                }
   970	            }
   971	        }
   972	    }
   973	
   974	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
   975	
   976	    // CLOSING: the destination is the scorer; the next event must be
   977	    // its summary (the receive half ends after forwarding it).
   978	    match events.recv().await {
   979	        Some(SourceEvent::Summary(summary)) => Ok(summary),
   980	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
   981	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
   982	            format!("need for '{}' after NeedComplete", h.relative_path),
   983	        ))),
   984	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
   985	            SessionFault::protocol_violation("duplicate NeedComplete"),
   986	        )),
   987	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
   988	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
   989	        )),
   990	        None => Err(eyre::Report::new(SessionFault::internal(
   991	            "source receive half ended before TransferSummary",
   992	        ))),
   993	    }
   994	}
   995	
   996	/// Process every event ready right now (needs accumulating, resize acks
   997	/// dialing their epoch-N socket) without blocking. Called between
   998	/// manifest sends and at the top of the payload loop.
   999	#[allow(clippy::too_many_arguments)]
  1000	async fn drain_ready_source_events(
  1001	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1002	    pending: &mut Vec<FileHeader>,
  1003	    need_complete: &mut bool,
  1004	    needed_bytes: &mut u64,
  1005	    needed_count: &mut usize,
  1006	    data_plane: Option<&data_plane::SourceDataPlane>,
  1007	    tx: &mut Box<dyn FrameTx>,
  1008	    pending_resize: &mut Option<data_plane::PendingResize>,
  1009	) -> Result<()> {
  1010	    while let Ok(event) = events.try_recv() {
  1011	        process_source_event(
  1012	            event,
  1013	            pending,
  1014	            need_complete,
  1015	            needed_bytes,
  1016	            needed_count,
  1017	            data_plane,
  1018	            tx,
  1019	            pending_resize,
  1020	        )
  1021	        .await?;
  1022	    }
  1023	    Ok(())
  1024	}
  1025	
  1026	/// Handle one source event. Needs accumulate into `pending` and the
  1027	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1028	/// next ADD (the one-per-epoch ramp).
  1029	#[allow(clippy::too_many_arguments)]
  1030	async fn process_source_event(
  1031	    event: SourceEvent,
  1032	    pending: &mut Vec<FileHeader>,
  1033	    need_complete: &mut bool,
  1034	    needed_bytes: &mut u64,
  1035	    needed_count: &mut usize,
  1036	    data_plane: Option<&data_plane::SourceDataPlane>,
  1037	    tx: &mut Box<dyn FrameTx>,
  1038	    pending_resize: &mut Option<data_plane::PendingResize>,
  1039	) -> Result<()> {
  1040	    match event {

exec
/usr/bin/zsh -lc 'git show 84be1cc''^:crates/blit-daemon/src/service/transfer.rs | nl -ba | sed -n '"'90,155p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    90	    })
    91	}
    92	
    93	/// Run one daemon-side transfer session to completion as the DESTINATION
    94	/// Responder, returning `Ok(())` on a clean transfer or `Err(Status)`
    95	/// carrying the session fault's message for the jobs record. The
    96	/// session communicates its own refusals to the peer as `SessionError`
    97	/// *frames* (via the response stream); this `Status` is for the
    98	/// daemon's outcome record and `resolve_streaming_outcome`'s terminal
    99	/// handling, not the primary error channel.
   100	pub(crate) async fn run_transfer_session(
   101	    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
   102	    default_root: Option<RootExport>,
   103	    inbound: Streaming<TransferFrame>,
   104	    tx: mpsc::Sender<Result<TransferFrame, Status>>,
   105	) -> Result<(), Status> {
   106	    let transport = grpc_daemon_transport(tx, inbound);
   107	    let resolver = make_open_resolver(modules, default_root);
   108	    let cfg = DestinationSessionConfig {
   109	        hello: HelloConfig::default(),
   110	        endpoint: SessionEndpoint::Responder,
   111	    };
   112	    match run_destination(cfg, transport, DestinationTarget::Resolve(resolver)).await {
   113	        Ok(_outcome) => Ok(()),
   114	        Err(report) => {
   115	            // run_destination already emitted a SessionError frame to
   116	            // the peer; surface the reason for the record.
   117	            let msg = report
   118	                .downcast_ref::<SessionFault>()
   119	                .map(|f| f.message.clone())
   120	                .unwrap_or_else(|| format!("{report:#}"));
   121	            Err(Status::internal(msg))
   122	        }
   123	    }
   124	}
   125	
   126	#[cfg(test)]
   127	mod tests {
   128	    use super::*;
   129	
   130	    #[test]
   131	    fn status_to_fault_maps_not_found_to_module_unknown() {
   132	        let fault = status_to_fault(Status::not_found("module 'x' not found"));
   133	        assert_eq!(fault.code, Code::ModuleUnknown);
   134	        assert!(fault.message.contains("not found"));
   135	    }
   136	
   137	    #[test]
   138	    fn status_to_fault_maps_permission_denied_to_protocol_violation() {
   139	        let fault = status_to_fault(Status::permission_denied("path containment: escape"));
   140	        assert_eq!(fault.code, Code::ProtocolViolation);
   141	    }
   142	
   143	    #[test]
   144	    fn status_to_fault_maps_invalid_argument_to_protocol_violation() {
   145	        let fault = status_to_fault(Status::invalid_argument("path not allowed"));
   146	        assert_eq!(fault.code, Code::ProtocolViolation);
   147	    }
   148	}

exec
/usr/bin/zsh -lc 'git diff --word-diff=plain --unified=20 84be1cc''^ 84be1cc -- crates/blit-core/src/transfer_session/mod.rs | sed -n '"'1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 48b3169..0ecb287 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -22,41 +22,41 @@ use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::generated::transfer_frame::Frame;
use crate::generated::{
    session_error, ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp,
    FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch, NeedComplete, NeedEntry,
    SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone, TarShardComplete,
    TarShardHeader, TransferFrame, TransferRole, TransferSummary,
};
use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
use crate::remote::transfer::diff_planner;
use crate::remote::transfer::payload::PreparedPayload;
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
use [-crate::remote::transfer::source::TransferSource;-]{+crate::remote::transfer::source::{FsTransferSource, TransferSource};+}
use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
use crate::transfer_plan::PlanOptions;
use transport::{FrameRx, FrameTransport, FrameTx};

/// Belt-and-braces wire-shape version, bumped on any change to the
/// frame set or grammar. Exchanged (and exact-matched) in
/// `SessionHello` alongside the build id (D-2026-07-05-2).
pub const CONTRACT_VERSION: u32 = 1;

/// Payload chunk size on the in-stream carrier. Same unit the gRPC
/// control plane uses today; the data plane (otp-4) has its own.
const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;

/// Manifest entries buffered per destination diff batch. Mirrors the
/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
/// blocking pool instead of inline per entry.
const DEST_DIFF_CHUNK: usize = 128;
@@ -308,40 +308,65 @@ pub struct ResolvedEndpoint {
/// silent close (contract §Phase state machine).
pub type OpenResolver = dyn Fn(
        &SessionOpen,
    )
        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
    + Send
    + Sync;

/// Where a DESTINATION driver writes. `Fixed` is a root known up front
/// (an initiator's own local root, or a test's temp dir). `Resolve`
/// defers to a caller callback that maps the received `SessionOpen` to
/// a local root — the daemon path, where the root depends on the wire
/// module name and so can only be resolved mid-handshake (after HELLO,
/// before SessionAccept). A `Resolve` target is meaningful only on a
/// Responder; an Initiator always knows its own root.
pub enum DestinationTarget {
    Fixed(PathBuf),
    Resolve(Box<OpenResolver>),
}

{+/// Where a SOURCE responder reads from. Symmetric with+}
{+/// [`DestinationTarget`]: `Fixed` is a source known up front (an+}
{+/// initiator's own tree, or a test), `Resolve` defers to the same+}
{+/// [`OpenResolver`] the destination path uses to map a received+}
{+/// `SessionOpen`'s module name to a local root, from which a+}
{+/// [`FsTransferSource`] is built inside blit-core (so callers stay free+}
{+/// of the concrete source type, exactly as `run_destination` builds its+}
{+/// sink from `dst_root`). A `Resolve` target is meaningful only on a+}
{+/// Responder; an Initiator always knows its own source. Used by+}
{+/// [`run_responder`] for the daemon-as-SOURCE (pull-equivalent, otp-5).+}
{+pub enum SourceResponderTarget {+}
{+    Fixed(Arc<dyn TransferSource>),+}
{+    Resolve(Box<OpenResolver>),+}
{+}+}

{+/// What a served session produced, tagged by which role the responder+}
{+/// played. `run_responder` dispatches on the initiator's declared role,+}
{+/// so the caller (the daemon) learns after the fact which half ran.+}
{+pub enum ResponderOutcome {+}
{+    /// The initiator was SOURCE; this end received (push-equivalent).+}
{+    Destination(DestinationOutcome),+}
{+    /// The initiator was DESTINATION; this end sent (pull-equivalent).+}
{+    Source(TransferSummary),+}
{+}+}

fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    if open.resume.as_ref().is_some_and(|r| r.enabled) {
        return Err(SessionFault::internal(
            "resume is not implemented on the unified session yet (otp-7)",
        ));
    }
    if open
        .filter
        .as_ref()
        .is_some_and(|f| *f != FilterSpec::default())
    {
        return Err(SessionFault::internal(
            "filters are not implemented on the unified session yet (otp-6)",
        ));
    }
    Ok(())
}

fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    if open.mirror_enabled {
@@ -357,210 +382,230 @@ fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), Ses
    Ok(())
}

/// Outcome of the HELLO + OPEN phases.
struct Negotiated {
    open: SessionOpen,
    /// The responder's reply. The SOURCE initiator reads
    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
    accept: SessionAccept,
    /// The write root a Responder's [`OpenResolver`] produced from the
    /// received open, if one was supplied; `None` for an Initiator or a
    /// fixed-root Responder (the caller supplies the root then).
    resolved_root: Option<PathBuf>,
    /// The bound data-plane listener + credentials a DESTINATION
    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
    /// on an Initiator, or when the responder granted no data plane
    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
    responder_data_plane: Option<data_plane::ResponderDataPlane>,
}

/// HELLO[-+ OPEN/ACCEPT, one implementation both roles call (otp-3-]
[-/// scoping requirement). Sends the refusal `SessionError` itself when-]
[-/// it detects the fault locally; returned faults are `peer_notified`.-]
[-async fn establish(-]
[-    transport: &mut FrameTransport,-]
[-    hello: &HelloConfig,-]
[-    endpoint: &SessionEndpoint,-]
[-    local_role: TransferRole,-]
[-    validate_open: &OpenValidator,-]
[-    // Consulted only on the Responder branch, after the received open-]
[-    // passes `validate_open` and before SessionAccept. `None` = the-]
[-    // caller supplies the root itself (Initiator, or fixed-root test).-]
[-    resolve_open: Option<&OpenResolver>,-]
[-) -> Result<Negotiated> {-]
[-    // HELLO-] both ways, exact match (D-2026-07-05-2). First frame each
[-//-]{+///+} direction; no ordering between the two directions. {+Factored out so a+}
{+/// serving end (`run_responder`) can exchange HELLO, then read the OPEN+}
{+/// and dispatch on the declared role before running a role driver.+}
{+async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {+}
    transport
        .send(frame(Frame::Hello(SessionHello {
            build_id: hello.build_id.clone(),
            contract_version: hello.contract_version,
        })))
        .await?;

    let peer_hello = match expect_frame(transport).await? {
        Frame::Hello(h) => h,
        other => {
            return Err(notify_and_wrap(
                transport,
                SessionFault::protocol_violation(format!(
                    "expected SessionHello, got {}",
                    frame_name(&Some(other))
                )),
            )
            .await)
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
    {+Ok(())+}
{+}+}

{+/// The responder half of establish AFTER the `SessionOpen` is read:+}
{+/// complement check, `validate_open`, endpoint resolution, data-plane+}
{+/// prepare, and `SessionAccept`. Factored out so both `establish` (which+}
{+/// reads the open then calls this) and `run_responder` (which reads the+}
{+/// open, dispatches on the declared role, then calls this with the+}
{+/// resolved local role) share one implementation. Sends the refusal+}
{+/// `SessionError` itself; returned faults are `peer_notified`.+}
{+async fn responder_finish(+}
{+    transport: &mut FrameTransport,+}
{+    open: SessionOpen,+}
{+    local_role: TransferRole,+}
{+    validate_open: &OpenValidator,+}
{+    resolve_open: Option<&OpenResolver>,+}
{+) -> Result<Negotiated> {+}
{+    // The initiator declares ITS role; this responder end must+}
{+    // hold the complement.+}
{+    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);+}
{+    if declared != complement(local_role) {+}
{+        return Err(notify_and_wrap(+}
{+            transport,+}
{+            SessionFault::protocol_violation(format!(+}
{+                "initiator declared role {} but this responder is {}",+}
{+                declared.as_str_name(),+}
{+                local_role.as_str_name()+}
{+            )),+}
{+        )+}
{+        .await);+}
{+    }+}
{+    if let Err(fault) = validate_open(&open) {+}
{+        // Refusal is a SessionError instead of SessionAccept,+}
{+        // never a silent close (contract §Phase state machine).+}
{+        return Err(notify_and_wrap(transport, fault).await);+}
{+    }+}
{+    // Responder endpoint resolution (otp-4): map the wire+}
{+    // module/path to a local root and enforce read-only, both+}
{+    // BEFORE SessionAccept so a refusal replaces the accept+}
{+    // (never follows it). The resolver is caller-supplied+}
{+    // (daemon module lookup); a fixed-root responder passes+}
{+    // None and resolves nothing here.+}
{+    let resolved_root = match resolve_open {+}
{+        Some(resolve) => match resolve(&open).await {+}
{+            Ok(resolved) => {+}
{+                // A read-only module is fatal only for a+}
{+                // DESTINATION (it would write); a SOURCE+}
{+                // responder (otp-5, daemon-send) reads happily.+}
{+                if local_role == TransferRole::Destination && resolved.read_only {+}
{+                    return Err(notify_and_wrap(+}
{+                        transport,+}
{+                        SessionFault::read_only("destination module is read-only".to_string()),+}
{+                    )+}
{+                    .await);+}
{+                }+}
{+                Some(resolved.root)+}
{+            }+}
{+            Err(fault) => return Err(notify_and_wrap(transport, fault).await),+}
{+        },+}
{+        None => None,+}
{+    };+}
{+    // Data plane (otp-4b): a DESTINATION responder binds a TCP+}
{+    // listener and grants it, unless the initiator requested the+}
{+    // in-stream carrier or the bind fails (grant-less accept ⇒+}
{+    // in-stream fallback). A SOURCE responder (otp-5, daemon-send)+}
{+    // grants no data plane in otp-5a — the transport/role decoupling+}
{+    // that lets a SOURCE responder bind+grant lands at otp-5b.+}
{+    let responder_data_plane = if local_role == TransferRole::Destination && !open.in_stream_bytes {+}
{+        data_plane::prepare_responder_data_plane().await+}
{+    } else {+}
{+        None+}

exec
/usr/bin/zsh -lc 'git show 84be1cc''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'1320,1485p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1320	    /// is the scorer).
  1321	    pub summary: TransferSummary,
  1322	    /// Paths this end put on the need list, in emission order. The
  1323	    /// role suite pins these identical across role assignments — the
  1324	    /// executable form of the owner's invariance requirement.
  1325	    pub needed_paths: Vec<String>,
  1326	    /// The settled data-plane stream count this end observed (epoch-0 +
  1327	    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
  1328	    /// pin (otp-4b-2) reads it to assert shape correction grew the
  1329	    /// stream set past the zero-knowledge single-stream grant.
  1330	    pub data_plane_streams: Option<usize>,
  1331	}
  1332	
  1333	/// Run the DESTINATION role of one transfer session over `transport`,
  1334	/// writing under the root named by `target`. Diffs the streamed
  1335	/// manifest against its own filesystem (the destination is the one
  1336	/// diff owner — plan §Design 3), returns the summary it computed and
  1337	/// sent.
  1338	///
  1339	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  1340	/// front (an Initiator's own local root, or a test), or
  1341	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  1342	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  1343	/// where the wire module name selects the root).
  1344	pub async fn run_destination(
  1345	    cfg: DestinationSessionConfig,
  1346	    transport: FrameTransport,
  1347	    target: DestinationTarget,
  1348	) -> Result<DestinationOutcome> {
  1349	    let mut transport = transport;
  1350	    let endpoint = match cfg.endpoint {
  1351	        SessionEndpoint::Initiator { mut open } => {
  1352	            let declared = TransferRole::try_from(open.initiator_role);
  1353	            if declared != Ok(TransferRole::Destination) {
  1354	                eyre::bail!(
  1355	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  1356	                );
  1357	            }
  1358	            if let Err(fault) = destination_open_validator(&open) {
  1359	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  1360	            }
  1361	            // Dial contract: the byte receiver advertises capacity in
  1362	            // its open when it is the initiator (contract §Invariants 5).
  1363	            if open.receiver_capacity.is_none() {
  1364	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
  1365	            }
  1366	            SessionEndpoint::Initiator { open }
  1367	        }
  1368	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  1369	    };
  1370	
  1371	    let resolve_open: Option<&OpenResolver> = match &target {
  1372	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1373	        DestinationTarget::Fixed(_) => None,
  1374	    };
  1375	
  1376	    let negotiated = establish(
  1377	        &mut transport,
  1378	        &cfg.hello,
  1379	        &endpoint,
  1380	        TransferRole::Destination,
  1381	        &destination_open_validator,
  1382	        resolve_open,
  1383	    )
  1384	    .await?;
  1385	
  1386	    // The resolver's root (Responder + Resolve) wins; otherwise the
  1387	    // caller-supplied Fixed root.
  1388	    let dst_root = match negotiated.resolved_root.clone() {
  1389	        Some(root) => root,
  1390	        None => match &target {
  1391	            DestinationTarget::Fixed(root) => root.clone(),
  1392	            // Unreachable: a Resolve target always yields a root on the
  1393	            // Responder branch, and establish only skips resolution on
  1394	            // the Initiator branch (which pairs with a Fixed root).
  1395	            DestinationTarget::Resolve(_) => {
  1396	                return Err(eyre::Report::new(SessionFault::internal(
  1397	                    "resolver target produced no destination root",
  1398	                )));
  1399	            }
  1400	        },
  1401	    };
  1402	
  1403	    match destination_session(&mut transport, negotiated, &dst_root).await {
  1404	        Ok(outcome) => Ok(outcome),
  1405	        Err(report) => {
  1406	            let mut fault = fault_from_report(report);
  1407	            if !fault.peer_notified {
  1408	                let _ = transport.send(error_frame(&fault)).await;
  1409	                fault.peer_notified = true;
  1410	            }
  1411	            Err(eyre::Report::new(fault))
  1412	        }
  1413	    }
  1414	}
  1415	
  1416	fn violation(message: String) -> eyre::Report {
  1417	    eyre::Report::new(SessionFault::protocol_violation(message))
  1418	}
  1419	
  1420	async fn destination_session(
  1421	    transport: &mut FrameTransport,
  1422	    negotiated: Negotiated,
  1423	    dst_root: &Path,
  1424	) -> Result<DestinationOutcome> {
  1425	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  1426	        .unwrap_or(ComparisonMode::Unspecified);
  1427	    let compare_opts = CompareOptions {
  1428	        mode: compare_mode.into(),
  1429	        ignore_existing: negotiated.open.ignore_existing,
  1430	        include_deletions: false, // mirror lands at otp-6
  1431	    };
  1432	    // src_root is only consumed by local File payloads, which never
  1433	    // occur on a session destination (payload bytes arrive as records
  1434	    // and go through the stream/tar write paths). `Arc` so the data-plane
  1435	    // receive task (otp-4b) can share the one sink across sockets.
  1436	    let sink = Arc::new(FsTransferSink::new(
  1437	        PathBuf::new(),
  1438	        dst_root.to_path_buf(),
  1439	        FsSinkConfig {
  1440	            preserve_times: true,
  1441	            dry_run: false,
  1442	            checksum: None,
  1443	            resume: false,
  1444	            compare_mode,
  1445	        },
  1446	    ));
  1447	    // Same canonical-containment chokepoint the sink write paths use
  1448	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  1449	    // make the destination stat outside its root.
  1450	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  1451	
  1452	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  1453	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  1454	    // insert-only, never removed, so a concurrent data-plane claim can
  1455	    // never re-open a grant (a duplicate manifest path is granted at
  1456	    // most once regardless of delivery timing). `outstanding` is the
  1457	    // not-yet-delivered COMPLETION set — inserted for each freshly
  1458	    // granted path before its NeedBatch, claimed by both carriers (the
  1459	    // in-stream arms inline, the data-plane NeedListSink as payloads
  1460	    // land), and empty at SourceDone. A count proxy was insufficient
  1461	    // (F1); merging the two into one set raced the data-plane claim
  1462	    // against the diff (fix-review F1).
  1463	    let mut granted: HashSet<String> = HashSet::new();
  1464	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  1465	
  1466	    // Data plane (otp-4b): when the responder granted a TCP data plane,
  1467	    // payload bytes arrive on sockets (not the control lane). Arm the
  1468	    // accept+receive task NOW — concurrent with the diff loop below, and
  1469	    // before the source dials — so the connections are accepted promptly.
  1470	    // The NeedListSink gives the socket receive the same need-list
  1471	    // strictness the in-stream control loop applies inline. AbortOnDrop
  1472	    // bounds it to this future: a control-lane fault that returns from
  1473	    // this fn aborts the receive task instead of leaking it.
  1474	    // `resize_live` tracks the stream count this end has granted (epoch-0
  1475	    // plus each accepted resize ADD); `resize_ceiling` is the receiver's
  1476	    // advertised max_streams, the cumulative bound a resize may not cross.
  1477	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  1478	        match negotiated.responder_data_plane {
  1479	            Some(rdp) => {
  1480	                let initial = rdp.initial_streams() as usize;
  1481	                let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  1482	                    Arc::clone(&sink) as Arc<dyn TransferSink>,
  1483	                    Arc::clone(&outstanding),
  1484	                ));
  1485	                let run = rdp.spawn(recv_sink);

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1630,1885p'" in /home/michael/dev/Blit
 succeeded in 0ms:
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

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1885,2055p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1885	            }
  1886	            Some(Frame::Error(err)) => {
  1887	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  1888	            }
  1889	            other => {
  1890	                // Everything else is off-lane or off-phase here:
  1891	                // destination-lane frames echoed back (a ResizeAck the
  1892	                // destination would never receive), resume frames in a
  1893	                // non-resume session (otp-7), stray handshake frames,
  1894	                // bare FileData/TarShardChunk outside a record. Fail
  1895	                // fast, no tolerant parsing.
  1896	                return Err(violation(format!(
  1897	                    "{} not valid on the destination's receive lane in this phase",
  1898	                    frame_name(&other)
  1899	                )));
  1900	            }
  1901	        }
  1902	    }
  1903	}
  1904	
  1905	/// Stat-and-compare one chunk of manifest entries on the blocking
  1906	/// pool (2+ syscalls per entry — same rationale as the daemon's
  1907	/// w4-4 chunked checks), then stream the resulting need batch.
  1908	async fn diff_chunk_and_send_needs(
  1909	    transport: &mut FrameTransport,
  1910	    chunk: Vec<FileHeader>,
  1911	    dst_root: &Path,
  1912	    canonical_dst_root: Option<&Path>,
  1913	    compare_opts: &CompareOptions,
  1914	    // Ever-granted DEDUP set (control-loop-local, insert-only): a path
  1915	    // the source manifests twice is granted at most once, and because it
  1916	    // is never removed, a concurrent data-plane claim can't re-open the
  1917	    // grant (fix-review F1).
  1918	    granted: &mut HashSet<String>,
  1919	    // Not-yet-delivered COMPLETION set (shared with the receive).
  1920	    outstanding: &data_plane::OutstandingNeeds,
  1921	    needed_paths: &mut Vec<String>,
  1922	) -> Result<()> {
  1923	    if chunk.is_empty() {
  1924	        return Ok(());
  1925	    }
  1926	    let dst_root = dst_root.to_path_buf();
  1927	    let canonical = canonical_dst_root.map(Path::to_path_buf);
  1928	    let opts = compare_opts.clone();
  1929	    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
  1930	        let mut needed = Vec::new();
  1931	        for header in &chunk {
  1932	            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
  1933	                needed.push(header.relative_path.clone());
  1934	            }
  1935	        }
  1936	        Ok(needed)
  1937	    })
  1938	    .await
  1939	    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
  1940	
  1941	    // Dedup on the ever-granted set (no lock — control-loop-local), then
  1942	    // insert the freshly granted paths into the shared `outstanding`
  1943	    // completion set BEFORE the NeedBatch goes out. The source can only
  1944	    // send a payload after receiving its need, so insert-before-send
  1945	    // orders the data-plane receive's `claim` strictly after this insert.
  1946	    let fresh: Vec<String> = needed
  1947	        .into_iter()
  1948	        .filter(|path| granted.insert(path.clone()))
  1949	        .collect();
  1950	    let entries: Vec<NeedEntry> = {
  1951	        let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  1952	        fresh
  1953	            .into_iter()
  1954	            .map(|relative_path| {
  1955	                needed_paths.push(relative_path.clone());
  1956	                out.insert(relative_path.clone());
  1957	                NeedEntry {
  1958	                    relative_path,
  1959	                    resume: false, // resume lands at otp-7
  1960	                }
  1961	            })
  1962	            .collect()
  1963	    };
  1964	    if entries.is_empty() {
  1965	        return Ok(());
  1966	    }
  1967	    transport
  1968	        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
  1969	        .await?;
  1970	    Ok(())
  1971	}
  1972	
  1973	/// Does the destination need this manifest entry? Stats its own file
  1974	/// and delegates the verdict to `manifest::header_transfer_status` —
  1975	/// the same mode-aware owner `compare_manifests` uses, fed from a
  1976	/// live stat instead of a materialized target manifest.
  1977	fn destination_needs(
  1978	    header: &FileHeader,
  1979	    dst_root: &Path,
  1980	    canonical_dst_root: Option<&Path>,
  1981	    opts: &CompareOptions,
  1982	) -> Result<bool> {
  1983	    let dst = match canonical_dst_root {
  1984	        Some(canonical) => {
  1985	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
  1986	        }
  1987	        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
  1988	    }
  1989	    .map_err(|err| {
  1990	        SessionFault::protocol_violation(format!(
  1991	            "manifest path '{}' escapes the destination root: {err:#}",
  1992	            header.relative_path
  1993	        ))
  1994	    })?;
  1995	
  1996	    let target = match std::fs::metadata(&dst) {
  1997	        Ok(meta) if meta.is_file() => {
  1998	            let mtime = match meta.modified() {
  1999	                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
  2000	                    Ok(d) => d.as_secs() as i64,
  2001	                    Err(e) => -(e.duration().as_secs() as i64),
  2002	                },
  2003	                Err(_) => 0,
  2004	            };
  2005	            Some((meta.len(), mtime))
  2006	        }
  2007	        // Absent — or present as a directory/other, which a file
  2008	        // write must replace: both diff as "target does not have it"
  2009	        // (matches the push daemon's file_requires_upload).
  2010	        _ => None,
  2011	    };
  2012	    let status = header_transfer_status(
  2013	        header,
  2014	        // Destination-side checksums are never precomputed; Checksum
  2015	        // mode therefore transfers (the conservative arm of
  2016	        // compare_file), matching what push does today.
  2017	        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
  2018	        opts,
  2019	    );
  2020	    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
  2021	}
  2022	
  2023	/// Receive one strictly-serialized file record (`file_begin` already
  2024	/// consumed) and stream its bytes into the sink through a bounded
  2025	/// in-memory pipe — record completion is exactly `header.size`
  2026	/// cumulative bytes (contract §Transport selection).
  2027	async fn receive_file_record(
  2028	    transport: &mut FrameTransport,
  2029	    sink: &FsTransferSink,
  2030	    header: &FileHeader,
  2031	) -> Result<crate::remote::transfer::SinkOutcome> {
  2032	    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
  2033	    let write = sink.write_file_stream(header, &mut pipe_rd);
  2034	    let feed = async {
  2035	        let mut remaining = header.size;
  2036	        while remaining > 0 {
  2037	            let received = match transport.recv().await? {
  2038	                Some(f) => f,
  2039	                None => {
  2040	                    return Err(eyre::Report::new(SessionFault::internal(format!(
  2041	                        "peer closed inside file record '{}'",
  2042	                        header.relative_path
  2043	                    ))))
  2044	                }
  2045	            };
  2046	            match received.frame {
  2047	                Some(Frame::FileData(data)) => {
  2048	                    let len = data.content.len() as u64;
  2049	                    if len > remaining {
  2050	                        return Err(violation(format!(
  2051	                            "file record '{}' overran its size by {} byte(s)",
  2052	                            header.relative_path,
  2053	                            len - remaining
  2054	                        )));
  2055	                    }

exec
/usr/bin/zsh -lc "sed -n '1,220p' docs/TRANSFER_SESSION.md" in /home/michael/dev/Blit
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

exec
/usr/bin/zsh -lc "sed -n '220,520p' docs/TRANSFER_SESSION.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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
/usr/bin/zsh -lc "sed -n '1,280p' crates/blit-core/src/remote/pull.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use base64::{engine::general_purpose, Engine as _};
use eyre::{bail, eyre, Context, Result};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

use crate::generated::blit_client::BlitClient;
use crate::generated::{
    client_pull_message, server_pull_message, BlockHashList, ClientPullMessage, ComparisonMode,
    DataTransferNegotiation, FileData, FileHeader, ManifestComplete, MirrorMode, PeerCapabilities,
    PullSummary, ResumeSettings, ServerPullMessage, TransferOperationSpec,
};
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use crate::remote::transfer::grpc_fallback::recv_fallback_message;
use crate::remote::transfer::progress::{ByteProgressSink, RemoteTransferProgress};
use crate::remote::transfer::AbortOnDrop;

/// Phase-bearing pull-sync error used by delegation callers to preserve the
/// source-refusal vs mid-transfer distinction across the `eyre::Report`
/// boundary. The CLI pull path still renders this as a normal error string.
#[derive(Debug)]
pub enum PullSyncError {
    Negotiation(String),
    Transfer(String),
}

impl PullSyncError {
    pub fn negotiation(status: tonic::Status) -> Self {
        Self::Negotiation(format_status(status))
    }

    fn transfer(status: tonic::Status) -> Self {
        Self::Transfer(format_status(status))
    }

    pub fn negotiation_message(message: impl Into<String>) -> Self {
        Self::Negotiation(message.into())
    }

    pub fn is_negotiation(&self) -> bool {
        matches!(self, Self::Negotiation(_))
    }
}

impl std::fmt::Display for PullSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negotiation(message) => write!(f, "pull sync negotiation failed: {message}"),
            Self::Transfer(message) => write!(f, "pull sync stream failed: {message}"),
        }
    }
}

impl std::error::Error for PullSyncError {}

fn format_status(status: tonic::Status) -> String {
    if status.message().is_empty() {
        status.code().to_string()
    } else {
        format!("{} ({})", status.message(), status.code())
    }
}

/// Options for pull synchronization operations.
#[derive(Debug, Default, Clone)]
pub struct PullSyncOptions {
    /// Force gRPC data plane (no TCP fallback).
    pub force_grpc: bool,
    /// Mirror mode: report files to delete.
    pub mirror_mode: bool,
    /// Mirror scope policy: when true, deletions extend across the
    /// full destination tree (`MirrorMode::All`). Default false →
    /// `MirrorMode::FilteredSubset` so files outside the source
    /// filter scope are never purged.
    pub delete_all_scope: bool,
    /// Filter rules to apply at the daemon's source enumeration.
    /// `None` means no filtering. The daemon converts this to a
    /// `FileFilter` via `NormalizedTransferOperation::from_spec`.
    pub filter: Option<crate::generated::FilterSpec>,
    /// Compare only by size, ignore modification time.
    pub size_only: bool,
    /// Transfer all files unconditionally.
    pub ignore_times: bool,
    /// Skip files that already exist on target.
    pub ignore_existing: bool,
    /// Overwrite even if target is newer (dangerous).
    pub force: bool,
    /// Force checksum comparison (slower but more accurate).
    pub checksum: bool,
    /// Enable block-level resume for partial/changed files.
    pub resume: bool,
    /// Block size for resume (0 = default 1 MiB).
    pub block_size: u32,
    /// R49-F2: when true, the daemon must refuse the operation if
    /// its source-side scan was incomplete. Set by `blit move`,
    /// which deletes the source after the transfer succeeds —
    /// without this signal an EACCES on a source subtree would
    /// silently lose files that never got copied. Independent of
    /// `mirror_mode`: move always uses mirror_mode=false (it
    /// doesn't purge dest extras) but carries the same
    /// scan-completeness requirement.
    pub require_complete_scan: bool,
}

#[derive(Debug, Default, Clone)]
pub struct RemotePullReport {
    pub files_transferred: usize,
    pub bytes_transferred: u64,
    pub downloaded_paths: Vec<PathBuf>,
    pub summary: Option<PullSummary>,
    /// Authoritative deletion list from the daemon (mirror mode only).
    /// `None` means the daemon never sent one (e.g. mirror=Off, or
    /// older daemon — but per the no-back-compat policy the latter
    /// shouldn't reach here). Empty `Some` means "daemon agrees
    /// nothing should be deleted." The CLI deletes exactly these
    /// relative paths and never walks the dest tree on its own.
    ///
    /// Stored as raw wire strings — the consumer routes each through
    /// `path_safety::safe_join` before performing any filesystem op,
    /// so a hostile daemon can't escape the destination via `..`,
    /// absolute paths, or Windows-shaped roots (R5-F1).
    pub paths_to_delete: Option<Vec<String>>,
    /// Daemon's `server_checksums_enabled` advertisement from the
    /// PullSyncAck. `None` means no ack arrived (legacy daemon or
    /// pre-spec wire shape). Set by the receive loop and read by
    /// the CLI to honor F11 of the 2026-05-01 baseline review:
    /// when the client asked for `--checksum` mode but the daemon
    /// has checksums disabled, the comparison would silently
    /// degrade to size+mtime — a real footgun for users expecting
    /// byte-level equality. The pull_sync handshake errors out
    /// before any data flows when this mismatch is detected.
    pub server_checksums_enabled: Option<bool>,
}

pub type RemotePullProgress = RemoteTransferProgress;

struct PullWorkerStats {
    start: Instant,
    files_transferred: u64,
    bytes_transferred: u64,
    downloaded_paths: Vec<PathBuf>,
    bytes: u64,
}

impl PullWorkerStats {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            files_transferred: 0,
            bytes_transferred: 0,
            downloaded_paths: Vec::new(),
            bytes: 0,
        }
    }
}

/// Result from data plane receiver, used to merge with control plane report.
#[derive(Debug)]
struct DataPlaneResult {
    files_transferred: usize,
    bytes_transferred: u64,
    downloaded_paths: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct RemotePullClient {
    endpoint: RemoteEndpoint,
    client: BlitClient<tonic::transport::Channel>,
}

impl RemotePullClient {
    pub async fn connect(endpoint: RemoteEndpoint) -> Result<Self> {
        let uri = endpoint.control_plane_uri();
        // audit-2: bound the connect (30s). Plain `BlitClient::connect`
        // has no deadline, so an unreachable source daemon would hang a
        // delegated/remote pull for the OS TCP timeout (60-127s). The
        // outer `tokio::time::timeout` is what bounds slow DNS too —
        // `connect_timeout` alone only bounds the post-resolution TCP
        // attempt (tonic/hyper-util resolve the name first).
        let conn = tonic::transport::Endpoint::from_shared(uri.clone())
            .map_err(|err| eyre!("invalid endpoint {}: {}", uri, err))?
            .connect_timeout(std::time::Duration::from_secs(30));
        let channel = tokio::time::timeout(std::time::Duration::from_secs(30), conn.connect())
            .await
            .map_err(|_| eyre!("connecting to {} timed out", uri))?
            .map_err(|err| eyre!("failed to connect to {}: {}", uri, err))?;
        let client = BlitClient::new(channel);

        Ok(Self { endpoint, client })
    }

    /// Open a PullSync session with `spec`, immediately completing the
    /// client-manifest phase with an empty manifest, and return the
    /// server-side frame stream. Shared by the relay's metadata scan
    /// and single-file streaming (ue-r2-1h: both rode the deprecated
    /// Pull RPC until its deletion; PullSync is the only pull wire).
    ///
    /// The request-stream sender is dropped on return, half-closing
    /// the client→daemon direction. That is protocol-complete: the
    /// daemon has the spec and the (empty) manifest, and these
    /// sessions never use the resume protocol, the only later
    /// client→daemon traffic. The response stream stays readable.
    async fn open_relay_session(
        &self,
        spec: TransferOperationSpec,
    ) -> Result<Streaming<ServerPullMessage>> {
        use tokio_stream::wrappers::ReceiverStream;

        let mut client = self.client.clone();
        // Capacity 4 comfortably holds both messages even before the
        // daemon consumes anything (cf. the >30-entry deadlock note in
        // `pull_sync_with_spec` — an empty manifest can't reproduce it).
        let (tx, rx) = tokio::sync::mpsc::channel::<ClientPullMessage>(4);
        let response_stream = client
            .pull_sync(ReceiverStream::new(rx))
            .await
            .map_err(|status| eyre!(format_status(status)))?
            .into_inner();

        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::Spec(spec)),
        })
        .await
        .map_err(|_| eyre!("failed to send pull sync spec"))?;
        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::ManifestDone(
                // Nothing enumerated on this side, so the empty
                // manifest is trivially complete.
                ManifestComplete {
                    scan_complete: true,
                },
            )),
        })
        .await
        .map_err(|_| eyre!("failed to send manifest done"))?;

        Ok(response_stream)
    }

    /// Minimal spec for the relay's PullSync sessions: no filter,
    /// default comparison against an empty client manifest (every file
    /// is New → the daemon sends everything asked for), no
    /// mirror/resume, and always `force_grpc` — bytes, when any, ride
    /// the control stream; these sessions never dial a data plane.
    fn build_relay_session_spec(
        endpoint: &RemoteEndpoint,
        path: &Path,
        metadata_only: bool,
    ) -> Result<TransferOperationSpec> {
        let (module, rel_path) = match &endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.join(path)),
            RemotePath::Root { rel_path } => (String::new(), rel_path.join(path)),
            RemotePath::Discovery => bail!("remote source must specify a module"),
        };
        Ok(TransferOperationSpec {
            spec_version: crate::remote::transfer::operation_spec::SUPPORTED_SPEC_VERSION,
            module,
            source_path: normalize_for_request(&rel_path),
            filter: None,
            compare_mode: ComparisonMode::Unspecified as i32,
            mirror_mode: MirrorMode::Unspecified as i32,
            resume: None,
            // Truthful for these sessions: no resume protocol, no tar
            // parsing (the single-file reader consumes bare file_data
            // frames), no TCP data plane, filter chokepoint present.
            client_capabilities: Some(PeerCapabilities {
                supports_resume: false,
                supports_tar_shards: false,
                supports_data_plane_tcp: false,
                supports_filter_spec: true,
                supports_stream_resize: false,
            }),
            force_grpc: true,
            ignore_existing: false,
            require_complete_scan: false,

exec
/usr/bin/zsh -lc "sed -n '280,680p' crates/blit-core/src/remote/pull.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
            require_complete_scan: false,
            // These sessions receive at most one file's bytes on the
            // control stream — nothing for a sender dial to ramp
            // against, so advertise nothing.
            receiver_capacity: None,
            metadata_only,
        })
    }

    /// Spawn data plane receiver as background task, returning JoinHandle.
    /// This allows the control plane to continue processing ManifestBatch messages.
    fn spawn_data_plane_receiver(
        &self,
        negotiation: DataTransferNegotiation,
        dest_root: &Path,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
        byte_progress: Option<&ByteProgressSink>,
    ) -> Result<(
        JoinHandle<Result<DataPlaneResult>>,
        Option<tokio::sync::mpsc::UnboundedSender<PullStreamAdd>>,
    )> {
        if negotiation.tcp_port == 0 {
            bail!("server provided zero data-plane port for pull");
        }
        let token = general_purpose::STANDARD_NO_PAD
            .decode(negotiation.one_time_token.as_bytes())
            .map_err(|err| eyre!("failed to decode pull data-plane token: {err}"))?;

        // Clone/own all values for the spawned task
        let host = self.endpoint.host.clone();
        let port = negotiation.tcp_port;
        let stream_count = bounded_stream_count(negotiation.stream_count);
        let dest_root = dest_root.to_path_buf();
        let progress = progress.cloned();
        let byte_progress = byte_progress.cloned();

        // ue-r2-2: the daemon negotiated resize — hand the control
        // loop a growth channel into the receiver task, and make the
        // epoch-0 sockets echo the sub-token. A malformed token
        // length reads as "not enabled" (fail toward today's shape).
        let resize_on = negotiation.resize_enabled
            && negotiation.epoch0_sub_token.len() == crate::remote::transfer::SUB_TOKEN_LEN;
        let (growth_tx, resize_arg) = if resize_on {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            (Some(tx), Some((negotiation.epoch0_sub_token.clone(), rx)))
        } else {
            (None, None)
        };

        let handle = tokio::spawn(async move {
            receive_data_plane_streams_owned(
                host,
                port,
                token,
                stream_count,
                dest_root,
                track_paths,
                progress,
                byte_progress,
                resize_arg,
            )
            .await
        });
        Ok((handle, growth_tx))
    }
    /// Enumerate the remote subtree's file headers without moving
    /// bytes. Rides a metadata-only PullSync session — ue-r2-1h's port
    /// of the deleted Pull RPC's `metadata_only` request; the daemon
    /// answers with one bare `file_header` frame per file, then a
    /// summary.
    ///
    /// Mixed versions: a daemon that predates
    /// `TransferOperationSpec.metadata_only` ignores the flag and runs
    /// the full protocol — with an empty client manifest and
    /// `force_grpc` it streams every file's bytes over the control
    /// stream. The loop still returns the right headers (they arrive
    /// as `file_header` / `tar_shard_header` frames ahead of the data)
    /// and discards the unwanted bytes: correct, just wasteful.
    pub async fn scan_remote_files(&mut self, path: &Path) -> Result<Vec<FileHeader>> {
        let spec = Self::build_relay_session_spec(&self.endpoint, path, true)?;
        let mut stream = self.open_relay_session(spec).await?;

        let mut headers = Vec::new();
        // audit-h3c slice 1: same receive chokepoint as the pull_sync
        // loop, so slice 2's progress watchdog covers this scan — a
        // stalled peer would otherwise hang it indefinitely.
        //
        // TODO(audit-h3c-2): same error-chain-stripping concern as the
        // pull_sync loop — slice 2 must preserve the io::Error chain
        // through this map_err for the retry classifier to fire.
        while let Some(msg) = recv_fallback_message(&mut stream)
            .await
            .map_err(|status| eyre!(format_status(status)))?
        {
            match msg.payload {
                Some(server_pull_message::Payload::FileHeader(header)) => {
                    headers.push(header);
                }
                // Old-daemon degradation: batched small files arrive
                // as tar shards, whose headers travel in the shard
                // header. The archive bytes themselves fall through to
                // the catch-all below like plain file_data.
                Some(server_pull_message::Payload::TarShardHeader(shard)) => {
                    headers.extend(shard.files);
                }
                Some(server_pull_message::Payload::Summary(_)) => break,
                // force_grpc was set, so a real TCP negotiation (as
                // opposed to a tcp_fallback announcement) means the
                // daemon expects us to dial a data plane this scan
                // never will — fail fast instead of stalling both ends
                // until the daemon's accept timeout.
                Some(server_pull_message::Payload::Negotiation(neg)) if !neg.tcp_fallback => {
                    bail!(
                        "daemon attempted a data-plane negotiation during a \
                         metadata-only scan (force_grpc was set)"
                    );
                }
                // Ack / PullSyncAck / ManifestBatch / FilesToDownload /
                // fallback-announcement Negotiation, and — from old
                // daemons — file_data / tar-shard bytes: all irrelevant
                // to a header scan.
                _ => {}
            }
        }
        Ok(headers)
    }

    /// Stream one remote file's bytes over the control stream. Rides a
    /// single-file `force_grpc` PullSync session — ue-r2-1h's port of
    /// the deleted Pull RPC's single-file gRPC path. With an empty
    /// client manifest the daemon always sends the file; the planner
    /// never tar-shards a single file (and the session's capabilities
    /// advertise no tar support), so the payload arrives as bare
    /// file_header + file_data frames followed by a summary.
    pub async fn open_remote_file(
        &self,
        path: &Path,
    ) -> Result<impl tokio::io::AsyncRead + Unpin + Send> {
        let spec = Self::build_relay_session_spec(&self.endpoint, path, false)?;
        let stream = self.open_relay_session(spec).await?;
        Ok(RemoteFileStream::new(stream))
    }

    /// Build a `TransferOperationSpec` from CLI-style `PullSyncOptions`
    /// and the client's endpoint. Pure function; testable in isolation.
    ///
    /// Lifts two non-contiguous regions of the pre-refactor `pull_sync`:
    /// the endpoint→`module`/`source_path` mapping (was `pull.rs:397-409`)
    /// and the options→spec block (was `pull.rs:433-484`). Returns
    /// `Result` because the `RemotePath::Discovery` variant bails.
    pub fn build_spec_from_options(
        endpoint: &RemoteEndpoint,
        options: &PullSyncOptions,
    ) -> Result<TransferOperationSpec> {
        let (module, rel_path) = match &endpoint.path {
            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
            RemotePath::Discovery => {
                bail!("remote source must specify a module (server:/module/...)");
            }
        };

        let path_str = if rel_path.as_os_str().is_empty() {
            ".".to_string()
        } else {
            normalize_for_request(&rel_path)
        };

        // ComparisonMode covers only the "given the file is being
        // considered, what counts as a match?" axis; the orthogonal
        // "skip if dst exists" axis travels in the top-level
        // `ignore_existing` spec field. The CLI rejects `--force
        // --ignore-existing` (contradictory) before reaching here —
        // but the spec normalizer also rejects it defensively.
        let compare_mode = if options.ignore_times {
            ComparisonMode::IgnoreTimes
        } else if options.force {
            ComparisonMode::Force
        } else if options.size_only {
            ComparisonMode::SizeOnly
        } else if options.checksum {
            ComparisonMode::Checksum
        } else {
            ComparisonMode::SizeMtime
        };
        let mirror = if options.mirror_mode {
            if options.delete_all_scope {
                MirrorMode::All
            } else {
                // Default — files outside the filter scope are not
                // purged from the destination, since the source
                // filter excluded them on purpose.
                MirrorMode::FilteredSubset
            }
        } else {
            MirrorMode::Off
        };
        let filter_spec = options.filter.clone().unwrap_or_default();
        Ok(TransferOperationSpec {
            spec_version: crate::remote::transfer::operation_spec::SUPPORTED_SPEC_VERSION,
            module,
            source_path: path_str,
            filter: Some(filter_spec),
            compare_mode: compare_mode as i32,
            mirror_mode: mirror as i32,
            resume: Some(ResumeSettings {
                enabled: options.resume,
                block_size: options.block_size,
            }),
            client_capabilities: Some(PeerCapabilities {
                supports_resume: true,
                supports_tar_shards: true,
                supports_data_plane_tcp: true,
                supports_filter_spec: true,
                // ue-r2-2: the pull client dials and its receive
                // worker set is growable — advertise resize. The
                // daemon folds this into `resize_enabled`; old daemons
                // skip the bit and nothing changes. (Review catch:
                // this flip was dropped from the original stack —
                // without it pull resize was dead on the CLI path.)
                supports_stream_resize: true,
            }),
            force_grpc: options.force_grpc,
            ignore_existing: options.ignore_existing,
            require_complete_scan: options.require_complete_scan,
            // ue-r2-1e: on pull the CLIENT is the byte receiver — it
            // advertises its capacity so the daemon's dial can ramp
            // within it.
            receiver_capacity: Some(crate::engine::local_receiver_capacity()),
            // Real pulls move bytes; metadata-only sessions build
            // their spec in `build_relay_session_spec`.
            metadata_only: false,
        })
    }

    /// Pull with manifest synchronization — sends local manifest to
    /// server, server compares and only sends files that need updating.
    /// Thin wrapper around `pull_sync_with_spec` for CLI call sites
    /// that build `PullSyncOptions`.
    pub async fn pull_sync(
        &mut self,
        dest_root: &Path,
        local_manifest: Vec<FileHeader>,
        options: &PullSyncOptions,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<RemotePullReport> {
        let spec = Self::build_spec_from_options(&self.endpoint, options)?;
        // CLI-side `pull_sync` has no daemon-side byte counter
        // to feed. Callers that need byte-level reports (e.g.
        // the dst-daemon handler for delegated_pull) reach
        // through `pull_sync_with_spec` directly with a sink.
        self.pull_sync_with_spec(dest_root, local_manifest, spec, track_paths, progress, None)
            .await
    }

    /// Pull using a pre-built, normalized `TransferOperationSpec`. The
    /// spec travels over the wire unchanged.
    ///
    /// IMPORTANT: this method MUST NOT read `self.endpoint.path` to
    /// derive any spec field. The endpoint is purely a transport
    /// handle (host:port for the gRPC connection); the spec is
    /// authoritative for `module` + `source_path` + every other field.
    /// Touching `endpoint.path` here would reopen the
    /// validate-then-reconstruct hole that motivated this split (see
    /// docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 R25-F1).
    ///
    /// Used by the delegated-pull daemon handler AND by the existing
    /// CLI pull entry point (via the `pull_sync` wrapper above).
    pub async fn pull_sync_with_spec(
        &mut self,
        dest_root: &Path,
        local_manifest: Vec<FileHeader>,
        spec: TransferOperationSpec,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
        byte_progress: Option<&ByteProgressSink>,
    ) -> Result<RemotePullReport> {
        use tokio_stream::wrappers::ReceiverStream;

        // Ensure the parent exists; do NOT mkdir dest_root itself — for a
        // single-file pull it's the target file path, and creating it as
        // a directory here would cause the subsequent File::create to fail.
        if let Some(parent) = dest_root.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("creating destination parent {}", parent.display()))?;
            }
        }

        // R46-F3: capture canonical destination root for symlink-
        // escape prevention on every per-file write below.
        let canonical_dest_root = crate::path_safety::canonical_dest_root(dest_root).ok();

        // Derive checksum-mode flag from the spec for the PullSyncAck
        // mismatch check below. This is the only spec field the body
        // proper inspects (apart from sending the spec itself).
        let checksum_requested = spec.compare_mode == ComparisonMode::Checksum as i32;

        // Create channel for sending messages to server. Capacity is
        // small (32) — adequate because the gRPC stream is opened
        // BEFORE we push manifest entries, so the daemon is consuming
        // continuously and the channel never fills.
        //
        // History: for a long time this code pushed all manifest
        // entries into the channel BEFORE opening the gRPC stream.
        // For any local manifest with >30 entries that deadlocked at
        // entry 33 (channel full, no consumer because stream wasn't
        // open yet). Mirror noop on a populated dest hung silently.
        let (tx, rx) = tokio::sync::mpsc::channel::<ClientPullMessage>(32);

        // Open the bidirectional stream FIRST so the daemon starts
        // consuming our messages as we push them.
        let request_stream = ReceiverStream::new(rx);
        let mut response_stream = self
            .client
            .pull_sync(request_stream)
            .await
            .map_err(PullSyncError::negotiation)?
            .into_inner();

        tx.send(ClientPullMessage {
            payload: Some(client_pull_message::Payload::Spec(spec)),
        })
        .await
        .map_err(|_| eyre!("failed to send pull sync spec"))?;

        // Send local manifest. Send in a separate task so we can also
        // drive response_stream concurrently — for large manifests the
        // daemon may start emitting need-list / data-plane responses
        // before we finish enumerating, and we must not block sending
        // the manifest just because we haven't started reading
        // responses yet.
        let local_manifest_clone = local_manifest.clone();
        let tx_for_manifest = tx.clone();
        // R32-F2: AbortOnDrop so an outer cancellation aborts the
        // manifest send task instead of detaching it. In practice
        // the task self-terminates the moment the request stream is
        // dropped (send returns Err), but the explicit guard is
        // robust to future shape changes (e.g. a task that holds
        // resources beyond the channel).
        let manifest_send_task = AbortOnDrop::new(tokio::spawn(async move {
            for header in &local_manifest_clone {
                if tx_for_manifest
                    .send(ClientPullMessage {
                        payload: Some(client_pull_message::Payload::LocalFile(header.clone())),
                    })
                    .await
                    .is_err()
                {
                    return Err(eyre!("failed to send local file header"));
                }
            }
            tx_for_manifest
                .send(ClientPullMessage {
                    payload: Some(client_pull_message::Payload::ManifestDone(
                        // Pull side: client manifest is locally enumerated and
                        // always complete (we error early on enumeration
                        // failure rather than streaming a partial manifest).
                        // scan_complete is reused from the push-side wire
                        // shape for symmetry; daemon ignores it on pull.
                        ManifestComplete {
                            scan_complete: true,
                        },
                    )),
                })
                .await
                .map_err(|_| eyre!("failed to send manifest done"))?;
            Ok::<(), eyre::Report>(())
        }));

        let mut report = RemotePullReport::default();
        // (open handle, local dest path for error context, wire-relative
        // path for the FileComplete progress event — the contract in
        // transfer::progress requires the source-relative wire path,
        // never the absolute local one).
        let mut active_file: Option<(File, PathBuf, String)> = None;
        let mut active_shard: Option<InProgressShard> = None;
        // R32-F2: wrap the data-plane handle in AbortOnDrop so an
        // outer-future drop cancels the spawned TCP receiver instead
        // of detaching it.
        let mut data_plane_handle: Option<AbortOnDrop<Result<DataPlaneResult>>> = None;
        // ue-r2-2: ADD commands travel through here into the receiver
        // task (present only when the negotiation enabled resize).
        let mut data_plane_growth: Option<tokio::sync::mpsc::UnboundedSender<PullStreamAdd>> = None;
        // ue-r2-2 review (codex): the acked live count. Bounds
        // CUMULATIVE growth — per-command target checks alone would
        // let a hostile daemon replay ADDs past the ceiling this
        // client advertised.
        let mut data_plane_live: usize = 0;
        let mut files_to_delete = 0u64;
        let mut negotiation_complete = false;

        // audit-h3c slice 1: route the pull_sync_with_spec fallback
        // receive through the helper. This is the load-bearing site
        // GPT-12 / R3 H3 originally named for h3c; the bulk control +
        // data messages all flow through here. Slice 2 will install the
        // dynamic progress watchdog inside `recv_fallback_message` and
        // every site (this one plus :316, :484) inherits it

exec
/usr/bin/zsh -lc "rg -n \"struct RemotePullClient|impl RemotePullClient|pull_sync\\(|PullSyncOptions|PullSync\" crates/blit-core/src/remote crates/blit-daemon/src/service crates/blit-cli/src crates/blit-app/src | head -200" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer_session_e2e.rs:38:use blit_core::remote::pull::PullSyncOptions;
crates/blit-daemon/src/service/transfer_session_e2e.rs:617:        .pull_sync(
crates/blit-daemon/src/service/transfer_session_e2e.rs:620:            &PullSyncOptions::default(),
crates/blit-core/src/remote/pull.rs:27:pub enum PullSyncError {
crates/blit-core/src/remote/pull.rs:32:impl PullSyncError {
crates/blit-core/src/remote/pull.rs:50:impl std::fmt::Display for PullSyncError {
crates/blit-core/src/remote/pull.rs:59:impl std::error::Error for PullSyncError {}
crates/blit-core/src/remote/pull.rs:71:pub struct PullSyncOptions {
crates/blit-core/src/remote/pull.rs:129:    /// PullSyncAck. `None` means no ack arrived (legacy daemon or
crates/blit-core/src/remote/pull.rs:171:pub struct RemotePullClient {
crates/blit-core/src/remote/pull.rs:176:impl RemotePullClient {
crates/blit-core/src/remote/pull.rs:197:    /// Open a PullSync session with `spec`, immediately completing the
crates/blit-core/src/remote/pull.rs:201:    /// Pull RPC until its deletion; PullSync is the only pull wire).
crates/blit-core/src/remote/pull.rs:220:            .pull_sync(ReceiverStream::new(rx))
crates/blit-core/src/remote/pull.rs:245:    /// Minimal spec for the relay's PullSync sessions: no filter,
crates/blit-core/src/remote/pull.rs:347:    /// bytes. Rides a metadata-only PullSync session — ue-r2-1h's port
crates/blit-core/src/remote/pull.rs:398:                // Ack / PullSyncAck / ManifestBatch / FilesToDownload /
crates/blit-core/src/remote/pull.rs:409:    /// single-file `force_grpc` PullSync session — ue-r2-1h's port of
crates/blit-core/src/remote/pull.rs:424:    /// Build a `TransferOperationSpec` from CLI-style `PullSyncOptions`
crates/blit-core/src/remote/pull.rs:433:        options: &PullSyncOptions,
crates/blit-core/src/remote/pull.rs:519:    /// that build `PullSyncOptions`.
crates/blit-core/src/remote/pull.rs:520:    pub async fn pull_sync(
crates/blit-core/src/remote/pull.rs:524:        options: &PullSyncOptions,
crates/blit-core/src/remote/pull.rs:576:        // Derive checksum-mode flag from the spec for the PullSyncAck
crates/blit-core/src/remote/pull.rs:598:            .pull_sync(request_stream)
crates/blit-core/src/remote/pull.rs:600:            .map_err(PullSyncError::negotiation)?
crates/blit-core/src/remote/pull.rs:684:        // `PullSyncError::transfer` / `negotiation`, both of which
crates/blit-core/src/remote/pull.rs:685:        // format the tonic::Status to a String (see PullSyncError's
crates/blit-core/src/remote/pull.rs:697:                        PullSyncError::transfer(status)
crates/blit-core/src/remote/pull.rs:699:                        PullSyncError::negotiation(status)
crates/blit-core/src/remote/pull.rs:706:                    // Header acknowledged, continue (deprecated, use PullSyncAck)
crates/blit-core/src/remote/pull.rs:708:                Some(server_pull_message::Payload::PullSyncAck(ack)) => {
crates/blit-core/src/remote/pull.rs:716:                        return Err(PullSyncError::negotiation_message(
crates/blit-core/src/remote/pull.rs:1188:/// `AsyncRead` over a single-file PullSync session's control-stream
crates/blit-core/src/remote/pull.rs:1289:                        // Ack / PullSyncAck / ManifestBatch /
crates/blit-core/src/remote/pull.rs:2034:    //! `receive_data_plane_streams_owned` — the machinery the PullSync
crates/blit-core/src/remote/pull.rs:2324:        let opts = PullSyncOptions::default();
crates/blit-core/src/remote/pull.rs:2356:        let mut opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2363:        opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2370:        opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2377:        opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2390:        let opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2399:        let opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2408:        let opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2429:        let opts = PullSyncOptions {
crates/blit-core/src/remote/pull.rs:2456:        let opts = PullSyncOptions::default();
crates/blit-core/src/remote/pull.rs:2467:        let opts = PullSyncOptions::default();
crates/blit-core/src/remote/pull.rs:2478:        let opts = PullSyncOptions::default();
crates/blit-core/src/remote/pull.rs:2493:        let opts = PullSyncOptions {
crates/blit-app/src/transfers/remote.rs:7://!   tree and produces the `Vec<FileHeader>` that PullSync
crates/blit-app/src/transfers/remote.rs:13://!   [`PullSyncExecution`] / [`PullSyncOutcome`] /
crates/blit-app/src/transfers/remote.rs:59:use blit_core::remote::pull::{PullSyncOptions, RemotePullReport};
crates/blit-app/src/transfers/remote.rs:274:pub struct PullSyncExecution {
crates/blit-app/src/transfers/remote.rs:277:    pub options: PullSyncOptions,
crates/blit-app/src/transfers/remote.rs:283:/// Output of [`run_pull_sync`]. The PullSync handshake is done
crates/blit-app/src/transfers/remote.rs:292:pub struct PullSyncOutcome {
crates/blit-app/src/transfers/remote.rs:298:/// PullSync report + actual destination + (mirror-mode) purge
crates/blit-app/src/transfers/remote.rs:299:/// stats. Composed by the caller from [`PullSyncOutcome`] plus
crates/blit-app/src/transfers/remote.rs:307:/// Run the PullSync half of a remote pull: connect, enumerate
crates/blit-app/src/transfers/remote.rs:308:/// the local manifest, and run the PullSync handshake. Does
crates/blit-app/src/transfers/remote.rs:313:/// `progress` is borrowed for the duration of the PullSync RPC
crates/blit-app/src/transfers/remote.rs:318:/// let sync = run_pull_sync(execution, handle.as_ref()).await?;
crates/blit-app/src/transfers/remote.rs:330:pub async fn run_pull_sync(
crates/blit-app/src/transfers/remote.rs:331:    execution: PullSyncExecution,
crates/blit-app/src/transfers/remote.rs:333:) -> Result<PullSyncOutcome> {
crates/blit-app/src/transfers/remote.rs:343:        .pull_sync(
crates/blit-app/src/transfers/remote.rs:359:    Ok(PullSyncOutcome {
crates/blit-app/src/transfers/remote.rs:380:    outcome: &PullSyncOutcome,
crates/blit-app/src/transfers/remote.rs:532:    pub options: PullSyncOptions,
crates/blit-app/src/transfers/remote.rs:950:            options: PullSyncOptions::default(),
crates/blit-daemon/src/service/core.rs:345:    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
crates/blit-daemon/src/service/core.rs:618:    async fn pull_sync(
crates/blit-daemon/src/service/core.rs:621:    ) -> Result<Response<Self::PullSyncStream>, Status> {
crates/blit-daemon/src/service/core.rs:636:            ActiveJobKind::PullSync,
crates/blit-daemon/src/service/core.rs:644:        self.emit_transfer_started(&job, ActiveJobKind::PullSync, &peer, "", "");
crates/blit-daemon/src/service/core.rs:1834:    async fn cancel_job_ok_for_push_and_pull_sync() {
crates/blit-daemon/src/service/core.rs:1839:        for kind in [ActiveJobKind::Push, ActiveJobKind::PullSync] {
crates/blit-daemon/src/service/mod.rs:18:pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
crates/blit-daemon/src/service/pull_sync.rs:3://! This module implements the PullSync RPC which allows clients to send their
crates/blit-daemon/src/service/pull_sync.rs:11:use super::PullSyncSender;
crates/blit-daemon/src/service/pull_sync.rs:22:    PullSummary, PullSyncAck, ServerPullMessage, TransferOperationSpec,
crates/blit-daemon/src/service/pull_sync.rs:45:/// Handle a bidirectional PullSync stream.
crates/blit-daemon/src/service/pull_sync.rs:50:    tx: PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:475:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:479:        payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
crates/blit-daemon/src/service/pull_sync.rs:488:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:505:async fn send_file_header(tx: &PullSyncSender, header: FileHeader) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:513:async fn send_need_list(tx: &PullSyncSender, files: &[String]) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:523:async fn send_delete_list(tx: &PullSyncSender, paths: &[String]) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:583:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:606:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:652:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:1094:/// required the multistream pattern to live in PullSync before
crates/blit-daemon/src/service/pull_sync.rs:1235:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:1487:    tx: &PullSyncSender,
crates/blit-cli/src/transfers/remote.rs:11:    PullSyncExecution, PushExecution,
crates/blit-cli/src/transfers/remote.rs:13:use blit_core::remote::pull::PullSyncOptions;
crates/blit-cli/src/transfers/remote.rs:364:    let execution = PullSyncExecution {
crates/blit-cli/src/transfers/remote.rs:367:        options: PullSyncOptions {
crates/blit-cli/src/transfers/remote.rs:390:    //   1. PullSync RPC with progress monitor live.
crates/blit-cli/src/transfers/remote.rs:405:    let sync_outcome = run_pull_sync(execution, progress_handle.as_ref()).await?;
crates/blit-cli/src/transfers/remote.rs:433:// (`PullSyncExecution` / `PullSyncOutcome` /
crates/blit-daemon/src/service/delegated_pull.rs:22:use blit_core::remote::pull::{PullSyncError, RemotePullClient};
crates/blit-daemon/src/service/delegated_pull.rs:88:    // header-scan session shape for the relay's direct PullSync use
crates/blit-daemon/src/service/delegated_pull.rs:395:                .downcast_ref::<PullSyncError>()
crates/blit-daemon/src/service/delegated_pull.rs:396:                .is_some_and(PullSyncError::is_negotiation)
crates/blit-cli/src/transfers/mod.rs:392:    //     and for remote-source pull (PullSyncOptions.checksum is
crates/blit-cli/src/transfers/mod.rs:614:            // scan` — since ue-r2-1h a metadata-only PullSync
crates/blit-core/src/remote/transfer/grpc_fallback.rs:40://!   metadata-only PullSync scan).
crates/blit-app/src/admin/jobs.rs:214:        Ok(TransferKind::PullSync) => "pull_sync",
crates/blit-app/src/admin/jobs.rs:229:        assert_eq!(kind_label(TransferKind::PullSync as i32), "pull_sync");
crates/blit-cli/src/transfers/remote_remote_direct.rs:9:use blit_core::remote::pull::PullSyncOptions;
crates/blit-cli/src/transfers/remote_remote_direct.rs:93:    let options = PullSyncOptions {

exec
/usr/bin/zsh -lc "rg -n \"pull_sync|PullSync\" crates/blit-daemon/src/service -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer_session_e2e.rs:23://! landing + A/B parity vs old `pull_sync`, proving the one served RPC
crates/blit-daemon/src/service/transfer_session_e2e.rs:38:use blit_core::remote::pull::PullSyncOptions;
crates/blit-daemon/src/service/transfer_session_e2e.rs:609:    // Arm A: OLD pull_sync into a client-local dest.
crates/blit-daemon/src/service/transfer_session_e2e.rs:617:        .pull_sync(
crates/blit-daemon/src/service/transfer_session_e2e.rs:620:            &PullSyncOptions::default(),
crates/blit-daemon/src/service/core.rs:5:use super::pull_sync::handle_pull_sync_stream;
crates/blit-daemon/src/service/core.rs:345:    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
crates/blit-daemon/src/service/core.rs:618:    async fn pull_sync(
crates/blit-daemon/src/service/core.rs:621:    ) -> Result<Response<Self::PullSyncStream>, Status> {
crates/blit-daemon/src/service/core.rs:636:            ActiveJobKind::PullSync,
crates/blit-daemon/src/service/core.rs:644:        self.emit_transfer_started(&job, ActiveJobKind::PullSync, &peer, "", "");
crates/blit-daemon/src/service/core.rs:652:            // site above — pull_sync's enumerate+checksum collection
crates/blit-daemon/src/service/core.rs:658:                handle_pull_sync_stream(
crates/blit-daemon/src/service/core.rs:680:            metrics.log_completion("pull_sync", started.elapsed(), ok);
crates/blit-daemon/src/service/core.rs:697:        // the streaming RPCs (push, pull_sync), which register
crates/blit-daemon/src/service/core.rs:708:        // delegated_pull at dispatch time (unlike push/pull_sync).
crates/blit-daemon/src/service/core.rs:726:        // loop inside `pull_sync_with_spec`. Reports land on the
crates/blit-daemon/src/service/core.rs:747:        // handler's pull_sync_with_spec future is then dropped,
crates/blit-daemon/src/service/core.rs:1313:/// output `bool`), while `push` / `pull_sync` go through
crates/blit-daemon/src/service/core.rs:1350:/// w4-3: resolve a streaming transfer RPC's (`push` / `pull_sync`)
crates/blit-daemon/src/service/core.rs:1355:/// that disconnected during a send-free compute phase (pull_sync's
crates/blit-daemon/src/service/core.rs:1363:/// and pull_sync's payload feeder exits when its channel closes. An
crates/blit-daemon/src/service/core.rs:1381:///   `ActiveJobKind::supports_cancellation` on for push/pull_sync, so
crates/blit-daemon/src/service/core.rs:1462:/// `pull_sync` dispatchers. `delegated_pull` has its own shape
crates/blit-daemon/src/service/core.rs:1578:    /// peer. Pre-fix the push/pull_sync dispatchers bare-awaited the
crates/blit-daemon/src/service/core.rs:1600:    /// flipped `supports_cancellation` on for push/pull_sync.)
crates/blit-daemon/src/service/core.rs:1834:    async fn cancel_job_ok_for_push_and_pull_sync() {
crates/blit-daemon/src/service/core.rs:1839:        for kind in [ActiveJobKind::Push, ActiveJobKind::PullSync] {
crates/blit-daemon/src/service/mod.rs:4:mod pull_sync;
crates/blit-daemon/src/service/mod.rs:18:pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
crates/blit-daemon/src/service/pull_sync.rs:3://! This module implements the PullSync RPC which allows clients to send their
crates/blit-daemon/src/service/pull_sync.rs:11:use super::PullSyncSender;
crates/blit-daemon/src/service/pull_sync.rs:22:    PullSummary, PullSyncAck, ServerPullMessage, TransferOperationSpec,
crates/blit-daemon/src/service/pull_sync.rs:45:/// Handle a bidirectional PullSync stream.
crates/blit-daemon/src/service/pull_sync.rs:46:pub(crate) async fn handle_pull_sync_stream(
crates/blit-daemon/src/service/pull_sync.rs:50:    tx: PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:96:    send_pull_sync_ack(&tx, server_checksums_enabled).await?;
crates/blit-daemon/src/service/pull_sync.rs:131:    //   - mirror mode is on (pull_sync's delete-list builder
crates/blit-daemon/src/service/pull_sync.rs:294:    // ue-r2-1e: the daemon is the byte SENDER on pull_sync — one dial
crates/blit-daemon/src/service/pull_sync.rs:405:/// ue-r2-1g: stream count for the pull_sync full-file data plane. The
crates/blit-daemon/src/service/pull_sync.rs:474:async fn send_pull_sync_ack(
crates/blit-daemon/src/service/pull_sync.rs:475:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:479:        payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
crates/blit-daemon/src/service/pull_sync.rs:488:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:505:async fn send_file_header(tx: &PullSyncSender, header: FileHeader) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:513:async fn send_need_list(tx: &PullSyncSender, files: &[String]) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:523:async fn send_delete_list(tx: &PullSyncSender, paths: &[String]) -> Result<(), Status> {
crates/blit-daemon/src/service/pull_sync.rs:583:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:606:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:652:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:1094:/// required the multistream pattern to live in PullSync before
crates/blit-daemon/src/service/pull_sync.rs:1175:            // failure — UNAUTHENTICATED, matching what the pull_sync
crates/blit-daemon/src/service/pull_sync.rs:1235:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:1487:    tx: &PullSyncSender,
crates/blit-daemon/src/service/pull_sync.rs:1621:// at ue-r2-1h — pull_sync is their only consumer) ────────────────────
crates/blit-daemon/src/service/pull_sync.rs:1629:/// list so destructive callers (pull_sync's delete-list builder)
crates/blit-daemon/src/service/pull_sync.rs:1989:        // stream_count" — 1 on pull_sync.
crates/blit-daemon/src/service/delegated_pull.rs:22:use blit_core::remote::pull::{PullSyncError, RemotePullClient};
crates/blit-daemon/src/service/delegated_pull.rs:46:        // ue-r2-2: the delegated byte path IS `pull_sync_with_spec`,
crates/blit-daemon/src/service/delegated_pull.rs:74:/// boundary that push and pull_sync use (R30-F3). Catches bad
crates/blit-daemon/src/service/delegated_pull.rs:88:    // header-scan session shape for the relay's direct PullSync use
crates/blit-daemon/src/service/delegated_pull.rs:90:    // stream bare FileHeaders that this daemon's pull_sync client
crates/blit-daemon/src/service/delegated_pull.rs:118:/// `pull_sync_with_spec` flow.
crates/blit-daemon/src/service/delegated_pull.rs:218:    // NormalizedTransferOperation::from_spec boundary push/pull_sync
crates/blit-daemon/src/service/delegated_pull.rs:298:        // pull_sync_with_spec would bail loudly on the Discovery
crates/blit-daemon/src/service/delegated_pull.rs:349:    // src can decide what to send) and run pull_sync_with_spec. The
crates/blit-daemon/src/service/delegated_pull.rs:365:    // pull_sync_with_spec doesn't currently expose a streaming progress
crates/blit-daemon/src/service/delegated_pull.rs:376:    // Capture mirror_mode before moving `spec` into pull_sync_with_spec
crates/blit-daemon/src/service/delegated_pull.rs:380:        .pull_sync_with_spec(
crates/blit-daemon/src/service/delegated_pull.rs:391:            // phases. Preserve pull_sync_with_spec's typed
crates/blit-daemon/src/service/delegated_pull.rs:395:                .downcast_ref::<PullSyncError>()
crates/blit-daemon/src/service/delegated_pull.rs:396:                .is_some_and(PullSyncError::is_negotiation)
crates/blit-daemon/src/service/delegated_pull.rs:827:        // pull_sync client loop materializes as zero-byte files —
crates/blit-daemon/src/service/delegated_pull.rs:845:        // pull_sync. (R30-F3.)

codex
One possible cross-platform edge is endpoint path normalization, so I’m checking whether the new pull client inherits an existing push-session assumption or introduces a fresh mismatch against old `PullSync` request shaping.
exec
/usr/bin/zsh -lc "sed -n '1,260p' crates/blit-core/src/remote/endpoint.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
use std::path::{Path, PathBuf};

use eyre::{bail, eyre, Result};

/// Canonical remote path variants.
#[derive(Debug, Clone)]
pub enum RemotePath {
    /// Addressed via `server:/module/...`
    Module { module: String, rel_path: PathBuf },
    /// Addressed via `server://...` (default root/export)
    Root { rel_path: PathBuf },
    /// Discovery form (`server` or `server:port`)
    Discovery,
}

/// Parsed representation of a canonical remote endpoint.
#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    pub host: String,
    pub port: u16,
    pub path: RemotePath,
}

impl RemoteEndpoint {
    /// The one statement of blit's default daemon port (w7-6): the
    /// daemon's bind default, `blit scan`'s display elision, and the
    /// TUI's local-row fallback all reference this constant.
    pub const DEFAULT_PORT: u16 = 9031;

    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!("remote location cannot be empty");
        }

        match check_local_path(trimmed) {
            LocalPathCheck::IsLocal => {
                bail!("input appears to be a local path");
            }
            LocalPathCheck::RemoteWithBackslashes => {
                bail!(
                    "remote paths must use forward slashes, not backslashes.\n\
                     Example: server:/module/path or server://path\n\
                     Got: {}",
                    trimmed
                );
            }
            LocalPathCheck::NotLocal => {}
        }

        if let Some(idx) = trimmed.find("://") {
            // Root export (server://path)
            let host_port = &trimmed[..idx];
            let remainder = &trimmed[idx + 3..];
            let (host, port) = parse_host_port(host_port)?;
            let rel = normalize_relative_path_buf(remainder);
            return Ok(Self {
                host,
                port,
                path: RemotePath::Root { rel_path: rel },
            });
        }

        if let Some(idx) = trimmed.find(":/") {
            // Module export (server:/module/...)
            let host_port = &trimmed[..idx];
            let remainder = &trimmed[idx + 2..];
            let (host, port) = parse_host_port(host_port)?;

            let slash_idx = remainder.find('/').ok_or_else(|| {
                eyre!(
                    "module path must end with '/' (e.g., server:/module/ or server:/module/path)"
                )
            })?;

            let module = &remainder[..slash_idx];
            if module.is_empty() {
                bail!("module name cannot be empty; expected server:/module/...");
            }
            let rest = &remainder[slash_idx + 1..];
            let rel = normalize_relative_path_buf(rest);

            return Ok(Self {
                host,
                port,
                path: RemotePath::Module {
                    module: module.to_string(),
                    rel_path: rel,
                },
            });
        }

        // Discovery (server or server:port)
        let (host, port) = parse_host_port(trimmed)?;
        Ok(Self {
            host,
            port,
            path: RemotePath::Discovery,
        })
    }

    pub fn control_plane_uri(&self) -> String {
        // R58-F10: IPv6 literals must be bracketed in the URI's
        // authority component. The host field is stored bracket-
        // less (the parser strips them), so we re-bracket here.
        // A colon-containing host can only be IPv6 in our schema —
        // hostnames and IPv4 addresses never contain colons. Bare
        // `2001:db8::1:9031` is parsed by HTTP libraries as host
        // `2001` with garbage trailing, which is the bug.
        if self.host.contains(':') {
            format!("http://[{}]:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }

    /// Render just the `host[:port]` portion of this endpoint
    /// — drops the module / path suffix. Used by user-facing
    /// hints that reference the host as an argument
    /// (e.g. `blit jobs cancel <host> <id>`), where stitching
    /// the full canonical path back together would be wrong.
    /// Handles bracketed IPv6 via [`display_host`]; preserves
    /// non-default ports.
    pub fn host_port_display(&self) -> String {
        let host = display_host(&self.host);
        if self.port == Self::DEFAULT_PORT {
            host
        } else {
            format!("{}:{}", host, self.port)
        }
    }

    /// Display helper used by CLI for canonical formatting.
    pub fn display(&self) -> String {
        let host = display_host(&self.host);
        let host_port = if self.port == Self::DEFAULT_PORT {
            host
        } else {
            format!("{}:{}", host, self.port)
        };

        match &self.path {
            RemotePath::Module { module, rel_path } => {
                let mut path = format!("{}:/{}", host_port, module);
                if rel_path.as_os_str().is_empty() {
                    path.push('/');
                } else {
                    path.push('/');
                    path.push_str(&rel_path_to_string(rel_path));
                }
                path
            }
            RemotePath::Root { rel_path } => {
                if rel_path.as_os_str().is_empty() {
                    format!("{host_port}://")
                } else {
                    format!("{}://{}", host_port, rel_path_to_string(rel_path))
                }
            }
            RemotePath::Discovery => host_port,
        }
    }
}

fn parse_host_port(authority: &str) -> Result<(String, u16)> {
    if authority.is_empty() {
        bail!("remote location missing host");
    }

    if let Some(stripped) = authority.strip_prefix('[') {
        // IPv6 literal
        let closing = stripped
            .find(']')
            .ok_or_else(|| eyre!("unterminated IPv6 address: {}", authority))?;
        let host = &stripped[..closing];
        let remainder = &stripped[closing + 1..];
        let port = if let Some(port_str) = remainder.strip_prefix(':') {
            parse_port(port_str)?
        } else if remainder.is_empty() {
            RemoteEndpoint::DEFAULT_PORT
        } else {
            bail!("invalid host specification: {}", authority);
        };
        return Ok((host.to_string(), port));
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() {
            bail!("remote location missing host before ':'");
        }
        Ok((host.to_string(), parse_port(port)?))
    } else {
        Ok((authority.to_string(), RemoteEndpoint::DEFAULT_PORT))
    }
}

fn parse_port(raw: &str) -> Result<u16> {
    if raw.is_empty() {
        return Ok(RemoteEndpoint::DEFAULT_PORT);
    }
    raw.parse::<u16>()
        .map_err(|_| eyre!("invalid port '{}'", raw))
}

fn normalize_relative_path_buf(raw: &str) -> PathBuf {
    if raw.is_empty() {
        PathBuf::new()
    } else {
        let trimmed = raw.trim_start_matches('/');
        if trimmed.is_empty() {
            PathBuf::new()
        } else {
            PathBuf::from(trimmed)
        }
    }
}

fn rel_path_to_string(path: &Path) -> String {
    // Delegate to the canonical POSIX-form helper.
    crate::path_posix::relative_path_to_posix(path)
}

fn display_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{}]", host)
    } else {
        host.to_string()
    }
}

/// Result of checking if input looks like a local path
enum LocalPathCheck {
    /// Definitely a local path
    IsLocal,
    /// Looks like a remote path with backslashes - user error
    RemoteWithBackslashes,
    /// Not a local path
    NotLocal,
}

fn check_local_path(input: &str) -> LocalPathCheck {
    if input.is_empty() {
        return LocalPathCheck::NotLocal;
    }

    let first = input.chars().next().expect("non-empty input checked above");
    if matches!(first, '.' | '/' | '\\' | '~') {
        return LocalPathCheck::IsLocal;
    }

    // UNC paths: \\server\share or //server/share
    if input.starts_with("//") || input.starts_with("\\\\") {
        return LocalPathCheck::IsLocal;
    }

    // Windows drive letter: C:\path or C:/path - check BEFORE backslash handling
    if input.len() >= 3 {
        let mut chars = input.chars();
        let drive = chars.next().expect("len >= 3 checked above");
        if drive.is_ascii_alphabetic() {

exec
/usr/bin/zsh -lc "rg -n \"normalize_for_request|relative_path_to_posix|to_string_lossy\\(\\)\\.into_owned\\(\\)|RemotePath::Module|RemotePath::Root\" crates/blit-core/src/remote crates/blit-cli/src crates/blit-app/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/src/rm.rs:35:        .map(|component| component.to_string_lossy().into_owned())
crates/blit-app/src/transfers/remote.rs:481:                RemotePath::Module { rel_path, .. } => rel_path.clone(),
crates/blit-app/src/transfers/remote.rs:482:                RemotePath::Root { rel_path } => rel_path.clone(),
crates/blit-app/src/transfers/remote.rs:632:        RemotePath::Module { module, rel_path } => {
crates/blit-app/src/transfers/remote.rs:633:            Ok((module.clone(), normalize_for_request(rel_path)))
crates/blit-app/src/transfers/remote.rs:635:        RemotePath::Root { rel_path } => Ok((String::new(), normalize_for_request(rel_path))),
crates/blit-app/src/transfers/remote.rs:642:fn normalize_for_request(path: &Path) -> String {
crates/blit-app/src/transfers/remote.rs:942:            path: RemotePath::Module {
crates/blit-app/src/transfers/remote.rs:1039:        let dst = delegated_endpoint(RemotePath::Module {
crates/blit-app/src/transfers/remote.rs:1050:        let dst = delegated_endpoint(RemotePath::Module {
crates/blit-cli/src/profile.rs:18:            "predictor_path": report.predictor_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
crates/blit-app/src/transfers/resolution.rs:83:            RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
crates/blit-app/src/transfers/resolution.rs:160:                RemotePath::Module { module, rel_path } => RemotePath::Module {
crates/blit-app/src/transfers/resolution.rs:164:                RemotePath::Root { rel_path } => RemotePath::Root {
crates/blit-app/src/transfers/resolution.rs:243:        let dst_raw = tmp.path().to_string_lossy().into_owned();
crates/blit-app/src/transfers/resolution.rs:282:        let dst_raw = tmp.path().to_string_lossy().into_owned();
crates/blit-app/src/transfers/resolution.rs:296:        let dst_raw = tmp.path().to_string_lossy().into_owned();
crates/blit-app/src/transfers/resolution.rs:309:        let dst_raw = tmp.path().to_string_lossy().into_owned();
crates/blit-app/src/transfers/resolution.rs:336:            path: RemotePath::Module {
crates/blit-app/src/transfers/resolution.rs:344:                RemotePath::Module { rel_path, .. } => {
crates/blit-app/src/transfers/resolution.rs:360:            path: RemotePath::Module {
crates/blit-app/src/transfers/resolution.rs:368:                RemotePath::Module { rel_path, .. } => {
crates/blit-app/src/transfers/resolution.rs:384:            path: RemotePath::Module {
crates/blit-app/src/transfers/resolution.rs:390:        let dst_raw = tmp.path().to_string_lossy().into_owned();
crates/blit-cli/src/ls.rs:62:        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
crates/blit-cli/src/ls.rs:63:        RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
crates/blit-app/src/transfers/filter.rs:117:            .map(|p| p.to_string_lossy().into_owned())
crates/blit-core/src/remote/pull.rs:256:            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.join(path)),
crates/blit-core/src/remote/pull.rs:257:            RemotePath::Root { rel_path } => (String::new(), rel_path.join(path)),
crates/blit-core/src/remote/pull.rs:263:            source_path: normalize_for_request(&rel_path),
crates/blit-core/src/remote/pull.rs:436:            RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
crates/blit-core/src/remote/pull.rs:437:            RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
crates/blit-core/src/remote/pull.rs:446:            normalize_for_request(&rel_path)
crates/blit-core/src/remote/pull.rs:2020:fn normalize_for_request(path: &Path) -> String {
crates/blit-core/src/remote/pull.rs:2311:        endpoint_with_path(RemotePath::Module {
crates/blit-core/src/remote/pull.rs:2451:        // RemotePath::Root → module is empty string, source_path is
crates/blit-core/src/remote/pull.rs:2453:        let ep = endpoint_with_path(RemotePath::Root {
crates/blit-app/src/transfers/dispatch.rs:153:            path: RemotePath::Module {
crates/blit-core/src/remote/endpoint.rs:60:                path: RemotePath::Root { rel_path: rel },
crates/blit-core/src/remote/endpoint.rs:86:                path: RemotePath::Module {
crates/blit-core/src/remote/endpoint.rs:143:            RemotePath::Module { module, rel_path } => {
crates/blit-core/src/remote/endpoint.rs:153:            RemotePath::Root { rel_path } => {
crates/blit-core/src/remote/endpoint.rs:220:    crate::path_posix::relative_path_to_posix(path)
crates/blit-core/src/remote/endpoint.rs:307:            RemotePath::Module {
crates/blit-core/src/remote/endpoint.rs:323:            RemotePath::Module {
crates/blit-core/src/remote/endpoint.rs:338:            RemotePath::Root { ref rel_path } => {
crates/blit-core/src/remote/endpoint.rs:363:            RemotePath::Module {
crates/blit-app/src/endpoints.rs:88:        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
crates/blit-app/src/endpoints.rs:98:        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
crates/blit-app/src/endpoints.rs:177:        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
crates/blit-app/src/endpoints.rs:178:        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
crates/blit-app/src/endpoints.rs:191:    blit_core::path_posix::relative_path_to_posix(path)
crates/blit-app/src/diagnostics/dump.rs:26:                RemotePath::Module { module, rel_path } => (
crates/blit-app/src/diagnostics/dump.rs:31:                RemotePath::Root { rel_path } => {
crates/blit-app/src/check.rs:100:                    .push(rel.to_string_lossy().into_owned());
crates/blit-app/src/check.rs:107:                            path: rel.to_string_lossy().into_owned(),
crates/blit-app/src/check.rs:118:                        path: rel.to_string_lossy().into_owned(),
crates/blit-app/src/check.rs:134:                            path: rel.to_string_lossy().into_owned(),
crates/blit-app/src/check.rs:141:                            .push((rel.to_string_lossy().into_owned(), format!("{e:#}"))),
crates/blit-app/src/check.rs:149:                            path: rel.to_string_lossy().into_owned(),
crates/blit-app/src/check.rs:168:                    .push(dst_entry.relative_path.to_string_lossy().into_owned());
crates/blit-cli/src/transfers/mod.rs:568:                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
crates/blit-cli/src/transfers/mod.rs:569:                    rel_path.to_string_lossy().into_owned()
crates/blit-cli/src/transfers/mod.rs:653:                RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
crates/blit-cli/src/transfers/mod.rs:654:                    rel_path.to_string_lossy().into_owned()
crates/blit-cli/src/transfers/mod.rs:689:            source: src.to_string_lossy().into_owned(),
crates/blit-cli/src/transfers/mod.rs:690:            destination: dest.to_string_lossy().into_owned(),
crates/blit-cli/src/transfers/mod.rs:738:            source: src.to_string_lossy().into_owned(),
crates/blit-cli/src/transfers/mod.rs:739:            destination: dest.to_string_lossy().into_owned(),
crates/blit-core/src/remote/push/client/mod.rs:677:                        .map(|p| p.to_string_lossy().into_owned())
crates/blit-app/src/admin/rm.rs:50:        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
crates/blit-app/src/admin/rm.rs:51:        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
crates/blit-app/src/admin/ls.rs:112:                name: entry.file_name().to_string_lossy().into_owned(),
crates/blit-app/src/admin/ls.rs:122:            .map(|n| n.to_string_lossy().into_owned())
crates/blit-core/src/remote/push/client/helpers.rs:79:    crate::path_posix::relative_path_to_posix(path)
crates/blit-core/src/remote/push/client/helpers.rs:314:        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
crates/blit-core/src/remote/push/client/helpers.rs:315:        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
crates/blit-core/src/remote/transfer/session_client.rs:189:        RemotePath::Module { module, rel_path } => {
crates/blit-core/src/remote/transfer/session_client.rs:190:            Ok((module.clone(), rel_path.to_string_lossy().into_owned()))
crates/blit-core/src/remote/transfer/session_client.rs:192:        RemotePath::Root { rel_path } => {
crates/blit-core/src/remote/transfer/session_client.rs:193:            Ok((String::new(), rel_path.to_string_lossy().into_owned()))
crates/blit-core/src/remote/transfer/payload.rs:207:    crate::path_posix::relative_path_to_posix(path)
crates/blit-core/src/remote/transfer/tar_safety.rs:138:        let rel_string = crate::path_posix::relative_path_to_posix(&raw_path);

exec
/usr/bin/zsh -lc "sed -n '220,360p' crates/blit-app/src/transfers/remote.rs" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '340,430p' crates/blit-cli/src/transfers/remote.rs" in /home/michael/dev/Blit
 succeeded in 0ms:

async fn run_remote_pull_transfer_inner(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    require_complete_scan: bool,
    defer_output: bool,
) -> Result<DeferredPullState> {
    // Filter parity (Step 4B): build the wire FilterSpec here and
    // ship it on TransferOperationSpec. The daemon applies the same
    // rules during its source enumeration, so the file set the daemon
    // sees matches what `--exclude/--include/--min-size/...` would
    // have produced for an equivalent push.
    let filter_spec = super::build_filter_spec(args)?;

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress final progress line on move
    );

    let execution = PullSyncExecution {
        remote: remote.clone(),
        dest_root: dest_root.to_path_buf(),
        options: PullSyncOptions {
            force_grpc: args.force_grpc,
            mirror_mode,
            delete_all_scope: args.delete_scope_all(),
            filter: Some(filter_spec),
            size_only: args.size_only,
            ignore_times: args.ignore_times,
            ignore_existing: args.ignore_existing,
            force: args.force,
            checksum: args.checksum,
            resume: args.resume,
            block_size: 0, // Use default (1 MiB)
            // R49-F2: move arms set this true so the daemon refuses
            // partial source scans before we delete the remote source.
            require_complete_scan,
        },
        compute_checksums: args.checksum,
        mirror_mode,
        remote_label: format_remote_endpoint(&remote),
    };

    // Lifecycle (round-2 fix for a0-pull-execution):
    //
    //   1. PullSync RPC with progress monitor live.
    //   2. Tear down progress channel + drain monitor task.
    //   3. Apply mirror-purge in the now-quiet state.
    //   4. Print summary (or defer to the move caller).
    //
    // Round-1 bundled steps 1 and 3 into a single library call,
    // which kept the monitor alive through purge and let stale
    // [progress] ticks emit during destructive cleanup. The
    // library now exposes the two halves separately so the CLI
    // (and TUI) can place the lifecycle boundary at step 2.
    //
    // R53-F1 (`suppress_final_line`) and R46-F6 (purge stats in
    // the same JSON document as the report) both still hold —
    // R46-F6 is about ordering relative to *printing*, which
    // still happens at the very end below.
    let sync_outcome = run_pull_sync(execution, progress_handle.as_ref()).await?;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let mirror_purge_stats = apply_pull_mirror_purge(&sync_outcome, mirror_mode).await?;

    let state = PullExecutionOutcome {
        report: sync_outcome.report,
        actual_dest: sync_outcome.actual_dest,
        mirror_purge_stats,
    };

    // R51-F4: when deferred, skip the inline print. The caller
    // (move) prints via `print_deferred_pull_result` after the
    // source-delete step succeeds — so a post-transfer failure
    // never leaves a success-looking JSON document on stdout.
    if !defer_output {
        print_deferred_pull_result(args, &state);
    }

    Ok(state)
}


 succeeded in 0ms:
    })?;

    for rel in relative_paths {
        let target = safe_join_contained(&canonical, dest_root, rel).map_err(|e| {
            eyre!(
                "daemon delete list contained unsafe path '{}': {:#}",
                rel,
                e
            )
        })?;
        // safe_join("") returns dest_root itself; we never delete the
        // destination root.
        if target == dest_root {
            bail!("daemon delete list referenced the destination root itself");
        }
        match tokio::fs::remove_file(&target).await {
            Ok(()) => {
                stats.files_deleted += 1;
                let mut p = target.parent();
                while let Some(parent) = p {
                    if parent == dest_root {
                        break;
                    }
                    candidate_parents.insert(parent.to_path_buf());
                    p = parent.parent();
                }
            }
            // Already gone is fine; daemon's view may lag behind.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(eyre!("failed to delete {}: {}", target.display(), e));
            }
        }
    }

    // Prune empty directories deepest-first.
    let mut dirs: Vec<_> = candidate_parents.into_iter().collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in dirs {
        if tokio::fs::remove_dir(&dir).await.is_ok() {
            stats.dirs_deleted += 1;
        }
    }
    Ok(stats)
}

/// Inputs for [`run_pull_sync`]. Primitive fields only — no
/// clap, no presentation types — so the CLI and the future TUI
/// can both build it without sharing a dependency.
///
/// `remote_label` is the human-readable string used in error
/// context (e.g. `pulling from <label> into <dest>`). The CLI
/// passes `format_remote_endpoint(&remote)`; the TUI passes
/// whatever string it shows the user in the picker.
pub struct PullSyncExecution {
    pub remote: RemoteEndpoint,
    pub dest_root: PathBuf,
    pub options: PullSyncOptions,
    pub compute_checksums: bool,
    pub mirror_mode: bool,
    pub remote_label: String,
}

/// Output of [`run_pull_sync`]. The PullSync handshake is done
/// and the daemon's report (including any mirror-mode delete
/// list) is in hand, but no destination filesystem mutation
/// has happened yet. The caller is expected to tear down its
/// progress channel here and then call
/// [`apply_pull_mirror_purge`] to run the destructive half of
/// the flow — that ordering is the round-2 fix for the
/// behavior regression where purge ran while the progress
/// monitor was still alive.
pub struct PullSyncOutcome {
    pub report: RemotePullReport,
    pub actual_dest: PathBuf,
}

/// Full post-pull state for the CLI printer / TUI summary —
/// PullSync report + actual destination + (mirror-mode) purge
/// stats. Composed by the caller from [`PullSyncOutcome`] plus
/// the result of [`apply_pull_mirror_purge`].
pub struct PullExecutionOutcome {
    pub report: RemotePullReport,
    pub actual_dest: PathBuf,
    pub mirror_purge_stats: Option<LocalPurgeStats>,
}

/// Run the PullSync half of a remote pull: connect, enumerate
/// the local manifest, and run the PullSync handshake. Does
/// **not** apply any mirror-mode delete list — that's
/// [`apply_pull_mirror_purge`], called by the caller after it
/// has had a chance to tear down the progress channel.
///
/// `progress` is borrowed for the duration of the PullSync RPC
/// only. The split exists so the caller can run the lifecycle:
///
/// ```text
/// let (handle, task) = spawn_progress_monitor(...);
/// let sync = run_pull_sync(execution, handle.as_ref()).await?;
/// drop(handle);
/// if let Some(t) = task { let _ = t.await; }
/// let purge = apply_pull_mirror_purge(&sync, mirror_mode).await?;
/// ```
///
/// Round 2 of `a0-pull-execution` introduced this split. Round
/// 1 had a single `run_remote_pull` that did pull_sync **and**
/// purge before returning, which forced the progress monitor
/// to stay alive across the (potentially long) purge — a
/// regression vs the pre-Phase-5 CLI lifecycle that the
/// reviewer caught.
pub async fn run_pull_sync(
    execution: PullSyncExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PullSyncOutcome> {
    let mut client = RemotePullClient::connect(execution.remote.clone())
        .await
        .with_context(|| format!("connecting to {}", execution.remote.control_plane_uri()))?;

    let actual_dest = execution.dest_root;
    let local_manifest =
        enumerate_local_manifest(&actual_dest, execution.compute_checksums).await?;

    let report = client
        .pull_sync(
            &actual_dest,
            local_manifest,
            &execution.options,
            execution.mirror_mode,
            progress,
        )
        .await
        .with_context(|| {
            format!(
                "pulling from {} into {}",
                execution.remote_label,
                actual_dest.display()
            )
        })?;

    Ok(PullSyncOutcome {
        report,

codex
The old pull path normalizes remote paths into POSIX wire strings; the new session helper still uses the raw `PathBuf` display string. I’m checking whether that helper predates this slice or whether pull just expanded the same bug surface.
exec
/usr/bin/zsh -lc 'git show 84be1cc''^:crates/blit-core/src/remote/transfer/session_client.rs | nl -ba | sed -n '"'1,170p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Client-side entry for initiating a unified transfer session as the
     2	//! SOURCE role (otp-4a).
     3	//!
     4	//! Builds a gRPC-backed [`FrameTransport`] over `BlitClient::transfer`
     5	//! and runs [`run_source`], so a CLI push becomes "open the Transfer
     6	//! RPC, declare SOURCE, stream the manifest + payloads." This is the
     7	//! push-equivalent on the unified path; the daemon answers by running
     8	//! `run_destination` as the Responder.
     9	//!
    10	//! Not yet wired to CLI verbs — the verbs keep riding the old push
    11	//! path until the otp-10 cutover; today the parity tests drive this.
    12	//! otp-4a uses the in-stream byte carrier only (`in_stream_bytes`);
    13	//! the TCP data plane lands at otp-4b.
    14	
    15	use std::sync::Arc;
    16	use std::time::Duration;
    17	
    18	use eyre::{eyre, Result};
    19	use tokio::sync::mpsc;
    20	use tokio_stream::wrappers::ReceiverStream;
    21	use tonic::transport::{Channel, Endpoint};
    22	
    23	use crate::generated::blit_client::BlitClient;
    24	use crate::generated::{ComparisonMode, SessionOpen, TransferRole, TransferSummary};
    25	use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
    26	use crate::remote::transfer::source::TransferSource;
    27	use crate::transfer_plan::PlanOptions;
    28	use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
    29	use crate::transfer_session::{run_source, HelloConfig, SessionEndpoint, SourceSessionConfig};
    30	
    31	/// The push-shaped subset of session options otp-4a/4b supports. Mirror,
    32	/// filters, and resume are refused at OPEN until their slices land
    33	/// (otp-6/otp-7), so they are intentionally absent here.
    34	pub struct PushSessionOptions {
    35	    pub compare_mode: ComparisonMode,
    36	    pub ignore_existing: bool,
    37	    pub require_complete_scan: bool,
    38	    pub plan_options: PlanOptions,
    39	    /// Force the in-stream byte carrier instead of the TCP data plane
    40	    /// (otp-4b). Default `false` = the responder grants a data plane and
    41	    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
    42	    /// data-plane fallback (`--force-grpc`-shaped).
    43	    pub in_stream_bytes: bool,
    44	}
    45	
    46	impl Default for PushSessionOptions {
    47	    fn default() -> Self {
    48	        Self {
    49	            compare_mode: ComparisonMode::SizeMtime,
    50	            ignore_existing: false,
    51	            require_complete_scan: false,
    52	            plan_options: PlanOptions::default(),
    53	            in_stream_bytes: false,
    54	        }
    55	    }
    56	}
    57	
    58	/// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
    59	/// session pushing `source`'s tree into the endpoint's module/path.
    60	/// Returns the destination-computed [`TransferSummary`] (contract:
    61	/// DESTINATION is the scorer).
    62	pub async fn run_push_session(
    63	    endpoint: &RemoteEndpoint,
    64	    source: Arc<dyn TransferSource>,
    65	    options: PushSessionOptions,
    66	) -> Result<TransferSummary> {
    67	    // The responder resolves module→root; the initiator's own local
    68	    // path never crosses the wire (contract §SessionOpen). Empty module
    69	    // targets the daemon's default root export.
    70	    let (module, path) = match &endpoint.path {
    71	        RemotePath::Module { module, rel_path } => {
    72	            (module.clone(), rel_path.to_string_lossy().into_owned())
    73	        }
    74	        RemotePath::Root { rel_path } => (String::new(), rel_path.to_string_lossy().into_owned()),
    75	        RemotePath::Discovery => {
    76	            return Err(eyre!(
    77	                "a transfer session needs a resolved module or root endpoint, not a discovery form"
    78	            ));
    79	        }
    80	    };
    81	
    82	    let mut client = connect_transfer_client(endpoint).await?;
    83	
    84	    let open = SessionOpen {
    85	        initiator_role: TransferRole::Source as i32,
    86	        module,
    87	        path,
    88	        compare_mode: options.compare_mode as i32,
    89	        ignore_existing: options.ignore_existing,
    90	        require_complete_scan: options.require_complete_scan,
    91	        // otp-4b: default to the TCP data plane; the responder grants it
    92	        // in SessionAccept unless this asks for the in-stream fallback.
    93	        in_stream_bytes: options.in_stream_bytes,
    94	        ..Default::default()
    95	    };
    96	
    97	    // Open the bidi RPC: the request stream is fed by `out_tx`, the
    98	    // response stream is the inbound half. The handler returns its
    99	    // response stream immediately (it spawns the session), so this
   100	    // await resolves before any frame flows — no deadlock.
   101	    let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
   102	    let inbound = client
   103	        .transfer(ReceiverStream::new(out_rx))
   104	        .await
   105	        .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
   106	        .into_inner();
   107	    let transport = grpc_client_transport(out_tx, inbound);
   108	
   109	    let cfg = SourceSessionConfig {
   110	        hello: HelloConfig::default(),
   111	        endpoint: SessionEndpoint::initiator(open),
   112	        plan_options: options.plan_options,
   113	        // The initiator dials the data plane on the same host it reached
   114	        // the control plane on (contract §Transport: initiator dials).
   115	        data_plane_host: Some(endpoint.host.clone()),
   116	    };
   117	    run_source(cfg, transport, source).await
   118	}
   119	
   120	/// Build a `BlitClient` over `endpoint`'s control-plane URI with the
   121	/// same bounded-connect policy `RemotePushClient::connect` uses.
   122	async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
   123	    let uri = endpoint.control_plane_uri();
   124	    let conn = Endpoint::from_shared(uri.clone())
   125	        .map_err(|e| eyre!("invalid endpoint uri {uri}: {e}"))?
   126	        .connect_timeout(Duration::from_secs(30));
   127	    let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
   128	        .await
   129	        .map_err(|_| eyre!("timed out connecting to {uri}"))?
   130	        .map_err(|e| eyre!("connecting to {uri}: {e}"))?;
   131	    Ok(BlitClient::new(channel))
   132	}

exec
/usr/bin/zsh -lc "sed -n '2000,2045p' crates/blit-core/src/remote/pull.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
             unavailable for {:?})",
            dest_root.display(),
            relative_path
        );
    }
    Ok(target)
}

/// Validate a wire-supplied relative path coming from the daemon.
///
/// Thin wrapper over `crate::path_safety::validate_wire_path` that
/// preserves the historical "server returned ..." error prefix so log
/// scrapers continue to find familiar messages. All actual policy
/// (rejecting absolute paths, `..`, Windows drive prefixes, UNC, etc.)
/// lives in the shared module — this is just the call site.
fn sanitize_relative_path(raw: &str) -> Result<PathBuf> {
    crate::path_safety::validate_wire_path(raw)
        .map_err(|e| eyre::eyre!("server returned unsafe path {:?}: {}", raw, e))
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[cfg(test)]
mod multi_stream_receive_tests {
    //! ue-r2-1g: pins the multistream fan-in semantics of
    //! `receive_data_plane_streams_owned` — the machinery the PullSync
    //! daemon now drives with `stream_count > 1`. Fail-whole is the
    //! contract (MULTISTREAM_PULL "per-stream failure" criterion): one
    //! stream dying must fail the entire receive deterministically,
    //! never silently drop that stream's files.
    //!
    //! `AbortOnDrop`'s own contract (drop-without-consume aborts,
    //! join-return-clears-drop, cancellation-during-join still
    //! aborts) is pinned generically in
    //! `crate::remote::transfer::abort_on_drop::tests` (hoisted
    //! there under w4-1); `cancellation_aborts_all_stream_workers`
    //! below is this module's integration point with that wrapper.

exec
/usr/bin/zsh -lc "sed -n '620,655p' crates/blit-app/src/transfers/remote.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
        }
        DelegatedPullPhase::Unknown => eyre!("delegated transfer failed: {message}"),
    }
}

/// Extract the `(module, destination_path)` pair the
/// `DelegatedPullRequest` needs from a parsed
/// [`RemoteEndpoint`]. Errors on `RemotePath::Discovery` —
/// remote destinations always require an explicit module or
/// root.
pub fn destination_spec_fields(dst: &RemoteEndpoint) -> Result<(String, String)> {
    match &dst.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), normalize_for_request(rel_path)))
        }
        RemotePath::Root { rel_path } => Ok((String::new(), normalize_for_request(rel_path))),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Run a delegated remote→remote pull: build the request,
/// connect to the destination's `BlitClient`, stream the
/// payload, and return the destination's summary.

exec
/usr/bin/zsh -lc "sed -n '300,390p' crates/blit-core/src/remote/endpoint.rs" in /home/michael/dev/Blit
 succeeded in 0ms:

    #[test]
    fn parses_module_root() {
        let ep = RemoteEndpoint::parse("example.com:/media/").unwrap();
        assert_eq!(ep.host, "example.com");
        assert_eq!(ep.port, RemoteEndpoint::DEFAULT_PORT);
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "media");
                assert!(rel_path.as_os_str().is_empty());
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn parses_module_with_subpath() {
        let ep = RemoteEndpoint::parse("example.com:9000:/data/projects/foo").unwrap();
        assert_eq!(ep.port, 9000);
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "data");
                assert_eq!(rel_path_to_string(rel_path), "projects/foo");
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn parses_root_path() {
        let ep = RemoteEndpoint::parse("example.com://backups").unwrap();
        match ep.path {
            RemotePath::Root { ref rel_path } => {
                assert_eq!(rel_path_to_string(rel_path), "backups");
            }
            _ => panic!("expected root path"),
        }
    }

    #[test]
    fn parses_discovery_host_only() {
        let ep = RemoteEndpoint::parse("example.com").unwrap();
        matches!(ep.path, RemotePath::Discovery);
    }

    #[test]
    fn parses_discovery_with_port() {
        let ep = RemoteEndpoint::parse("example.com:9130").unwrap();
        assert_eq!(ep.port, 9130);
        matches!(ep.path, RemotePath::Discovery);
    }

    #[test]
    fn parses_ipv6_module() {
        let ep = RemoteEndpoint::parse("[2001:db8::1]:/share/").unwrap();
        assert_eq!(ep.host, "2001:db8::1");
        match ep.path {
            RemotePath::Module {
                ref module,
                ref rel_path,
            } => {
                assert_eq!(module, "share");
                assert!(rel_path.as_os_str().is_empty());
            }
            _ => panic!("expected module path"),
        }
    }

    #[test]
    fn errors_on_missing_module_slash() {
        assert!(RemoteEndpoint::parse("example.com:/module").is_err());
    }

    #[test]
    fn errors_on_backslash_with_helpful_message() {
        let result = RemoteEndpoint::parse(r"server:\module\path");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("forward slashes"),
            "error should mention forward slashes, got: {}",
            err
        );
    }


codex
So far I don’t see the establish/body factoring changing the otp-4 push sequence: the pre-commit blocks were moved into `exchange_hello`, `responder_finish`, `drive_source`, and `drive_destination` without semantic drift. I’m checking cancellation/job handling around the daemon wrapper now because that’s the highest-risk part of using the source body as a responder.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '600,760p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   600	            // observe the transfer already drained from
   601	            // active[] and present in recent[] — the event
   602	            // signals reconcilable state, not "about to drain."
   603	            let finished_event = build_transfer_finished_event(&job, ok, err_msg.as_deref());
   604	            drop(job);
   605	            // §3.1 followup: drop the active-transfer guard BEFORE the
   606	            // completion log so `active=N` reflects state AFTER the
   607	            // just-finished RPC is removed from the gauge. Pre-fix
   608	            // a single-transfer log showed `active=1`, which is
   609	            // misleading for an end-of-RPC summary.
   610	            drop(guard);
   611	            let _ = events_tx.send(finished_event);
   612	            metrics.log_completion("push", started.elapsed(), ok);
   613	        });
   614	
   615	        Ok(Response::new(ReceiverStream::new(rx)))
   616	    }
   617	
   618	    async fn pull_sync(
   619	        &self,
   620	        request: Request<Streaming<ClientPullMessage>>,
   621	    ) -> Result<Response<Self::PullSyncStream>, Status> {
   622	        let peer = peer_addr_string(&request);
   623	        let modules = Arc::clone(&self.modules);
   624	        let (tx, rx) = mpsc::channel(32);
   625	        let stream = request.into_inner();
   626	        let force_grpc_data = self.force_grpc_data;
   627	        let default_root = self.default_root.clone();
   628	        let server_checksums_enabled = self.server_checksums_enabled;
   629	        let metrics = Arc::clone(&self.metrics);
   630	        metrics.inc_pull();
   631	        let guard = Arc::clone(&metrics).enter_transfer();
   632	        // Same shape as `push` above: module + path arrive in
   633	        // the first stream frame; handler calls
   634	        // `job.set_endpoint(...)` after parsing the spec.
   635	        let job = self.active_jobs.register(
   636	            ActiveJobKind::PullSync,
   637	            peer.clone(),
   638	            String::new(),
   639	            String::new(),
   640	        );
   641	        // Subscribe event with empty module/path — same caveat
   642	        // as the push site above. Subscribers reconcile via
   643	        // GetState.active[].
   644	        self.emit_transfer_started(&job, ActiveJobKind::PullSync, &peer, "", "");
   645	        let started = std::time::Instant::now();
   646	        let events_tx = self.events_tx();
   647	
   648	        tokio::spawn(async move {
   649	            let guard = guard;
   650	            let job = job;
   651	            // w4-3: same handler-vs-hangup-vs-cancel race as the push
   652	            // site above — pull_sync's enumerate+checksum collection
   653	            // is the longest send-free compute window of the three
   654	            // transfer RPCs, so it was the most exposed to running to
   655	            // completion for a client that had already disconnected.
   656	            let cancel_token = job.cancellation_token().clone();
   657	            let (ok, err_msg) = resolve_streaming_outcome(
   658	                handle_pull_sync_stream(
   659	                    modules,
   660	                    default_root,
   661	                    stream,
   662	                    tx.clone(),
   663	                    force_grpc_data,
   664	                    server_checksums_enabled,
   665	                    &job,
   666	                ),
   667	                &tx,
   668	                &cancel_token,
   669	                &metrics,
   670	            )
   671	            .await;
   672	            job.record_outcome(ok, err_msg.clone());
   673	            // c-3 round 2: same ordering as push/pull — build,
   674	            // drain, then broadcast so subscribers can race
   675	            // GetState and see reconcilable state.
   676	            let finished_event = build_transfer_finished_event(&job, ok, err_msg.as_deref());
   677	            drop(guard);
   678	            drop(job);
   679	            let _ = events_tx.send(finished_event);
   680	            metrics.log_completion("pull_sync", started.elapsed(), ok);
   681	        });
   682	
   683	        Ok(Response::new(ReceiverStream::new(rx)))
   684	    }
   685	
   686	    async fn delegated_pull(
   687	        &self,
   688	        request: Request<DelegatedPullRequest>,
   689	    ) -> Result<Response<Self::DelegatedPullStream>, Status> {
   690	        let peer = peer_addr_string(&request);
   691	        let req = request.into_inner();
   692	        // ActiveJobs row mirrors the metrics gauge — both are
   693	        // owned by the spawned task so the row drains on every
   694	        // termination path (success, handler failure, client
   695	        // hangup). Module + dst path come straight off the
   696	        // request; they're synchronously available here unlike
   697	        // the streaming RPCs (push, pull_sync), which register
   698	        // with empty endpoint strings and have their handlers
   699	        // fill them in via `ActiveJobGuard::set_endpoint` once
   700	        // the first stream frame parses.
   701	        let job = self.active_jobs.register(
   702	            ActiveJobKind::DelegatedPull,
   703	            peer.clone(),
   704	            req.dst_module.clone(),
   705	            req.dst_destination_path.clone(),
   706	        );
   707	        // Subscribe event — module/path are populated for
   708	        // delegated_pull at dispatch time (unlike push/pull_sync).
   709	        self.emit_transfer_started(
   710	            &job,
   711	            ActiveJobKind::DelegatedPull,
   712	            &peer,
   713	            &req.dst_module,
   714	            &req.dst_destination_path,
   715	        );
   716	        // Captured before `req` moves into the handler call.
   717	        // Drives the conditional select arm below: when
   718	        // `detach=true` the `tx.closed()` race is disabled, so
   719	        // a CLI disconnect no longer drops the transfer
   720	        // future. The transfer then runs to completion,
   721	        // failure, or `CancelJob(transfer_id)` regardless of
   722	        // client connection state.
   723	        let detach = req.detach;
   724	        let transfer_id_for_started = job.transfer_id().to_string();
   725	        // c-1b: byte-progress sink fed by the data-plane write
   726	        // loop inside `pull_sync_with_spec`. Reports land on the
   727	        // same atomic the table row holds, so GetState sees live
   728	        // progress while the transfer is in flight.
   729	        let byte_progress = job.bytes_counter();
   730	        let modules = Arc::clone(&self.modules);
   731	        let default_root = self.default_root.clone();
   732	        let delegation = Arc::clone(&self.delegation);
   733	        let metrics = Arc::clone(&self.metrics);
   734	        let metrics_for_log = Arc::clone(&self.metrics);
   735	        let (tx, rx) = mpsc::channel(32);
   736	        // §3.1 followup: cover delegated_pull in the per-RPC summary
   737	        // log too. The handler increments `pull_ops` + the active
   738	        // gauge inside `run_delegated_pull` (delegated_pull.rs:227),
   739	        // so without this site `delegated_pull` would count toward
   740	        // `pull_ops` but never emit its own completion line.
   741	        let started = std::time::Instant::now();
   742	
   743	        // R30-F2: race the handler against tx.closed() so a CLI
   744	        // disconnect drops the inner pull future. tonic's response
   745	        // stream drops the mpsc Receiver when the client cancels;
   746	        // that closes the Sender, and tx.closed() resolves. The
   747	        // handler's pull_sync_with_spec future is then dropped,
   748	        // which propagates cancellation through the existing pull
   749	        // cancellation path (data plane connection drop, manifest
   750	        // task cleanup). Without this race the spawned task would
   751	        // continue to write — and post-R30-F1 to delete — on dst
   752	        // after the operator has Ctrl-C'd.
   753	        //
   754	        // Cloning tx for the handler so the original tx survives
   755	        // long enough for tx.closed() to be the racing future.
   756	        let handler_tx = tx.clone();
   757	        // Clone the cancellation token off the guard before
   758	        // moving the guard into the spawn task. The future's
   759	        // select needs a `.cancelled()` future; cloning the
   760	        // token (cheap, internal Arc) lets us hold the

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '1465,1545p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1465	fn outcome_from_status<T>(result: &Result<T, Status>) -> (bool, Option<String>) {
  1466	    match result {
  1467	        Ok(_) => (true, None),
  1468	        Err(status) => (false, Some(status.message().to_string())),
  1469	    }
  1470	}
  1471	
  1472	#[cfg(test)]
  1473	mod tests {
  1474	    use super::*;
  1475	    use crate::active_jobs::ActiveJobKind;
  1476	    use blit_core::generated::TransferKind as WireKind;
  1477	
  1478	    fn empty_service() -> BlitService {
  1479	        BlitService::with_modules(HashMap::new(), false)
  1480	    }
  1481	
  1482	    /// audit-10: a handler that has completed must win the `biased`
  1483	    /// select even when the cancel token (and the client-hangup signal)
  1484	    /// have ALSO fired — otherwise a transfer that succeeded at the same
  1485	    /// instant `CancelJob` fired gets mis-recorded as cancelled.
  1486	    /// (Helper renamed `resolve_delegated_pull_outcome` →
  1487	    /// `resolve_transfer_outcome` in w4-3; same select, now generic.)
  1488	    #[tokio::test]
  1489	    async fn resolve_pull_handler_completion_wins_over_simultaneous_cancel() {
  1490	        use std::future::ready;
  1491	        // Handler ready(success); client hung up; cancel fired — all
  1492	        // simultaneously. Handler-first ordering must yield Some(true).
  1493	        let outcome = resolve_transfer_outcome(ready(true), ready(()), ready(()), false).await;
  1494	        assert_eq!(outcome, Some(true), "ready success must win the race");
  1495	
  1496	        // The same holds for a handler that completed with failure: a
  1497	        // real result beats a simultaneous cancel.
  1498	        let outcome = resolve_transfer_outcome(ready(false), ready(()), ready(()), false).await;
  1499	        assert_eq!(outcome, Some(false), "ready failure must win the race");
  1500	    }
  1501	
  1502	    /// audit-10: a still-running (Pending) handler must still yield to a
  1503	    /// `CancelJob` cancel — the fix must not make transfers
  1504	    /// uncancellable.
  1505	    #[tokio::test]
  1506	    async fn resolve_pull_pending_handler_yields_to_cancel() {
  1507	        use std::future::{pending, ready};
  1508	        let outcome = resolve_transfer_outcome(
  1509	            pending::<bool>(), // handler still running
  1510	            pending::<()>(),   // client still connected
  1511	            ready(()),         // CancelJob fired
  1512	            false,
  1513	        )
  1514	        .await;
  1515	        assert_eq!(outcome, None, "a running handler must yield to cancel");
  1516	    }
  1517	
  1518	    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
  1519	    /// must reach the client as a framed `SessionError{CANCELLED}` on
  1520	    /// the response stream — not a bare `Status::cancelled` (the
  1521	    /// session speaks frames, and the aborted session future can't
  1522	    /// send it itself). Guard: with the cancel branch reverted to
  1523	    /// `Err(Status::cancelled)` this fails (no `Ok` error frame lands).
  1524	    #[tokio::test]
  1525	    async fn transfer_cancel_emits_framed_cancelled_error() {
  1526	        use blit_core::generated::session_error::Code;
  1527	        use blit_core::generated::transfer_frame::Frame as WireFrame;
  1528	
  1529	        let (tx, mut rx) = mpsc::channel::<Result<blit_core::generated::TransferFrame, Status>>(4);
  1530	        let cancel = CancellationToken::new();
  1531	        cancel.cancel();
  1532	        let metrics = TransferMetrics::disabled();
  1533	        // A session that never completes on its own — cancel must win.
  1534	        let never = std::future::pending::<Result<(), Status>>();
  1535	        let (ok, msg) = resolve_transfer_session_outcome(never, &tx, &cancel, &metrics).await;
  1536	        assert!(!ok, "cancel is not a success");
  1537	        assert_eq!(msg.as_deref(), Some("cancelled via CancelJob"));
  1538	
  1539	        let frame = rx
  1540	            .recv()
  1541	            .await
  1542	            .expect("a terminal frame")
  1543	            .expect("a framed SessionError, not a gRPC Status");
  1544	        match frame.frame {
  1545	            Some(WireFrame::Error(err)) => assert_eq!(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '1290,1465p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1290	        let module = resolve_module(&self.modules, self.default_root.as_ref(), &req.module).await?;
  1291	        let stats = filesystem_stats_for_path(&module.path)?;
  1292	
  1293	        Ok(Response::new(stats))
  1294	    }
  1295	}
  1296	
  1297	/// Format the remote peer of a tonic request as `<ip>:<port>`,
  1298	/// or `"unknown"` when the transport didn't surface one (eg.
  1299	/// in-process tests that bypass the network).
  1300	fn peer_addr_string<T>(request: &Request<T>) -> String {
  1301	    request
  1302	        .remote_addr()
  1303	        .map(|a| a.to_string())
  1304	        .unwrap_or_else(|| "unknown".to_string())
  1305	}
  1306	
  1307	/// Resolve a transfer's terminal outcome from its three racing
  1308	/// conditions, giving handler completion priority (audit-10).
  1309	///
  1310	/// Extracted for `delegated_pull` (R30-F2 / m-jobs-1) and generalized
  1311	/// in w4-3 to be the single owner of the biased select every transfer
  1312	/// RPC races through — `delegated_pull` calls it directly (handler
  1313	/// output `bool`), while `push` / `pull_sync` go through
  1314	/// [`resolve_streaming_outcome`] (handler output `Result<(), Status>`).
  1315	///
  1316	/// The select is `biased` with the **handler branch first**: when the
  1317	/// handler future is `Ready`, its result wins even if the cancel token
  1318	/// has also just fired or the client just hung up. A handler that is
  1319	/// still `Pending` yields to a client hangup (only when `!detach`) or a
  1320	/// `CancelJob` cancel, both of which resolve to `None` so the caller
  1321	/// records the cancellation.
  1322	///
  1323	/// Pre-audit-10 the cancel branch was evaluated before the handler, so a
  1324	/// transfer that completed at the same instant `CancelJob` fired its
  1325	/// token was mis-recorded as "cancelled via CancelJob" despite having
  1326	/// actually succeeded. Ordering completion first makes a real result
  1327	/// (success *or* failure) authoritative over a simultaneous cancel.
  1328	///
  1329	/// Returns `Some(output)` when the handler completed, or `None` for a
  1330	/// client hangup or cancel.
  1331	async fn resolve_transfer_outcome<T, H, C, K>(
  1332	    handler: H,
  1333	    tx_closed: C,
  1334	    cancelled: K,
  1335	    detach: bool,
  1336	) -> Option<T>
  1337	where
  1338	    H: std::future::Future<Output = T>,
  1339	    C: std::future::Future<Output = ()>,
  1340	    K: std::future::Future<Output = ()>,
  1341	{
  1342	    tokio::select! {
  1343	        biased;
  1344	        output = handler => Some(output),
  1345	        _ = tx_closed, if !detach => None,
  1346	        _ = cancelled => None,
  1347	    }
  1348	}
  1349	
  1350	/// w4-3: resolve a streaming transfer RPC's (`push` / `pull_sync`)
  1351	/// terminal outcome, racing the handler against client hangup and the
  1352	/// row's `CancelJob` token via [`resolve_transfer_outcome`].
  1353	///
  1354	/// Pre-w4-3 these dispatchers bare-awaited their handlers, so a client
  1355	/// that disconnected during a send-free compute phase (pull_sync's
  1356	/// enumerate+checksum collection, push's mirror purge) left the daemon
  1357	/// running the whole remaining handler for a dead peer — unbounded,
  1358	/// unobservable work that `CancelJob` also refused to touch
  1359	/// (async-daemon-handlers-blind-to-disconnect-in-compute-phases).
  1360	/// Dropping the handler future propagates through the existing
  1361	/// cancellation paths: the push data-plane accept task is
  1362	/// `AbortOnDrop`-wrapped and its workers live in a `JoinSet` (w4-1),
  1363	/// and pull_sync's payload feeder exits when its channel closes. An
  1364	/// in-flight `spawn_blocking` enumeration/checksum batch still runs to
  1365	/// its natural end with the result discarded — making that window
  1366	/// abortable is the finding's stated follow-up slice.
  1367	///
  1368	/// The streaming RPCs have no `detach` mode (the client is inherently
  1369	/// attached to the byte path), so the hangup arm is always armed —
  1370	/// hence the hardcoded `detach: false`.
  1371	///
  1372	/// Returns the `(ok, error_message)` pair the ActiveJobs ring records:
  1373	/// - handler completed → its result via [`outcome_from_status`]; an
  1374	///   `Err` is counted (`inc_error`) and forwarded to the
  1375	///   still-connected client, exactly as the pre-w4-3 dispatchers did.
  1376	/// - client hung up → `(false, "client cancelled")`; nothing is sent —
  1377	///   the receiver is gone, that's what ended the race.
  1378	/// - cancel token fired → `(false, "cancelled via CancelJob")`, and the
  1379	///   still-connected client gets a terminal `Status::cancelled`. This
  1380	///   arm is live in production: D-2026-07-04-3 flipped
  1381	///   `ActiveJobKind::supports_cancellation` on for push/pull_sync, so
  1382	///   `blit jobs cancel` (and the TUI `K`/`Shift+X`) reaches it for
  1383	///   attached transfers.
  1384	async fn resolve_streaming_outcome<T, H>(
  1385	    handler: H,
  1386	    tx: &mpsc::Sender<Result<T, Status>>,
  1387	    cancel_token: &CancellationToken,
  1388	    metrics: &TransferMetrics,
  1389	) -> (bool, Option<String>)
  1390	where
  1391	    H: std::future::Future<Output = Result<(), Status>>,
  1392	{
  1393	    let outcome =
  1394	        resolve_transfer_outcome(handler, tx.closed(), cancel_token.cancelled(), false).await;
  1395	    match outcome {
  1396	        Some(result) => {
  1397	            let (ok, err_msg) = outcome_from_status(&result);
  1398	            if let Err(status) = result {
  1399	                metrics.inc_error();
  1400	                let _ = tx.send(Err(status)).await;
  1401	            }
  1402	            (ok, err_msg)
  1403	        }
  1404	        // Same disambiguation the delegated_pull closure uses: a fired
  1405	        // token means the cause was CancelJob; otherwise the client
  1406	        // hung up.
  1407	        None if cancel_token.is_cancelled() => {
  1408	            let _ = tx
  1409	                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
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
/usr/bin/zsh -lc "rg -n \"transfer\\(|run_transfer_session|resolve_transfer_session_outcome|ActiveJobKind::Push\" crates/blit-daemon/src/service/core.rs crates/blit-daemon/src/service/transfer.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer.rs:106:pub(crate) async fn run_transfer_session(
crates/blit-daemon/src/service/core.rs:362:    async fn transfer(
crates/blit-daemon/src/service/core.rs:373:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:378:        // reuses ActiveJobKind::Push (daemon-receive = push-equivalent)
crates/blit-daemon/src/service/core.rs:381:            ActiveJobKind::Push,
crates/blit-daemon/src/service/core.rs:386:        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
crates/blit-daemon/src/service/core.rs:396:            let (ok, err_msg) = resolve_transfer_session_outcome(
crates/blit-daemon/src/service/core.rs:397:                super::transfer::run_transfer_session(modules, default_root, inbound, tx.clone()),
crates/blit-daemon/src/service/core.rs:535:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:541:            ActiveJobKind::Push,
crates/blit-daemon/src/service/core.rs:551:        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
crates/blit-daemon/src/service/core.rs:631:        let guard = Arc::clone(&metrics).enter_transfer();
crates/blit-daemon/src/service/core.rs:1426:async fn resolve_transfer_session_outcome<H>(
crates/blit-daemon/src/service/core.rs:1535:        let (ok, msg) = resolve_transfer_session_outcome(never, &tx, &cancel, &metrics).await;
crates/blit-daemon/src/service/core.rs:1839:        for kind in [ActiveJobKind::Push, ActiveJobKind::PullSync] {
crates/blit-daemon/src/service/core.rs:2180:            ActiveJobKind::Push,
crates/blit-daemon/src/service/core.rs:2299:    async fn progress_event_cannot_arrive_after_terminal_for_same_transfer() {
crates/blit-daemon/src/service/core.rs:2442:    async fn event_matches_filter_matches_only_target_transfer() {
crates/blit-daemon/src/service/core.rs:2806:            ActiveJobKind::Push,
crates/blit-daemon/src/service/core.rs:2811:        svc.emit_transfer_started(&guard, ActiveJobKind::Push, "p", "", "");
crates/blit-daemon/src/service/core.rs:2823:                ActiveJobKind::Push,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '345,425p'" in /home/michael/dev/Blit
 succeeded in 0ms:
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

exec
/usr/bin/zsh -lc 'git show 84be1cc -- crates/blit-daemon/src/service/core.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 84be1cccda812029c1347e6e2cf9661d0d9b4b95
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Mon Jul 6 02:02:32 2026 -0400

    otp-5a: daemon serves both roles — pull-equivalent over the in-stream carrier
    
    The one served Transfer RPC now handles BOTH directions by the client's
    declared initiator role, not a second code path. A DESTINATION initiator
    makes the daemon the SOURCE Responder (streaming its module tree, the
    pull-equivalent); a SOURCE initiator keeps otp-4's push-equivalent
    unchanged. otp-5a is in-stream only — the data-plane transport/role
    decoupling (responder binds+grants while SENDING; initiator dials while
    RECEIVING) is otp-5b.
    
    blit-core:
    - establish split into exchange_hello + responder_finish so a serving end
      can read the SessionOpen, learn the initiator's declared role, and only
      then pick a driver. establish keeps its shape for the direct role
      drivers (the in-process role suite).
    - run_responder: the daemon's single serving entry. Exchanges HELLO, reads
      the open, dispatches SOURCE-initiator -> local DESTINATION,
      DESTINATION-initiator -> local SOURCE. Carries a DestinationTarget and a
      new SourceResponderTarget; returns ResponderOutcome::{Destination,Source}.
    - SourceResponderTarget::{Fixed, Resolve}: symmetric with DestinationTarget.
      Resolve reuses the same OpenResolver the destination path uses to map
      module -> root, then builds FsTransferSource inside blit-core (read-only
      is ignored for a SOURCE). blit-core stays free of module/tonic::Status.
    - run_source/run_destination bodies factored into drive_source/
      drive_destination (both include the fault-notify wrapping) so all three
      entries share one session loop. source_send_half takes plan_options +
      data_plane_host directly instead of the whole SourceSessionConfig.
    
    daemon: run_transfer_session dispatches via run_responder (both resolvers
    built from make_open_resolver); both outcome arms map to the jobs record
    as before.
    
    client: run_pull_session (DESTINATION initiator, DestinationTarget::Fixed,
    in_stream) is the pull-equivalent entry. Not wired to CLI verbs (otp-10).
    
    Tests (1516 -> 1519):
    - pull_session_lands_bytes_and_scores_them: daemon module tree lands in the
      client dest byte-identically; in_stream_carrier_used.
    - old_pull_and_session_produce_identical_trees_and_counts: A/B parity vs
      old pull_sync (byte-identical, equal shared counters — no caveat, both
      SKIP the same-size dest-NEWER cell).
    - unknown_module_refuses_the_pull_session: MODULE_UNKNOWN to a DESTINATION
      initiator.
    Guard: forcing the responder to serve DESTINATION unconditionally makes the
    daemon refuse a DESTINATION initiator with PROTOCOL_VIOLATION (complement
    check) — the 3 pull tests FAIL, the 7 push tests still pass; restored, all
    pass. Role suite (14) still green (establish refactor).
    
    Finding: .review/findings/otp-5-daemon-as-source.md.
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>

exec
/usr/bin/zsh -lc "rg -n \"set_endpoint\\(|emit_transfer_started|with_byte_progress|bytes_counter|TransferStarted|TransferFinished\" crates/blit-daemon/src/service crates/blit-daemon/src/active_jobs.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/transfer.rs:23://! wiring (`with_byte_progress`) is not threaded yet — session rows
crates/blit-daemon/src/service/core.rs:24:    TransferProgress, TransferRecord, TransferStarted,
crates/blit-daemon/src/service/core.rs:159:    /// Send a `TransferStarted` event onto the broadcast channel.
crates/blit-daemon/src/service/core.rs:172:    pub(crate) fn emit_transfer_started(
crates/blit-daemon/src/service/core.rs:181:            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
crates/blit-daemon/src/service/core.rs:296:        Some(daemon_event::Payload::TransferStarted(e)) => e.transfer_id == filter,
crates/blit-daemon/src/service/core.rs:307:/// ActiveJobs ring records. Pairs with `emit_transfer_started` on
crates/blit-daemon/src/service/core.rs:386:        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
crates/blit-daemon/src/service/core.rs:538:        // calls `job.set_endpoint(...)` once the header is
crates/blit-daemon/src/service/core.rs:551:        self.emit_transfer_started(&job, ActiveJobKind::Push, &peer, "", "");
crates/blit-daemon/src/service/core.rs:634:        // `job.set_endpoint(...)` after parsing the spec.
crates/blit-daemon/src/service/core.rs:644:        self.emit_transfer_started(&job, ActiveJobKind::PullSync, &peer, "", "");
crates/blit-daemon/src/service/core.rs:709:        self.emit_transfer_started(
crates/blit-daemon/src/service/core.rs:729:        let byte_progress = job.bytes_counter();
crates/blit-daemon/src/service/core.rs:1728:        guard.bytes_counter().report(4096);
crates/blit-daemon/src/service/core.rs:1754:        // captured at Drop. The earlier `bytes_counter().report(4096)`
crates/blit-daemon/src/service/core.rs:1982:        // Fire a TransferStarted by registering a job on the
crates/blit-daemon/src/service/core.rs:1991:        svc.emit_transfer_started(
crates/blit-daemon/src/service/core.rs:2000:        // First (and only) frame should be a TransferStarted.
crates/blit-daemon/src/service/core.rs:2008:            daemon_event::Payload::TransferStarted(ev) => {
crates/blit-daemon/src/service/core.rs:2016:            other => panic!("expected TransferStarted, got {other:?}"),
crates/blit-daemon/src/service/core.rs:2050:        svc.emit_transfer_started(&guard, ActiveJobKind::Pull, "p", "m", "/");
crates/blit-daemon/src/service/core.rs:2063:            daemon_event::Payload::TransferStarted(e) => e.transfer_id,
crates/blit-daemon/src/service/core.rs:2064:            other => panic!("expected TransferStarted, got {other:?}"),
crates/blit-daemon/src/service/core.rs:2067:            daemon_event::Payload::TransferStarted(e) => e.transfer_id,
crates/blit-daemon/src/service/core.rs:2068:            other => panic!("expected TransferStarted, got {other:?}"),
crates/blit-daemon/src/service/core.rs:2090:        guard.bytes_counter().report(2048);
crates/blit-daemon/src/service/core.rs:2218:        g1.bytes_counter().report(4096);
crates/blit-daemon/src/service/core.rs:2226:        // dispatch site calls emit_transfer_started, which we
crates/blit-daemon/src/service/core.rs:2286:        guard.bytes_counter().report(50 * 1024);
crates/blit-daemon/src/service/core.rs:2333:        guard.bytes_counter().report(1024);
crates/blit-daemon/src/service/core.rs:2429:            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
crates/blit-daemon/src/service/core.rs:2459:            payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
crates/blit-daemon/src/service/core.rs:2494:        // emit_transfer_started has fired (and any early
crates/blit-daemon/src/service/core.rs:2511:        svc.emit_transfer_started(&guard, ActiveJobKind::DelegatedPull, "p", "m", "/");
crates/blit-daemon/src/service/core.rs:2512:        guard.bytes_counter().report(1024);
crates/blit-daemon/src/service/core.rs:2514:        guard.bytes_counter().report(2048);
crates/blit-daemon/src/service/core.rs:2538:                daemon_event::Payload::TransferStarted(_) => seen.push("started"),
crates/blit-daemon/src/service/core.rs:2563:        svc.emit_transfer_started(&guard, ActiveJobKind::DelegatedPull, "p", "m", "/");
crates/blit-daemon/src/service/core.rs:2712:                    payload: Some(daemon_event::Payload::TransferStarted(TransferStarted {
crates/blit-daemon/src/service/core.rs:2729:        svc.emit_transfer_started(&g_a, ActiveJobKind::DelegatedPull, "a", "m", "/");
crates/blit-daemon/src/service/core.rs:2740:            daemon_event::Payload::TransferStarted(e) => assert_eq!(e.transfer_id, id_a),
crates/blit-daemon/src/service/core.rs:2741:            other => panic!("expected id_a TransferStarted, got {other:?}"),
crates/blit-daemon/src/service/core.rs:2780:        svc.emit_transfer_started(&g_a, ActiveJobKind::DelegatedPull, "p", "m", "/");
crates/blit-daemon/src/service/core.rs:2781:        svc.emit_transfer_started(&g_b, ActiveJobKind::DelegatedPull, "p", "m", "/");
crates/blit-daemon/src/service/core.rs:2785:            daemon_event::Payload::TransferStarted(e) => assert_eq!(e.transfer_id, id_a),
crates/blit-daemon/src/service/core.rs:2786:            other => panic!("expected id_a TransferStarted, got {other:?}"),
crates/blit-daemon/src/service/core.rs:2802:        // SendError, which `emit_transfer_started` ignores.
crates/blit-daemon/src/service/core.rs:2811:        svc.emit_transfer_started(&guard, ActiveJobKind::Push, "p", "", "");
crates/blit-daemon/src/service/pull_sync.rs:74:    active_job.set_endpoint(spec.module.clone(), spec.source_path.clone());
crates/blit-daemon/src/service/push/control.rs:118:                active_job.set_endpoint(header.module.clone(), header.destination_path.clone());
crates/blit-daemon/src/active_jobs.rs:17://! - `b-2-set-endpoint`: `ActiveJobGuard::set_endpoint(module,
crates/blit-daemon/src/active_jobs.rs:45://!   `bytes_completed`. [`ActiveJobGuard::bytes_counter`]
crates/blit-daemon/src/active_jobs.rs:113:/// Sized to comfortably hold one TransferStarted + many
crates/blit-daemon/src/active_jobs.rs:122:/// introduces the `TransferStarted.Kind` wire enum, the
crates/blit-daemon/src/active_jobs.rs:322:    /// loaded from `bytes_counter`.
crates/blit-daemon/src/active_jobs.rs:327:    /// [`ActiveJobGuard::bytes_counter`]; loaded by
crates/blit-daemon/src/active_jobs.rs:330:    bytes_counter: Arc<AtomicU64>,
crates/blit-daemon/src/active_jobs.rs:434:        let bytes_counter = Arc::new(AtomicU64::new(0));
crates/blit-daemon/src/active_jobs.rs:438:            bytes_counter: Arc::clone(&bytes_counter),
crates/blit-daemon/src/active_jobs.rs:454:            bytes_counter,
crates/blit-daemon/src/active_jobs.rs:536:                job.bytes_completed = e.bytes_counter.load(Ordering::Relaxed);
crates/blit-daemon/src/active_jobs.rs:657:            let cur_bytes = entry.bytes_counter.load(Ordering::Relaxed);
crates/blit-daemon/src/active_jobs.rs:702:            let cur_bytes = entry.bytes_counter.load(Ordering::Relaxed);
crates/blit-daemon/src/active_jobs.rs:883:    /// `TransferStarted` events don't need to hit the table lock
crates/blit-daemon/src/active_jobs.rs:899:    /// Same atomic the [`TableEntry::bytes_counter`] holds —
crates/blit-daemon/src/active_jobs.rs:903:    bytes_counter: Arc<AtomicU64>,
crates/blit-daemon/src/active_jobs.rs:917:    /// `TransferStarted.transfer_id`, M-Jobs `CancelJob`) can
crates/blit-daemon/src/active_jobs.rs:928:    /// `TransferStarted` event without re-stamping the clock
crates/blit-daemon/src/active_jobs.rs:942:        self.bytes_counter.load(Ordering::Relaxed)
crates/blit-daemon/src/active_jobs.rs:971:    pub fn set_endpoint(&self, module: String, path: String) {
crates/blit-daemon/src/active_jobs.rs:1000:    pub fn bytes_counter(&self) -> ByteProgressSink {
crates/blit-daemon/src/active_jobs.rs:1001:        ByteProgressSink::from_counter(Arc::clone(&self.bytes_counter))
crates/blit-daemon/src/active_jobs.rs:1057:                let bytes = entry.bytes_counter.load(Ordering::Relaxed);
crates/blit-daemon/src/active_jobs.rs:1328:        guard.set_endpoint("mod-streaming".to_string(), "sub/dir".to_string());
crates/blit-daemon/src/active_jobs.rs:1364:        guard.set_endpoint("mod".to_string(), "p".to_string());
crates/blit-daemon/src/active_jobs.rs:1956:    async fn bytes_counter_starts_at_zero_and_reflects_reports() {
crates/blit-daemon/src/active_jobs.rs:1968:        let sink = guard.bytes_counter();
crates/blit-daemon/src/active_jobs.rs:1979:    async fn bytes_counter_clones_share_state() {
crates/blit-daemon/src/active_jobs.rs:1991:        let sink_a = guard.bytes_counter();
crates/blit-daemon/src/active_jobs.rs:1993:        let sink_c = guard.bytes_counter();
crates/blit-daemon/src/active_jobs.rs:2011:            let sink = guard.bytes_counter();
crates/blit-daemon/src/active_jobs.rs:2039:            let sink = guard.bytes_counter();

codex
The claimed guard proof is mechanically sound: if `run_responder` is replaced with unconditional `run_destination`, a DESTINATION initiator fails the complement check as `PROTOCOL_VIOLATION`. I’m doing one last pass over test coverage/count and line-specific candidates before writing the verdict.
exec
/usr/bin/zsh -lc "rg -n \"#\\[(tokio::test|test)\" crates | wc -l" in /home/michael/dev/Blit
 succeeded in 0ms:
1540

exec
/usr/bin/zsh -lc 'git diff --unified=0 84be1cc''^ 84be1cc -- crates/blit-daemon/src/service/transfer_session_e2e.rs | rg -n "''^'"\\+\\s*#\\[tokio::test|"'^-'"\\s*#\\[tokio::test|"'^'"\\+\\s*#\\[test|"'^-'"\\s*#\\[test\"" in /home/michael/dev/Blit
 succeeded in 0ms:
27:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
62:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
113:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]

exec
/usr/bin/zsh -lc "git grep -n \"#\\[(tokio::test\\|test)\" 84be1cc -- crates | wc -l" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "git grep -n \"#\\[(tokio::test\\|test)\" 84be1cc"'^ -- crates | wc -l' in /home/michael/dev/Blit
 succeeded in 0ms:
134

 succeeded in 0ms:
135

exec
/usr/bin/zsh -lc 'git diff --check 84be1cc''^ 84be1cc' in /home/michael/dev/Blit
 succeeded in 0ms:
exec
/usr/bin/zsh -lc 'git diff --unified=0 84be1cc''^ 84be1cc -- crates | rg -n "''^[+-]'"\\s*#\\[(tokio::test|test)\"" in /home/michael/dev/Blit
 succeeded in 0ms:
672:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
707:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
758:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]

exec
/usr/bin/zsh -lc "git grep -n -E \"#\\[(tokio::test|test)\" 84be1cc"'^ -- crates | wc -l' in /home/michael/dev/Blit
 succeeded in 0ms:
1537

exec
/usr/bin/zsh -lc "git grep -n -E \"#\\[(tokio::test|test)\" 84be1cc -- crates | wc -l" in /home/michael/dev/Blit
 succeeded in 0ms:
1540

exec
/usr/bin/zsh -lc 'git diff --numstat 84be1cc''^ 84be1cc' in /home/michael/dev/Blit
 succeeded in 0ms:
129	0	.review/findings/otp-5-daemon-as-source.md
106	25	crates/blit-core/src/remote/transfer/session_client.rs
294	106	crates/blit-core/src/transfer_session/mod.rs
46	28	crates/blit-daemon/src/service/transfer.rs
116	2	crates/blit-daemon/src/service/transfer_session_e2e.rs

codex
The diff itself adds exactly three `#[tokio::test]` functions and removes none, so the test-source count moves upward. I can’t run the cargo suite in this read-only sandbox, so the final verdict will separate static confirmation from executed verification.
exec
/usr/bin/zsh -lc "sed -n '1,230p' crates/blit-core/tests/transfer_session_roles.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
//! Role-parameterized session suite (otp-3).
//!
//! Every fixture runs under BOTH role assignments — the initiator as
//! SOURCE (push-shaped) and the initiator as DESTINATION (pull-shaped)
//! — over the in-process transport, and the outcomes must be
//! IDENTICAL: same need-list set, same summary counts, same bytes on
//! disk. This is the owner's invariance requirement
//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
//! executable form: there is no per-direction code to diverge, and
//! this suite pins that the one code path really is
//! initiator-indifferent.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use blit_core::generated::transfer_frame::Frame;
use blit_core::generated::{
    session_error, ComparisonMode, FileHeader, ManifestComplete, NeedBatch, NeedComplete,
    NeedEntry, SessionHello, SessionOpen, TransferFrame, TransferRole, TransferSummary,
};
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::transfer_plan::PlanOptions;
use blit_core::transfer_session::transport::{in_process_pair, FrameTransport};
use blit_core::transfer_session::{
    run_destination, run_source, DestinationOutcome, DestinationSessionConfig, DestinationTarget,
    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
};

const SUITE_TIMEOUT: Duration = Duration::from_secs(120);

/// (relative path, content, mtime seconds). Fixture mtimes are fixed
/// epochs so both role-assignment runs see byte-for-byte identical
/// trees.
type FileSpec = (&'static str, Vec<u8>, i64);

fn write_tree(root: &Path, files: &[FileSpec]) {
    for (rel, content, mtime) in files {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
    }
}

/// Every regular file under `root` as rel-path → bytes.
fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    if root.exists() {
        walk(root, root, &mut out);
    }
    out
}

fn assert_trees_identical(src: &Path, dst: &Path) {
    let src_tree = collect_tree(src);
    let dst_tree = collect_tree(dst);
    assert_eq!(
        src_tree.keys().collect::<Vec<_>>(),
        dst_tree.keys().collect::<Vec<_>>(),
        "path sets differ between {src:?} and {dst:?}"
    );
    for (rel, bytes) in &src_tree {
        assert_eq!(
            bytes, &dst_tree[rel],
            "content differs for '{rel}' between {src:?} and {dst:?}"
        );
    }
}

fn basic_open(initiator_role: TransferRole) -> SessionOpen {
    SessionOpen {
        initiator_role: initiator_role as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: true,
        ..Default::default()
    }
}

/// Drive one full session between `src_root` and `dst_root` with the
/// given end acting as initiator. Data direction is FIXED
/// (src_root → dst_root); the parameter only swaps which end opens
/// the session — the thing the owner's invariant says must not
/// matter.
async fn run_session(
    initiator_role: TransferRole,
    src_root: &Path,
    dst_root: &Path,
    plan_options: PlanOptions,
) -> (
    eyre::Result<TransferSummary>,
    eyre::Result<DestinationOutcome>,
) {
    let open = basic_open(initiator_role);
    let (source_endpoint, dest_endpoint) = match initiator_role {
        TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
        TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
        TransferRole::Unspecified => panic!("fixture must pick a role"),
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: source_endpoint,
        plan_options,
        data_plane_host: None,
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: dest_endpoint,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
    tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(
                dest_cfg,
                b,
                DestinationTarget::Fixed(dst_root.to_path_buf())
            ),
        )
    })
    .await
    .expect("session run timed out")
}

/// Run the same fixture under both role assignments (fresh trees per
/// run) and pin the invariance property: identical need sets,
/// identical summaries, byte-identical destinations.
async fn assert_invariant_across_roles(
    src_files: &[FileSpec],
    dst_files: &[FileSpec],
    plan_options: PlanOptions,
) -> (TransferSummary, Vec<String>) {
    let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();
        write_tree(&src_root, src_files);
        write_tree(&dst_root, dst_files);

        let (source_result, dest_result) =
            run_session(initiator_role, &src_root, &dst_root, plan_options).await;
        let source_summary = source_result
            .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
        let dest_outcome = dest_result.unwrap_or_else(|e| {
            panic!("destination failed under initiator {initiator_role:?}: {e:#}")
        });

        assert_eq!(
            source_summary, dest_outcome.summary,
            "both ends must hold the same summary (initiator {initiator_role:?})"
        );
        assert!(
            source_summary.in_stream_carrier_used,
            "otp-3 sessions ride the in-stream carrier"
        );
        assert_trees_identical(&src_root, &dst_root);

        let mut needed = dest_outcome.needed_paths.clone();
        needed.sort();
        per_role.push((dest_outcome.summary, needed));
    }

    let (summary_a, needed_a) = per_role.remove(0);
    let (summary_b, needed_b) = per_role.remove(0);
    assert_eq!(
        needed_a, needed_b,
        "need-list set must be identical whichever end initiates"
    );
    assert_eq!(
        summary_a, summary_b,
        "summary must be identical whichever end initiates"
    );
    (summary_a, needed_a)
}

fn fault_of(err: &eyre::Report) -> &SessionFault {
    err.downcast_ref::<SessionFault>()
        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Mixed small tree: nested dirs, an empty file, a name with spaces,
/// and a file larger than the in-stream chunk so file records span
/// multiple FileData frames.
fn small_tree() -> Vec<FileSpec> {
    vec![
        ("a.txt", b"alpha".to_vec(), 1_600_000_001),
        ("empty.bin", Vec::new(), 1_600_000_002),
        ("dir one/b.log", vec![0xAB; 4096], 1_600_000_003),
        (
            "dir one/deeper/c.dat",
            b"gamma-content".to_vec(),
            1_600_000_004,
        ),
        // 3 MiB + 17 so the record needs 4 FileData frames and ends
        // on a partial chunk.
        (
            "big/blob.bin",
            make_patterned(3 * 1024 * 1024 + 17),
            1_600_000_005,
        ),
    ]
}


exec
/usr/bin/zsh -lc "sed -n '230,430p' crates/blit-core/tests/transfer_session_roles.rs" in /home/michael/dev/Blit
 succeeded in 0ms:

fn make_patterned(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[tokio::test]
async fn small_tree_byte_identical_under_both_initiators() {
    let src = small_tree();
    let (summary, needed) = assert_invariant_across_roles(&src, &[], PlanOptions::default()).await;
    assert_eq!(summary.files_transferred, src.len() as u64);
    assert_eq!(
        summary.bytes_transferred,
        src.iter().map(|(_, c, _)| c.len() as u64).sum::<u64>()
    );
    assert_eq!(summary.entries_deleted, 0);
    assert_eq!(summary.files_resumed, 0);
    assert_eq!(
        needed.len(),
        src.len(),
        "empty destination needs everything"
    );
}

#[tokio::test]
async fn tiny_file_tree_tar_shard_records_under_both_initiators() {
    // 200 tiny files under nested dirs; force_tar makes the planner's
    // tar-shard choice deterministic so the tar record grammar
    // (header + chunks + complete → tar-safety unpack) is exercised
    // under both role assignments.
    let mut src: Vec<FileSpec> = Vec::new();
    let names: Vec<String> = (0..200)
        .map(|i| format!("shards/d{}/f{:03}.txt", i % 7, i))
        .collect();
    let leaked: Vec<&'static str> = names
        .into_iter()
        .map(|n| Box::leak(n.into_boxed_str()) as &'static str)
        .collect();
    for (i, name) in leaked.iter().enumerate() {
        src.push((
            name,
            format!("tiny-{i}").into_bytes(),
            1_600_100_000 + i as i64,
        ));
    }
    let plan = PlanOptions {
        force_tar: true,
        ..PlanOptions::default()
    };
    let (summary, needed) = assert_invariant_across_roles(&src, &[], plan).await;
    assert_eq!(summary.files_transferred, 200);
    assert_eq!(needed.len(), 200);
}

#[tokio::test]
async fn incremental_transfer_needs_only_missing_and_changed() {
    let src: Vec<FileSpec> = vec![
        // Identical on both sides (same size, same mtime) → skipped.
        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
        // Same size, source newer → transferred.
        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
        // Absent on destination → transferred.
        ("sub/missing.txt", b"fresh".to_vec(), 1_600_000_300),
    ];
    let dst: Vec<FileSpec> = vec![
        ("same.txt", b"unchanged-content".to_vec(), 1_600_000_100),
        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
    ];
    let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
    assert_eq!(
        needed,
        vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
        "need list must be exactly the changed + missing files"
    );
    assert_eq!(summary.files_transferred, 2);
    assert_eq!(summary.bytes_transferred, 9 + 5);
}

#[tokio::test]
async fn preexisting_identical_tree_yields_empty_need_list() {
    let files: Vec<FileSpec> = vec![
        ("one.txt", b"matching".to_vec(), 1_600_000_400),
        ("nested/two.txt", b"also matching".to_vec(), 1_600_000_500),
    ];
    let (summary, needed) =
        assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
    assert!(needed.is_empty(), "identical trees must need nothing");
    assert_eq!(summary.files_transferred, 0);
    assert_eq!(summary.bytes_transferred, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
    // sf-2 pin ported onto the unified session (otp-4b-2). The responder
    // grants the zero-knowledge single stream (no manifest seen at
    // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
    // must re-run the shape table over the accumulated need list and grow
    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
    // push sf-2 pin (`shape_resize_e2e.rs`), now on the session: the
    // settled count is read from the destination's `data_plane_streams`.
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    const FILE_COUNT: usize = 10_000;
    for i in 0..FILE_COUNT {
        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
    }

    // SOURCE initiator over the TCP data plane: the control lane rides the
    // in-process transport; the data-plane sockets ride loopback TCP (the
    // responder binds 0.0.0.0:0 and the source dials 127.0.0.1).
    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        compare_mode: ComparisonMode::SizeMtime as i32,
        in_stream_bytes: false,
        ..Default::default()
    };
    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: Some("127.0.0.1".into()),
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
    };
    let (a, b) = in_process_pair();
    let source = Arc::new(FsTransferSource::new(src_root.clone()));
    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
        tokio::join!(
            run_source(source_cfg, a, source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        )
    })
    .await
    .expect("session run timed out");

    let summary = source_result.expect("source succeeds");
    let outcome = dest_result.expect("destination succeeds");
    assert!(
        !summary.in_stream_carrier_used,
        "the sf-2 pin must ride the TCP data plane"
    );
    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
    let streams = outcome
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!(
        streams > 1,
        "a {FILE_COUNT}-file transfer must correct the single-stream grant \
         upward via shape resize; settled at {streams}"
    );
    assert_trees_identical(&src_root, &dst_root);
}

#[tokio::test]
async fn preserves_mtime_on_streamed_files() {
    // Not part of the role matrix — pins that the file-record write
    // path applies the manifest mtime (parity with today's receive
    // paths, which the byte-identical asserts alone wouldn't catch).
    let tmp = tempfile::tempdir().unwrap();
    let src_root = tmp.path().join("src");
    let dst_root = tmp.path().join("dst");
    std::fs::create_dir_all(&src_root).unwrap();
    std::fs::create_dir_all(&dst_root).unwrap();
    write_tree(
        &src_root,
        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
    );

    let (source_result, dest_result) = run_session(
        TransferRole::Source,
        &src_root,
        &dst_root,
        PlanOptions::default(),
    )
    .await;
    source_result.unwrap();
    dest_result.unwrap();

    let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
    let mtime = filetime::FileTime::from_last_modification_time(&meta);
    assert_eq!(mtime.unix_seconds(), 1_555_555_555);
}

// ---------------------------------------------------------------------------
// Handshake refusals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn build_mismatch_refused_under_both_initiators() {
    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
        let tmp = tempfile::tempdir().unwrap();
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).unwrap();
        std::fs::create_dir_all(&dst_root).unwrap();

        let open = basic_open(initiator_role);

exec
/usr/bin/zsh -lc 'rg -n "same_size|newer|SizeMtime|old pull|pull.*newer|ignore_existing|force|checksum" crates/blit-core/tests crates/blit-daemon/src/service/transfer_session_e2e.rs crates/blit-core/src/remote/pull.rs crates/blit-daemon/src/service/pull_sync.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/pull_sync.rs:51:    force_grpc_override: bool,
crates/blit-daemon/src/service/pull_sync.rs:52:    server_checksums_enabled: bool,
crates/blit-daemon/src/service/pull_sync.rs:79:    let force_grpc = spec.force_grpc || force_grpc_override;
crates/blit-daemon/src/service/pull_sync.rs:84:    let client_wants_checksum = matches!(compare_mode_kind, ComparisonMode::Checksum);
crates/blit-daemon/src/service/pull_sync.rs:96:    send_pull_sync_ack(&tx, server_checksums_enabled).await?;
crates/blit-daemon/src/service/pull_sync.rs:118:    // Compute checksums if client requests checksum mode and server has checksums enabled.
crates/blit-daemon/src/service/pull_sync.rs:119:    let compute_checksums = client_wants_checksum && server_checksums_enabled;
crates/blit-daemon/src/service/pull_sync.rs:120:    let (server_entries, scan_outcome) = collect_pull_entries_with_checksums(
crates/blit-daemon/src/service/pull_sync.rs:124:        compute_checksums,
crates/blit-daemon/src/service/pull_sync.rs:207:    // by manifest comparison primitives. ignore_existing is now its
crates/blit-daemon/src/service/pull_sync.rs:211:    let ignore_existing = spec.ignore_existing;
crates/blit-daemon/src/service/pull_sync.rs:216:        ignore_existing,
crates/blit-daemon/src/service/pull_sync.rs:222:    // - Off: no deletions ever (compare_opts already enforces this).
crates/blit-daemon/src/service/pull_sync.rs:301:    if force_grpc {
crates/blit-daemon/src/service/pull_sync.rs:391:/// internal `CompareMode`. `ignore_existing` is no longer carried via
crates/blit-daemon/src/service/pull_sync.rs:400:        // Unspecified | SizeMtime — both fall back to the historical default.
crates/blit-daemon/src/service/pull_sync.rs:476:    server_checksums_enabled: bool,
crates/blit-daemon/src/service/pull_sync.rs:480:            server_checksums_enabled,
crates/blit-daemon/src/service/pull_sync.rs:1637:/// ran its metadata probes and — with checksums on — a full
crates/blit-daemon/src/service/pull_sync.rs:1641:async fn collect_pull_entries_with_checksums(
crates/blit-daemon/src/service/pull_sync.rs:1645:    compute_checksums: bool,
crates/blit-daemon/src/service/pull_sync.rs:1652:        collect_pull_entries_sync(&module_root, &root, &requested, compute_checksums, filter)
crates/blit-daemon/src/service/pull_sync.rs:1662:    compute_checksums: bool,
crates/blit-daemon/src/service/pull_sync.rs:1698:        let mut header = build_file_header(module_root, &physical, compute_checksums)?;
crates/blit-daemon/src/service/pull_sync.rs:1733:    let files: Result<Vec<PullEntry>, Status> = if compute_checksums {
crates/blit-daemon/src/service/pull_sync.rs:1769:    compute_checksum: bool,
crates/blit-daemon/src/service/pull_sync.rs:1773:    if compute_checksum {
crates/blit-daemon/src/service/pull_sync.rs:1783:        let checksum =
crates/blit-daemon/src/service/pull_sync.rs:1784:            blit_core::checksum::hash_reader(&mut file, blit_core::checksum::ChecksumType::Blake3)
crates/blit-daemon/src/service/pull_sync.rs:1792:            checksum,
crates/blit-daemon/src/service/pull_sync.rs:1795:        // Just get metadata, no checksum needed
crates/blit-daemon/src/service/pull_sync.rs:1804:            checksum: vec![],
crates/blit-daemon/src/service/pull_sync.rs:1816:    //! `collect_pull_entries_with_checksums`, the function they pin.)
crates/blit-daemon/src/service/pull_sync.rs:1832:        let (entries, outcome) = collect_pull_entries_with_checksums(
crates/blit-daemon/src/service/pull_sync.rs:1858:        let (entries, _) = collect_pull_entries_with_checksums(
crates/blit-daemon/src/service/pull_sync.rs:1884:            checksum: vec![],
crates/blit-daemon/src/service/transfer_session_e2e.rs:16://! - the unified SizeMtime semantic: a same-size destination file that
crates/blit-daemon/src/service/transfer_session_e2e.rs:514:async fn same_size_newer_destination_is_skipped_not_clobbered() {
crates/blit-daemon/src/service/transfer_session_e2e.rs:515:    // The unified SizeMtime decision (finding doc compare section): the
crates/blit-daemon/src/service/transfer_session_e2e.rs:518:    // a newer destination file with older source content. (--force
crates/blit-daemon/src/service/transfer_session_e2e.rs:550:    // Only stale.txt transfers; keep.txt (newer on dest) is left intact.
crates/blit-daemon/src/service/transfer_session_e2e.rs:558:        "a newer same-size destination file must NOT be clobbered"
crates/blit-daemon/src/service/transfer_session_e2e.rs:615:        .expect("old pull client connects");
crates/blit-daemon/src/service/transfer_session_e2e.rs:625:        .expect("old pull succeeds");
crates/blit-core/src/remote/pull.rs:73:    pub force_grpc: bool,
crates/blit-core/src/remote/pull.rs:90:    pub ignore_existing: bool,
crates/blit-core/src/remote/pull.rs:91:    /// Overwrite even if target is newer (dangerous).
crates/blit-core/src/remote/pull.rs:92:    pub force: bool,
crates/blit-core/src/remote/pull.rs:93:    /// Force checksum comparison (slower but more accurate).
crates/blit-core/src/remote/pull.rs:94:    pub checksum: bool,
crates/blit-core/src/remote/pull.rs:128:    /// Daemon's `server_checksums_enabled` advertisement from the
crates/blit-core/src/remote/pull.rs:132:    /// when the client asked for `--checksum` mode but the daemon
crates/blit-core/src/remote/pull.rs:133:    /// has checksums disabled, the comparison would silently
crates/blit-core/src/remote/pull.rs:137:    pub server_checksums_enabled: Option<bool>,
crates/blit-core/src/remote/pull.rs:248:    /// mirror/resume, and always `force_grpc` — bytes, when any, ride
crates/blit-core/src/remote/pull.rs:278:            force_grpc: true,
crates/blit-core/src/remote/pull.rs:279:            ignore_existing: false,
crates/blit-core/src/remote/pull.rs:355:    /// `force_grpc` it streams every file's bytes over the control
crates/blit-core/src/remote/pull.rs:387:                // force_grpc was set, so a real TCP negotiation (as
crates/blit-core/src/remote/pull.rs:395:                         metadata-only scan (force_grpc was set)"
crates/blit-core/src/remote/pull.rs:409:    /// single-file `force_grpc` PullSync session — ue-r2-1h's port of
crates/blit-core/src/remote/pull.rs:452:        // `ignore_existing` spec field. The CLI rejects `--force
crates/blit-core/src/remote/pull.rs:457:        } else if options.force {
crates/blit-core/src/remote/pull.rs:461:        } else if options.checksum {
crates/blit-core/src/remote/pull.rs:464:            ComparisonMode::SizeMtime
crates/blit-core/src/remote/pull.rs:503:            force_grpc: options.force_grpc,
crates/blit-core/src/remote/pull.rs:504:            ignore_existing: options.ignore_existing,
crates/blit-core/src/remote/pull.rs:576:        // Derive checksum-mode flag from the spec for the PullSyncAck
crates/blit-core/src/remote/pull.rs:579:        let checksum_requested = spec.compare_mode == ComparisonMode::Checksum as i32;
crates/blit-core/src/remote/pull.rs:710:                    // the user explicitly asked for `--checksum` but
crates/blit-core/src/remote/pull.rs:711:                    // the daemon has checksums disabled. Silently
crates/blit-core/src/remote/pull.rs:714:                    report.server_checksums_enabled = Some(ack.server_checksums_enabled);
crates/blit-core/src/remote/pull.rs:715:                    if checksum_requested && !ack.server_checksums_enabled {
crates/blit-core/src/remote/pull.rs:717:                            "client requested checksum comparison (--checksum) but the daemon \
crates/blit-core/src/remote/pull.rs:718:                             has checksums disabled; aborting before transfer to avoid silent \
crates/blit-core/src/remote/pull.rs:1280:                        // force_grpc was set — a real TCP negotiation
crates/blit-core/src/remote/pull.rs:1286:                                 force_grpc single-file session",
crates/blit-core/src/remote/pull.rs:1439:            checksum: vec![],
crates/blit-core/src/remote/pull.rs:1506:        // Recompute checksum (offset 148, 8 bytes ASCII octal). Tar
crates/blit-core/src/remote/pull.rs:1507:        // checksum spec: sum of all header bytes treating chksum
crates/blit-core/src/remote/pull.rs:1653:/// enforce its own ceiling too — a buggy or hostile daemon advertising
crates/blit-core/src/remote/pull.rs:1910:        checksum: None,
crates/blit-core/src/remote/pull.rs:1915:        compare_mode: ComparisonMode::SizeMtime,
crates/blit-core/src/remote/pull.rs:2153:        // ue-r2-1g self-review F1: the receive side enforces its own
crates/blit-core/src/remote/pull.rs:2321:        // source_path="x/y", compare_mode=SizeMtime, mirror=Off,
crates/blit-core/src/remote/pull.rs:2332:        assert_eq!(spec.compare_mode, ComparisonMode::SizeMtime as i32);
crates/blit-core/src/remote/pull.rs:2334:        assert!(!spec.force_grpc);
crates/blit-core/src/remote/pull.rs:2335:        assert!(!spec.ignore_existing);
crates/blit-core/src/remote/pull.rs:2353:        // force > size_only > checksum > size_mtime.
crates/blit-core/src/remote/pull.rs:2364:            force: true,
crates/blit-core/src/remote/pull.rs:2378:            checksum: true,
crates/blit-core/src/remote/pull.rs:2418:    fn wire_equivalence_resume_and_filter_and_force_grpc() {
crates/blit-core/src/remote/pull.rs:2433:            force_grpc: true,
crates/blit-core/src/remote/pull.rs:2434:            ignore_existing: true,
crates/blit-core/src/remote/pull.rs:2445:        assert!(spec.force_grpc);
crates/blit-core/src/remote/pull.rs:2446:        assert!(spec.ignore_existing);
crates/blit-core/src/remote/pull.rs:2494:            checksum: true,
crates/blit-core/src/remote/pull.rs:2499:            force_grpc: false,
crates/blit-core/src/remote/pull.rs:2500:            ignore_existing: false,
crates/blit-core/tests/transfer_session_roles.rs:93:        compare_mode: ComparisonMode::SizeMtime as i32,
crates/blit-core/tests/transfer_session_roles.rs:255:    // 200 tiny files under nested dirs; force_tar makes the planner's
crates/blit-core/tests/transfer_session_roles.rs:275:        force_tar: true,
crates/blit-core/tests/transfer_session_roles.rs:288:        // Same size, source newer → transferred.
crates/blit-core/tests/transfer_session_roles.rs:289:        ("newer.txt", b"NEW-eight".to_vec(), 1_600_000_200),
crates/blit-core/tests/transfer_session_roles.rs:295:        ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
crates/blit-core/tests/transfer_session_roles.rs:300:        vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
crates/blit-core/tests/transfer_session_roles.rs:344:        compare_mode: ComparisonMode::SizeMtime as i32,
crates/blit-core/tests/transfer_session_roles.rs:620:        checksum: vec![],
crates/blit-core/tests/transfer_session_roles.rs:853:        checksum: vec![],
crates/blit-core/tests/pull_sync_with_spec_wire.rs:106:                    // cleanly without hitting --checksum mismatch
crates/blit-core/tests/pull_sync_with_spec_wire.rs:111:                                server_checksums_enabled: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:256:        force_grpc: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:257:        ignore_existing: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:357:        force_grpc: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:371:        ignore_existing: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:373:        force: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:374:        checksum: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:671:        checksum: vec![],
crates/blit-core/tests/pull_sync_with_spec_wire.rs:692:            server_checksums_enabled: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:727:        spec.force_grpc,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:743:    // `metadata_only` and runs the full force_grpc fallback — headers
crates/blit-core/tests/pull_sync_with_spec_wire.rs:749:            server_checksums_enabled: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:789:    // force_grpc is set, so a daemon steering the session onto a TCP
crates/blit-core/tests/pull_sync_with_spec_wire.rs:826:            server_checksums_enabled: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:856:    assert!(spec.force_grpc);
crates/blit-core/tests/pull_sync_with_spec_wire.rs:951:            server_checksums_enabled: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:969:    spec.compare_mode = blit_core::generated::ComparisonMode::SizeMtime as i32;
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1048:            server_checksums_enabled: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:1079:    spec.compare_mode = blit_core::generated::ComparisonMode::SizeMtime as i32;
crates/blit-core/tests/proto_wire_compat.rs:26://! at 2. `NormalizedTransferOperation::from_spec` enforces the version
crates/blit-core/tests/proto_wire_compat.rs:69:    force_grpc: bool,
crates/blit-core/tests/proto_wire_compat.rs:112:    force_grpc: bool,
crates/blit-core/tests/proto_wire_compat.rs:114:    ignore_existing: bool,
crates/blit-core/tests/proto_wire_compat.rs:200:        compare_mode: blit_core::generated::ComparisonMode::SizeMtime as i32,
crates/blit-core/tests/proto_wire_compat.rs:210:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:211:        ignore_existing: false,
crates/blit-core/tests/proto_wire_compat.rs:263:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:264:        ignore_existing: false,
crates/blit-core/tests/proto_wire_compat.rs:285:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:347:        force_grpc: true,
crates/blit-core/tests/proto_wire_compat.rs:356:    assert!(old.force_grpc);
crates/blit-core/tests/engine_streaming_plan.rs:43:        checksum: vec![],
crates/blit-core/tests/engine_streaming_plan.rs:200:                checksum: None,
crates/blit-core/tests/engine_streaming_plan.rs:202:                compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
crates/blit-core/tests/predictor_streaming.rs:59:            checksum: false,
crates/blit-core/tests/predictor_streaming.rs:60:            compare_mode: CompareModeSnapshot::SizeMtime,
crates/blit-core/tests/predictor_streaming.rs:72:fn streaming_forced_when_prediction_low() -> Result<()> {
crates/blit-core/tests/mirror_planner_tests.rs:1:use blit_core::checksum::ChecksumType;
crates/blit-core/tests/mirror_planner_tests.rs:26:    // Ensure timestamps allow skip (dest newer or equal).
crates/blit-core/tests/mirror_planner_tests.rs:230:fn planner_remote_checksum_mismatch_triggers_copy() -> Result<()> {
crates/blit-core/tests/mirror_planner_tests.rs:235:    fs::write(&file, b"checksum")?;
crates/blit-core/tests/mirror_planner_tests.rs:252:    remote_same.hash = Some(blit_core::checksum::hash_file(&file, ChecksumType::Blake3)?);
crates/blit-core/tests/mirror_planner_tests.rs:318:fn planner_checksum_fetch_logic() -> Result<()> {
crates/blit-core/tests/mirror_planner_tests.rs:324:    fs::write(&dest_path, b"checksum data")?;
crates/blit-core/tests/mirror_planner_tests.rs:341:    remote_match.hash = Some(blit_core::checksum::hash_file(
crates/blit-core/tests/checksum_partial.rs:5:use blit_core::checksum;
crates/blit-core/tests/checksum_partial.rs:21:    let ha = checksum::partial_hash_first_last(&a, 1024 * 1024)?;
crates/blit-core/tests/checksum_partial.rs:22:    let hb = checksum::partial_hash_first_last(&b, 1024 * 1024)?;
crates/blit-core/tests/checksum_partial.rs:29:    use blit_core::checksum::ChecksumType;
crates/blit-core/tests/checksum_partial.rs:30:    use blit_core::copy::file_needs_copy_with_checksum_type as needs;

codex
**Findings**
- None.

VERDICT: PASS — static review found no correctness regressions in `84be1cc`. The dispatch guard is sound: reverting to unconditional `run_destination` makes a DESTINATION initiator fail the complement check with `PROTOCOL_VIOLATION`. The diff adds three `#[tokio::test]` cases and removes none, matching the claimed 1516 -> 1519 test-count direction. I did not run cargo tests or mutation proof in this read-only sandbox.
tokens used
183,684
**Findings**
- None.

VERDICT: PASS — static review found no correctness regressions in `84be1cc`. The dispatch guard is sound: reverting to unconditional `run_destination` makes a DESTINATION initiator fail the complement check with `PROTOCOL_VIOLATION`. The diff adds three `#[tokio::test]` cases and removes none, matching the claimed 1516 -> 1519 test-count direction. I did not run cargo tests or mutation proof in this read-only sandbox.
