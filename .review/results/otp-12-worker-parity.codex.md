Reading additional input from stdin...
OpenAI Codex v0.144.4
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f6369-73c4-7403-b84a-3736c48a35a5
--------
user
Review the diff of commit a76b785 (run: git show a76b785). It implements otp-12-worker-parity under docs/plan/ONE_TRANSFER_PATH.md and .review/findings/otp-12-worker-parity.md: both initiator orientations must settle the same shape-derived stream target, and receiver max_streams=0 must mean unknown rather than a role-specific one-stream cap. Check correctness regressions, the acceptance criteria, FAST/SIMPLE/RELIABLE, byte-identical/StallGuard/cancellation/byte-accounting invariants, and that the test count did not drop. Specifically inspect deadlock or starvation risks while settling resize epochs, handling of resize refusal and concurrent SOURCE events, and performance-regression risk from settling before payload dispatch. Output a concise markdown findings list, each finding with file:line, severity, rationale, then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the commit, its governing plan/finding, and the surrounding transfer/resize paths, then compare verification and test counts against the parent. I’ll focus the report strictly on actionable regressions and the requested invariants.
exec
/bin/zsh -lc "sed -n '1,280p' docs/plan/ONE_TRANSFER_PATH.md; sed -n '1,260p' .review/findings/otp-12-worker-parity.md" in /Users/michael/Dev/blit_v2
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
  on the write-strategy seam. One narrow owner-granted exception
  (D-2026-07-09-1, otp-7b): the CLI end-of-operation fault summary —
  name the file(s) a session fault affected and suggest a re-run —
  lands inside otp-7. Nothing else new rides this plan.

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
      (Instantiation: no same-fs-class 10 GbE pair exists in the
      fleet; the owner designated Mac↔Windows as the closest-spec
      cross-direction rig, 2026-07-10 — otp-2w README §Status. The
      invariance A/B stays valid there because both arms of a pair
      share the same endpoints, so endpoint asymmetry cancels within
      each pair; cross-direction evaluation per D-2026-07-12-1.)
- [ ] **Converge up, measured (codex F4)**: before cutover, the
      corrected symmetric-fs harness records a per-cell baseline of
      the OLD paths, both directions; after cutover, every unified
      cell must be ≤ the better of that cell's two old directions
      + run noise (±10%). A symmetric-but-slower result fails.
      (Evaluation rule on the owner-designated cross-direction rig:
      a cell that meets per-direction converge-up and invariance but
      misses this bar only by a discriminator-attributed destination
      write-path residue counts as satisfied — D-2026-07-12-1;
      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
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
# otp-12-worker-parity — initiator-independent stream target

**Slice**: ONE_TRANSFER_PATH otp-12 acceptance repair. The active plan
requires one sender-owned, receiver-bounded stream policy for both role
assignments; initiator/verb may not change the realized worker count.

## What

The unified session computed the same shape target in both orientations but
did not guarantee reaching it. Resize advances one stream per epoch. Once
`NeedComplete` arrived, the SOURCE resolved only the one epoch already in
flight and stopped proposing. On the same 10,000-file fixture (shape target
8), the source-initiator test settled at 3 streams and the
destination-initiator test at 2.

The destination-initiator admission side also interpreted an advertised
`max_streams = 0` as a one-stream ceiling, while the SOURCE dial correctly
interpreted the wire value as unknown/default. That was a role-specific cap.

## Approach

- Before each payload batch enters the shared elastic send pipeline, drive
  the existing one-stream-per-epoch resize protocol until the currently known
  shape target is settled. Needs and resume hashes continue to be processed
  while acknowledgements are in flight, so the target incorporates all work
  learned during the ramp.
- Stop a refused ramp instead of retrying the same unattainable target under
  fresh epochs forever.
- Centralize receiver stream-ceiling resolution in `dial.rs` and use it on
  both the SOURCE dial and destination-initiator admission path. Wire value
  zero remains unknown/default, never one.
- Strengthen both role-orientation integration pins from merely `> 1` to the
  exact shared target `8`; the destination-initiator case explicitly carries
  the legal unknown-capacity value.

## Files

- `crates/blit-core/src/dial.rs`
- `crates/blit-core/src/transfer_session/mod.rs`
- `crates/blit-core/tests/transfer_session_roles.rs`

## Tests

- Guard proof before the implementation: the strengthened role pins failed
  at 3 streams (SOURCE initiator) and 2 streams (DESTINATION initiator).
- Separate zero-capacity guard proof: after the ramp fix but before the
  shared ceiling fix, the DESTINATION-initiator pin failed at 1 stream.
- After both fixes: the two exact-target pins pass at 8 and the complete
  `transfer_session_roles` integration target passes 39/39.
- Full workspace gate passes: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (1488 tests, 2 ignored; no failures).

## Known gaps

- Socket acquisition remains connection-role-specific by design: the network
  initiator dials the responder so a pull caller does not need an inbound
  listener through NAT/firewalls. Byte work is still one SOURCE send pipeline
  and one DESTINATION receive pipeline. This slice removes worker-count drift;
  it does not invert that network topology.
- No hardware benchmark is part of this code slice. The existing otp-12
  acceptance rigs remain the performance proof after review.

## Reviewer comments

(appended after the codex round)

exec
/bin/zsh -lc "sed -n '1,260p' AGENTS.md; sed -n '1,260p' .agents/repo-guidance.md; sed -n '1,220p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# Agent Guidance

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code. Do not expand scope without approval.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's `.agents/` files; reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Record important repo facts, decisions, invariants, verification rules, non-goals, and open questions in repo files, or explicitly report them as unrecorded. Write them generalized, tied to repo evidence or explicit human intent, so they make sense without the conversation that produced them — never as transient chat wording. Label inferred-but-unverified facts as assumptions until repo evidence or explicit human approval supports them.
- Keep one canonical location for each durable truth. Prefer pointers over duplicating the same rule; never keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point (`.agents/state.md`). Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
- Prefer the smallest durable guidance set that fits the repo.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/`, with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, operator names — are portable and allowed.
- `AGENTS.md` is the toolkit template, installed and replaced whole by governance refresh; no agent hand-edits it. Durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds — question it, do not perform it.

## Session Startup

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Clone freshness: before trusting `.agents/state.md`, compare this clone against its canonical remote with a read-only check (`git ls-remote <remote> HEAD` against the local ref). Behind or diverged — say so and treat recorded state as possibly stale; unreachable — proceed with a one-line caveat, never block.
3. This repo ships a compaction re-ground hook (Claude Code; other harnesses only as listed in the toolkit's harness-capabilities record); if your harness gates hooks until the workspace is trusted, say what the hook does and run the trust step only on an explicit go — never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`, extended by `.agents/repo-guidance.md` (extends, never overrides).
3. `.agents/state.md` for current work; `.agents/decisions.md` for settled decisions; approved `.agents/playbooks/*`.
4. Current code, tests, and CI as evidence for behavior.
5. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: a fast save-my-place snapshot — seconds, not minutes. Update `.agents/state.md` `## Now` / `## Next` (and `## Blockers` if something is live) so the next session resumes without chat context: in-flight work, next action, stop. No archive rotation, no re-verification sweep, no mandatory re-anchoring of volatile facts — that hygiene belongs to `drift`. Machine-specific facts (CLI paths, local tool versions, host layout) go to the tracked `.agents/machines.md` under a heading for the current machine, dated, created on first use — never into `.agents/state.md`, which stays portable and may at most point there.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (CI state, counts) carry `as of <commit>` and are re-verified or dropped; push status is never recorded in state files — git owns it, sessions check it live, and unpushed work is mentioned only in the moment it matters — so any recorded push-state line is deleted on sight, not refreshed; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work. Plan documents are written for agents, never the owner: self-contained and technical, implementable by a completely cold, less-capable agent — no human-facing summary prose, no chat or session references that need the originating conversation to make sense. The owner does not read plan documents; present every decision a plan needs in chat as roughly 25-50 plain-English words — the problem, the change, the cost or risk — one decision at a time, never a batch, no jargon. Record the owner's approved wording durably (the decisions log, the plan's status line) so the approval survives the chat.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows; this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-guidance.md` (Verification).

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
- This section is the canonical home for the verification commands (the
  `.agents/repo-map.json` mirror was retired 2026-07-08 with the toolkit's
  JSON layer).

## Remotes & Sync

- `origin` — `http://q:3000/michael/blit_v2.git` (**LAN gitea**). This is
  what a bare `git push` / `git ls-remote origin` talks to.
- `github` — `https://github.com/roethlar/Blit.git` (**GitHub**).
- **`git push` does NOT update GitHub.** The two remotes are independent
  and nothing auto-syncs them: pushing `origin` moves the LAN gitea only,
  and GitHub needs its own explicit `git push github`. Either can lag the
  other by many commits. A ref-listing before a push must therefore name
  the remote's **URL**, not just "origin" — the name alone does not say
  which host is being published to.
- **CORRECTED 2026-07-13** against `git remote -v` (evidence in the
  session's push output: a bare `git push` printed
  `To http://q:3000/michael/blit_v2.git`, while `git push github` printed
  `To https://github.com/roethlar/Blit.git` and fast-forwarded GitHub
  `6d37a22..be1989d` — i.e. GitHub had been dozens of commits stale while
  every agent push went to the mirror). The prior text had the two names
  **inverted** (`origin` = GitHub, plus a `gitea` remote that does not
  exist) and asserted it had been "verified against `git remote -v`
  2026-07-04" while disparaging an earlier revision that was in fact
  correct. Do not re-invert this without re-reading `git remote -v`.
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
  PascalCase, constants SHOUT_CASE; match existing names
  (`transfer_session`, `TransferSession`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, the unified
  `transfer_session` — every transfer, local included, since otp-11);
  most logic and unit tests live here. New modules get re-exported in
  `crates/blit-core/src/lib.rs`.
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
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-guidance.md` (Verification)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

exec
/bin/zsh -lc 'git show --stat --oneline --decorate --no-renames a76b785; git show --format=fuller --no-ext-diff --no-renames a76b785' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-hkvo0F5a' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-N1cJ311c' (errno=Operation not permitted)
a76b785 (HEAD -> master) Fix otp-12 worker-count parity
 .review/findings/otp-12-worker-parity.md         | 66 ++++++++++++++++++
 crates/blit-core/src/dial.rs                     | 17 +++--
 crates/blit-core/src/transfer_session/mod.rs     | 89 +++++++++++++++++++++---
 crates/blit-core/tests/transfer_session_roles.rs | 38 ++++++----
 4 files changed, 180 insertions(+), 30 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-u3Z56OQr' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rqoLO89w' (errno=Operation not permitted)
commit a76b7854d6e2e662248fadf3ea5ffbe0a672b6d6
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 21:34:13 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 21:34:13 2026 -0400

    Fix otp-12 worker-count parity

diff --git a/.review/findings/otp-12-worker-parity.md b/.review/findings/otp-12-worker-parity.md
new file mode 100644
index 0000000..327155b
--- /dev/null
+++ b/.review/findings/otp-12-worker-parity.md
@@ -0,0 +1,66 @@
+# otp-12-worker-parity — initiator-independent stream target
+
+**Slice**: ONE_TRANSFER_PATH otp-12 acceptance repair. The active plan
+requires one sender-owned, receiver-bounded stream policy for both role
+assignments; initiator/verb may not change the realized worker count.
+
+## What
+
+The unified session computed the same shape target in both orientations but
+did not guarantee reaching it. Resize advances one stream per epoch. Once
+`NeedComplete` arrived, the SOURCE resolved only the one epoch already in
+flight and stopped proposing. On the same 10,000-file fixture (shape target
+8), the source-initiator test settled at 3 streams and the
+destination-initiator test at 2.
+
+The destination-initiator admission side also interpreted an advertised
+`max_streams = 0` as a one-stream ceiling, while the SOURCE dial correctly
+interpreted the wire value as unknown/default. That was a role-specific cap.
+
+## Approach
+
+- Before each payload batch enters the shared elastic send pipeline, drive
+  the existing one-stream-per-epoch resize protocol until the currently known
+  shape target is settled. Needs and resume hashes continue to be processed
+  while acknowledgements are in flight, so the target incorporates all work
+  learned during the ramp.
+- Stop a refused ramp instead of retrying the same unattainable target under
+  fresh epochs forever.
+- Centralize receiver stream-ceiling resolution in `dial.rs` and use it on
+  both the SOURCE dial and destination-initiator admission path. Wire value
+  zero remains unknown/default, never one.
+- Strengthen both role-orientation integration pins from merely `> 1` to the
+  exact shared target `8`; the destination-initiator case explicitly carries
+  the legal unknown-capacity value.
+
+## Files
+
+- `crates/blit-core/src/dial.rs`
+- `crates/blit-core/src/transfer_session/mod.rs`
+- `crates/blit-core/tests/transfer_session_roles.rs`
+
+## Tests
+
+- Guard proof before the implementation: the strengthened role pins failed
+  at 3 streams (SOURCE initiator) and 2 streams (DESTINATION initiator).
+- Separate zero-capacity guard proof: after the ramp fix but before the
+  shared ceiling fix, the DESTINATION-initiator pin failed at 1 stream.
+- After both fixes: the two exact-target pins pass at 8 and the complete
+  `transfer_session_roles` integration target passes 39/39.
+- Full workspace gate passes: `cargo fmt --all -- --check`,
+  `cargo clippy --workspace --all-targets -- -D warnings`, and
+  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
+
+## Known gaps
+
+- Socket acquisition remains connection-role-specific by design: the network
+  initiator dials the responder so a pull caller does not need an inbound
+  listener through NAT/firewalls. Byte work is still one SOURCE send pipeline
+  and one DESTINATION receive pipeline. This slice removes worker-count drift;
+  it does not invert that network topology.
+- No hardware benchmark is part of this code slice. The existing otp-12
+  acceptance rigs remain the performance proof after review.
+
+## Reviewer comments
+
+(appended after the codex round)
diff --git a/crates/blit-core/src/dial.rs b/crates/blit-core/src/dial.rs
index 611820c..5ec1eaf 100644
--- a/crates/blit-core/src/dial.rs
+++ b/crates/blit-core/src/dial.rs
@@ -81,6 +81,18 @@ pub fn local_receiver_capacity() -> CapacityProfile {
     }
 }
 
+/// Resolve the receiver's advertised stream ceiling with the wire
+/// contract's `0 = unknown` semantics. Both the SOURCE-owned dial and the
+/// DESTINATION's resize admission must call this one function; otherwise a
+/// destination-initiated session can interpret the same profile as a
+/// one-stream cap while its source interprets it as the default ceiling.
+pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
+    profile
+        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
+        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
+        .clamp(1, DIAL_CEILING_MAX_STREAMS)
+}
+
 /// The one mutable tuning object for a transfer.
 #[derive(Debug)]
 pub struct TransferDial {
@@ -144,15 +156,12 @@ impl TransferDial {
     pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
         let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
         let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
-        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
+        let ceiling_streams = receiver_stream_ceiling(profile);
         let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
         if let Some(profile) = profile {
             if profile.max_chunk_bytes > 0 {
                 ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
             }
-            if profile.max_streams > 0 {
-                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
-            }
             if profile.max_inflight_bytes > 0 {
                 // The in-flight budget bounds the CHUNK ceiling first
                 // (codex ue-r2-1e F1: with max_chunk unknown, a budget
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 60ba57a..50e7ebf 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -1413,10 +1413,25 @@ async fn source_send_half(
                 Some(dp) => {
                     // sf-2: correct the stream count toward the shape the
                     // accumulated need list implies before queueing this
-                    // batch (one ADD per epoch; a no-op while one is in
-                    // flight or the shape wants no more).
+                    // batch. Settle the whole shape-derived target before
+                    // handing payloads to the pipeline: otherwise the
+                    // one-ADD-per-epoch ramp races NeedComplete/payload
+                    // drain, so a fast transfer can finish at a different
+                    // worker count depending on which endpoint initiated.
                     maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                         .await?;
+                    settle_shape_resizes(
+                        &mut events,
+                        &mut pending,
+                        &mut resume,
+                        &mut need_complete,
+                        &mut needed_bytes,
+                        &mut needed_count,
+                        dp,
+                        tx,
+                        &mut pending_resize,
+                    )
+                    .await?;
                     let payloads =
                         diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
                     // A cancel while earlier batches are actively moving
@@ -1475,6 +1490,18 @@ async fn source_send_half(
                     // zero-knowledge single stream.
                     maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                         .await?;
+                    settle_shape_resizes(
+                        &mut events,
+                        &mut pending,
+                        &mut resume,
+                        &mut need_complete,
+                        &mut needed_bytes,
+                        &mut needed_count,
+                        dp,
+                        tx,
+                        &mut pending_resize,
+                    )
+                    .await?;
                     let payloads = ready
                         .into_iter()
                         .map(|(header, hashes)| TransferPayload::ResumeFile {
@@ -1772,12 +1799,16 @@ async fn process_source_event(
                 dp.add_stream(&pending_r.sub_token).await?;
                 dp.dial()
                     .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
+                // Ramp one stream per accepted epoch: propose the next ADD.
+                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
             } else {
                 dp.dial()
                     .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
+                // A refusal is terminal for this shape ramp. Retrying the
+                // same unattainable target under a fresh epoch would loop
+                // forever; the settled live set still carries the transfer.
+                Ok(())
             }
-            // Ramp one stream per accepted epoch: propose the next ADD.
-            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
         }
         SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
             "TransferSummary before SourceDone",
@@ -1814,6 +1845,46 @@ async fn maybe_propose_resize(
     Ok(())
 }
 
+/// Drive the one-stream-per-epoch shape ramp to its currently known target
+/// before payload dispatch. Needs and resume hashes may continue arriving
+/// while an ack is in flight, so process the shared SOURCE event lane rather
+/// than waiting for only an ack. Each accepted ack proposes the next epoch
+/// from the latest accumulated shape; the loop ends only when no proposal is
+/// outstanding (target reached or the destination refused growth).
+#[allow(clippy::too_many_arguments)]
+async fn settle_shape_resizes(
+    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
+    pending: &mut Vec<FileHeader>,
+    resume: &mut ResumeSendState,
+    need_complete: &mut bool,
+    needed_bytes: &mut u64,
+    needed_count: &mut usize,
+    data_plane: &data_plane::SourceDataPlane,
+    tx: &mut Box<dyn FrameTx>,
+    pending_resize: &mut Option<data_plane::PendingResize>,
+) -> Result<()> {
+    while pending_resize.is_some() {
+        let event = events.recv().await.ok_or_else(|| {
+            eyre::Report::new(SessionFault::internal(
+                "source receive half ended during data-plane shape resize",
+            ))
+        })?;
+        process_source_event(
+            event,
+            pending,
+            resume,
+            need_complete,
+            needed_bytes,
+            needed_count,
+            Some(data_plane),
+            tx,
+            pending_resize,
+        )
+        .await?;
+    }
+    Ok(())
+}
+
 /// Block for the ack of the one in-flight resize and dial its socket (or
 /// settle it refused). Does NOT propose further — it resolves exactly the
 /// pending proposal so the destination's armed slot is consumed before we
@@ -2729,13 +2800,9 @@ async fn destination_session(
                     // host's fresh local reading (codex otp-5b-2 F1). On a
                     // Resize frame the initiator dials the epoch-N socket (vs
                     // the responder path's arm).
-                    let ceiling = negotiated
-                        .open
-                        .receiver_capacity
-                        .as_ref()
-                        .map(|c| c.max_streams)
-                        .unwrap_or(0)
-                        .max(1) as usize;
+                    let ceiling = crate::dial::receiver_stream_ceiling(
+                        negotiated.open.receiver_capacity.as_ref(),
+                    );
                     (
                         Some(data_plane::DestRecvPlane::Initiator(run)),
                         initial,
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
index 1139b8a..7a9ef4e 100644
--- a/crates/blit-core/tests/transfer_session_roles.rs
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -17,9 +17,10 @@ use std::time::Duration;
 
 use blit_core::generated::transfer_frame::Frame;
 use blit_core::generated::{
-    session_error, BlockHashList, ComparisonMode, FileData, FileHeader, FilterSpec,
-    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, ResumeSettings, SessionError,
-    SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole, TransferSummary,
+    session_error, BlockHashList, CapacityProfile, ComparisonMode, FileData, FileHeader,
+    FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, ResumeSettings,
+    SessionError, SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole,
+    TransferSummary,
 };
 use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
 use blit_core::remote::transfer::{PreparedPayload, TransferPayload};
@@ -1157,7 +1158,7 @@ async fn block_hashes_without_a_held_resume_need_fault_the_source() {
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
+async fn many_tiny_files_reach_shape_target_when_source_initiates() {
     // sf-2 pin ported onto the unified session (otp-4b-2). The responder
     // grants the zero-knowledge single stream (no manifest seen at
     // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
@@ -1219,10 +1220,10 @@ async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
     let streams = outcome
         .data_plane_streams
         .expect("data plane ran, stream count recorded");
-    assert!(
-        streams > 1,
-        "a {FILE_COUNT}-file transfer must correct the single-stream grant \
-         upward via shape resize; settled at {streams}"
+    assert_eq!(
+        streams, 8,
+        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
+         target regardless of which endpoint initiated the session"
     );
     assert_trees_identical(&src_root, &dst_root);
 }
@@ -1238,7 +1239,7 @@ async fn pull_data_plane_single_stream_lands_bytes() {
     // 127.0.0.1). Single-stream because this 4-file tree's shape wants only
     // one stream — the pull data plane CAN resize (otp-5b-2), but a small
     // need list never crosses the shape threshold; the resize itself is
-    // pinned by `pull_data_plane_shape_corrects_to_more_than_one_stream`.
+    // pinned by `many_tiny_files_reach_shape_target_when_destination_initiates`.
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     let dst_root = tmp.path().join("dst");
@@ -1307,9 +1308,9 @@ async fn pull_data_plane_single_stream_lands_bytes() {
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
+async fn many_tiny_files_reach_shape_target_when_destination_initiates() {
     // otp-5b-2: the sf-2 shape correction in the PULL direction — the
-    // mirror of `many_tiny_files_shape_correct_to_more_than_one_stream`
+    // mirror of `many_tiny_files_reach_shape_target_when_source_initiates`
     // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
     // sockets it grows to) and the SOURCE is the *responder* (accepts them
     // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
@@ -1332,6 +1333,13 @@ async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
         initiator_role: TransferRole::Destination as i32,
         compare_mode: ComparisonMode::SizeMtime as i32,
         in_stream_bytes: false,
+        // Wire contract: zero means unknown, not a one-stream cap. Pin it
+        // on the destination-initiator orientation, where this end both
+        // advertises and enforces the receiver ceiling.
+        receiver_capacity: Some(CapacityProfile {
+            max_streams: 0,
+            ..Default::default()
+        }),
         ..Default::default()
     };
     let source_cfg = SourceSessionConfig {
@@ -1369,10 +1377,10 @@ async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
     let streams = outcome
         .data_plane_streams
         .expect("data plane ran, stream count recorded");
-    assert!(
-        streams > 1,
-        "a {FILE_COUNT}-file PULL transfer must correct the single-stream \
-         grant upward via shape resize; settled at {streams}"
+    assert_eq!(
+        streams, 8,
+        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
+         target regardless of which endpoint initiated the session"
     );
     assert_trees_identical(&src_root, &dst_root);
 }

exec
/bin/zsh -lc 'git status --short; git remote -v; git rev-parse a76b785''^; git rev-parse a76b785' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GRMgtEO4' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NV6IN8bV' (errno=Operation not permitted)
?? .review/results/otp-12-worker-parity.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fz5nVqPL' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RSb40VgJ' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-VhzdF2uw' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-IOmKoNKY' (errno=Operation not permitted)
6b0f01c5f8e2ed679d2f88e10df0ae8da60939d2
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2fhdgMWi' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-PdaygpTw' (errno=Operation not permitted)
a76b7854d6e2e662248fadf3ea5ffbe0a672b6d6

exec
/bin/zsh -lc 'git diff a76b785''^ a76b785 -- crates/blit-core/src/dial.rs' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-fWhihd3Y' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-J7KjeOaC' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/dial.rs b/crates/blit-core/src/dial.rs
index 611820c..5ec1eaf 100644
--- a/crates/blit-core/src/dial.rs
+++ b/crates/blit-core/src/dial.rs
@@ -81,6 +81,18 @@ pub fn local_receiver_capacity() -> CapacityProfile {
     }
 }
 
+/// Resolve the receiver's advertised stream ceiling with the wire
+/// contract's `0 = unknown` semantics. Both the SOURCE-owned dial and the
+/// DESTINATION's resize admission must call this one function; otherwise a
+/// destination-initiated session can interpret the same profile as a
+/// one-stream cap while its source interprets it as the default ceiling.
+pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
+    profile
+        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
+        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
+        .clamp(1, DIAL_CEILING_MAX_STREAMS)
+}
+
 /// The one mutable tuning object for a transfer.
 #[derive(Debug)]
 pub struct TransferDial {
@@ -144,15 +156,12 @@ impl TransferDial {
     pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
         let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
         let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
-        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
+        let ceiling_streams = receiver_stream_ceiling(profile);
         let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
         if let Some(profile) = profile {
             if profile.max_chunk_bytes > 0 {
                 ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
             }
-            if profile.max_streams > 0 {
-                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
-            }
             if profile.max_inflight_bytes > 0 {
                 // The in-flight budget bounds the CHUNK ceiling first
                 // (codex ue-r2-1e F1: with max_chunk unknown, a budget

exec
/bin/zsh -lc 'git diff a76b785''^ a76b785 -- crates/blit-core/tests/transfer_session_roles.rs' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git diff a76b785''^ a76b785 -- crates/blit-core/src/transfer_session/mod.rs' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lY55Dmoi' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LaJHi0Wb' (errno=Operation not permitted)
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
index 1139b8a..7a9ef4e 100644
--- a/crates/blit-core/tests/transfer_session_roles.rs
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -17,9 +17,10 @@ use std::time::Duration;
 
 use blit_core::generated::transfer_frame::Frame;
 use blit_core::generated::{
-    session_error, BlockHashList, ComparisonMode, FileData, FileHeader, FilterSpec,
-    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, ResumeSettings, SessionError,
-    SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole, TransferSummary,
+    session_error, BlockHashList, CapacityProfile, ComparisonMode, FileData, FileHeader,
+    FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, ResumeSettings,
+    SessionError, SessionHello, SessionOpen, SourceDone, TransferFrame, TransferRole,
+    TransferSummary,
 };
 use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
 use blit_core::remote::transfer::{PreparedPayload, TransferPayload};
@@ -1157,7 +1158,7 @@ async fn block_hashes_without_a_held_resume_need_fault_the_source() {
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
+async fn many_tiny_files_reach_shape_target_when_source_initiates() {
     // sf-2 pin ported onto the unified session (otp-4b-2). The responder
     // grants the zero-knowledge single stream (no manifest seen at
     // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
@@ -1219,10 +1220,10 @@ async fn many_tiny_files_shape_correct_to_more_than_one_stream() {
     let streams = outcome
         .data_plane_streams
         .expect("data plane ran, stream count recorded");
-    assert!(
-        streams > 1,
-        "a {FILE_COUNT}-file transfer must correct the single-stream grant \
-         upward via shape resize; settled at {streams}"
+    assert_eq!(
+        streams, 8,
+        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
+         target regardless of which endpoint initiated the session"
     );
     assert_trees_identical(&src_root, &dst_root);
 }
@@ -1238,7 +1239,7 @@ async fn pull_data_plane_single_stream_lands_bytes() {
     // 127.0.0.1). Single-stream because this 4-file tree's shape wants only
     // one stream — the pull data plane CAN resize (otp-5b-2), but a small
     // need list never crosses the shape threshold; the resize itself is
-    // pinned by `pull_data_plane_shape_corrects_to_more_than_one_stream`.
+    // pinned by `many_tiny_files_reach_shape_target_when_destination_initiates`.
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     let dst_root = tmp.path().join("dst");
@@ -1307,9 +1308,9 @@ async fn pull_data_plane_single_stream_lands_bytes() {
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
+async fn many_tiny_files_reach_shape_target_when_destination_initiates() {
     // otp-5b-2: the sf-2 shape correction in the PULL direction — the
-    // mirror of `many_tiny_files_shape_correct_to_more_than_one_stream`
+    // mirror of `many_tiny_files_reach_shape_target_when_source_initiates`
     // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
     // sockets it grows to) and the SOURCE is the *responder* (accepts them
     // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
@@ -1332,6 +1333,13 @@ async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
         initiator_role: TransferRole::Destination as i32,
         compare_mode: ComparisonMode::SizeMtime as i32,
         in_stream_bytes: false,
+        // Wire contract: zero means unknown, not a one-stream cap. Pin it
+        // on the destination-initiator orientation, where this end both
+        // advertises and enforces the receiver ceiling.
+        receiver_capacity: Some(CapacityProfile {
+            max_streams: 0,
+            ..Default::default()
+        }),
         ..Default::default()
     };
     let source_cfg = SourceSessionConfig {
@@ -1369,10 +1377,10 @@ async fn pull_data_plane_shape_corrects_to_more_than_one_stream() {
     let streams = outcome
         .data_plane_streams
         .expect("data plane ran, stream count recorded");
-    assert!(
-        streams > 1,
-        "a {FILE_COUNT}-file PULL transfer must correct the single-stream \
-         grant upward via shape resize; settled at {streams}"
+    assert_eq!(
+        streams, 8,
+        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
+         target regardless of which endpoint initiated the session"
     );
     assert_trees_identical(&src_root, &dst_root);
 }

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lQwtfLE0' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-xxbUsCOk' (errno=Operation not permitted)
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index 60ba57a..50e7ebf 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -1413,10 +1413,25 @@ async fn source_send_half(
                 Some(dp) => {
                     // sf-2: correct the stream count toward the shape the
                     // accumulated need list implies before queueing this
-                    // batch (one ADD per epoch; a no-op while one is in
-                    // flight or the shape wants no more).
+                    // batch. Settle the whole shape-derived target before
+                    // handing payloads to the pipeline: otherwise the
+                    // one-ADD-per-epoch ramp races NeedComplete/payload
+                    // drain, so a fast transfer can finish at a different
+                    // worker count depending on which endpoint initiated.
                     maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                         .await?;
+                    settle_shape_resizes(
+                        &mut events,
+                        &mut pending,
+                        &mut resume,
+                        &mut need_complete,
+                        &mut needed_bytes,
+                        &mut needed_count,
+                        dp,
+                        tx,
+                        &mut pending_resize,
+                    )
+                    .await?;
                     let payloads =
                         diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
                     // A cancel while earlier batches are actively moving
@@ -1475,6 +1490,18 @@ async fn source_send_half(
                     // zero-knowledge single stream.
                     maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                         .await?;
+                    settle_shape_resizes(
+                        &mut events,
+                        &mut pending,
+                        &mut resume,
+                        &mut need_complete,
+                        &mut needed_bytes,
+                        &mut needed_count,
+                        dp,
+                        tx,
+                        &mut pending_resize,
+                    )
+                    .await?;
                     let payloads = ready
                         .into_iter()
                         .map(|(header, hashes)| TransferPayload::ResumeFile {
@@ -1772,12 +1799,16 @@ async fn process_source_event(
                 dp.add_stream(&pending_r.sub_token).await?;
                 dp.dial()
                     .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
+                // Ramp one stream per accepted epoch: propose the next ADD.
+                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
             } else {
                 dp.dial()
                     .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
+                // A refusal is terminal for this shape ramp. Retrying the
+                // same unattainable target under a fresh epoch would loop
+                // forever; the settled live set still carries the transfer.
+                Ok(())
             }
-            // Ramp one stream per accepted epoch: propose the next ADD.
-            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
         }
         SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
             "TransferSummary before SourceDone",
@@ -1814,6 +1845,46 @@ async fn maybe_propose_resize(
     Ok(())
 }
 
+/// Drive the one-stream-per-epoch shape ramp to its currently known target
+/// before payload dispatch. Needs and resume hashes may continue arriving
+/// while an ack is in flight, so process the shared SOURCE event lane rather
+/// than waiting for only an ack. Each accepted ack proposes the next epoch
+/// from the latest accumulated shape; the loop ends only when no proposal is
+/// outstanding (target reached or the destination refused growth).
+#[allow(clippy::too_many_arguments)]
+async fn settle_shape_resizes(
+    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
+    pending: &mut Vec<FileHeader>,
+    resume: &mut ResumeSendState,
+    need_complete: &mut bool,
+    needed_bytes: &mut u64,
+    needed_count: &mut usize,
+    data_plane: &data_plane::SourceDataPlane,
+    tx: &mut Box<dyn FrameTx>,
+    pending_resize: &mut Option<data_plane::PendingResize>,
+) -> Result<()> {
+    while pending_resize.is_some() {
+        let event = events.recv().await.ok_or_else(|| {
+            eyre::Report::new(SessionFault::internal(
+                "source receive half ended during data-plane shape resize",
+            ))
+        })?;
+        process_source_event(
+            event,
+            pending,
+            resume,
+            need_complete,
+            needed_bytes,
+            needed_count,
+            Some(data_plane),
+            tx,
+            pending_resize,
+        )
+        .await?;
+    }
+    Ok(())
+}
+
 /// Block for the ack of the one in-flight resize and dial its socket (or
 /// settle it refused). Does NOT propose further — it resolves exactly the
 /// pending proposal so the destination's armed slot is consumed before we
@@ -2729,13 +2800,9 @@ async fn destination_session(
                     // host's fresh local reading (codex otp-5b-2 F1). On a
                     // Resize frame the initiator dials the epoch-N socket (vs
                     // the responder path's arm).
-                    let ceiling = negotiated
-                        .open
-                        .receiver_capacity
-                        .as_ref()
-                        .map(|c| c.max_streams)
-                        .unwrap_or(0)
-                        .max(1) as usize;
+                    let ceiling = crate::dial::receiver_stream_ceiling(
+                        negotiated.open.receiver_capacity.as_ref(),
+                    );
                     (
                         Some(data_plane::DestRecvPlane::Initiator(run)),
                         initial,

exec
/bin/zsh -lc 'git show -s --format=fuller a76b785; git diff --stat a76b785''^ a76b785; git diff --numstat a76b785''^ a76b785' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-j2hskqwB' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-3wOzpOPf' (errno=Operation not permitted)
commit a76b7854d6e2e662248fadf3ea5ffbe0a672b6d6
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 21:34:13 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 21:34:13 2026 -0400

    Fix otp-12 worker-count parity
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-AaEhcIik' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-cXJZiMxR' (errno=Operation not permitted)
 .review/findings/otp-12-worker-parity.md         | 66 ++++++++++++++++++
 crates/blit-core/src/dial.rs                     | 17 +++--
 crates/blit-core/src/transfer_session/mod.rs     | 89 +++++++++++++++++++++---
 crates/blit-core/tests/transfer_session_roles.rs | 38 ++++++----
 4 files changed, 180 insertions(+), 30 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Mr6v8mIb' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ju8IHPqt' (errno=Operation not permitted)
66	0	.review/findings/otp-12-worker-parity.md
13	4	crates/blit-core/src/dial.rs
78	11	crates/blit-core/src/transfer_session/mod.rs
23	15	crates/blit-core/tests/transfer_session_roles.rs

exec
/bin/zsh -lc 'git status --short; git branch --show-current; git log -8 --oneline --decorate; git remote -v; git ls-remote origin HEAD; git rev-parse origin/master 2>/dev/null || true' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-BE67AbSW' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-5tEjJ8FY' (errno=Operation not permitted)
?? .review/results/otp-12-worker-parity.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XMueFNpk' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-CWNhldoI' (errno=Operation not permitted)
master
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ybwJmDAI' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-SrRPYtsZ' (errno=Operation not permitted)
a76b785 (HEAD -> master) Fix otp-12 worker-count parity
6b0f01c revert: remove this session P1-mechanism investigation notes (owner: throw away trash)
e83f9b9 pf-1: MEASURED on loopback — pull ramps to 2 streams vs push 4 (cold cache)
e328fcf pf-1: connect P1 to the ALREADY-MEASURED streams-dont-scale finding (stop re-deriving it)
e8c87aa pf-1: CORRECT the record — codex refuted my starvation story; suspect narrowed to ramp latency
c91ebb0 pf-1: PIN P1 mechanism by code reading — the pull dest control loop blocks on the resize dial
2932a6d handoff (52nd): round 11 fixed, round-12 consensus — P1 is REAL, Mac<->Mac parked pending owner direction
df461f9 revert: keep D-2026-07-14-5 in DECISIONS.md only; leave the review-loop doc to the toolkit
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-i02eA4vB' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-PkEYOqOG' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-qd2lA1Cy' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-O8bn38vy' (errno=Operation not permitted)
fatal: unable to access 'http://q:3000/michael/blit_v2.git/': Could not resolve host: q
9e826dd41a6da6223a6fb3cc154f63e5514da1fb

codex
The central parity fix is small, but its behavior depends on the dial state machine and the ordering of control frames. I’m tracing those call sites now—especially whether a refusal is remembered across later need batches and whether sequential resize acknowledgements can block first-byte progress.
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/dial.rs | sed -n '1,300p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2500,2925p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1220,1915p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	//! The single live transfer dial (`ue-r2-1e`, REV4 Design §4).
     2	//!
     3	//! Sender-owned, receiver-bounded: the byte sender constructs one
     4	//! `TransferDial` per transfer, clamped by the receiver's advertised
     5	//! [`CapacityProfile`] (the `ue-r2-1b` wire fields), starts at the
     6	//! conservative floor (D-2026-06-20-1/-2: no probe phase, no
     7	//! size-gated start — begin immediately and tune live), and a tuner
     8	//! steps the cheap dials from the PR1 stream telemetry.
     9	//!
    10	//! Mutability model (the C-ready seam `ue-r2-2` builds on):
    11	//! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
    12	//!   tuner steps mid-transfer. Consumers read them when a session,
    13	//!   pipeline, or fallback batch is set up, so a step takes effect for
    14	//!   sockets/batches started afterwards (epoch-N resize adds, the next
    15	//!   gRPC-fallback batch) — existing sessions keep their snapshot.
    16	//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
    17	//!   read when a socket/pool is built; changes affect sockets opened
    18	//!   afterwards (no setsockopt on live sockets this slice).
    19	//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
    20	//!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
    21	//!   dial only carries the negotiation-time value and the
    22	//!   profile-clamped ceiling.
    23	//!
    24	//! This replaces the size-keyed `determine_remote_tuning` static
    25	//! ladder: the ladder's floor tier is the dial's start, its top tier
    26	//! is the dial's default ceiling, and everything between is reached by
    27	//! ramping on evidence instead of guessing from `total_bytes`.
    28	
    29	use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
    30	use std::sync::Arc;
    31	
    32	use crate::generated::CapacityProfile;
    33	
    34	const MIB: usize = 1024 * 1024;
    35	
    36	/// Floor (conservative start) values — the old ladder's smallest tier.
    37	pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
    38	pub const DIAL_FLOOR_PREFETCH: usize = 4;
    39	pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;
    40	pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;
    41	
    42	/// Default ceilings — the old ladder's top tier (a fully ramped dial
    43	/// matches today's best static behavior).
    44	pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
    45	pub const DIAL_CEILING_PREFETCH: usize = 32;
    46	pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
    47	pub const DIAL_CEILING_TCP_BUFFER_BYTES: usize = 8 * MIB;
    48	
    49	/// Tuner policy (initial, deliberately simple): sampled every
    50	/// [`DIAL_TUNER_TICK`]; below [`DIAL_STEP_UP_BLOCKED_RATIO`] blocked
    51	/// time the pipe is not back-pressured → step up; above
    52	/// [`DIAL_STEP_DOWN_BLOCKED_RATIO`] → step down. One step per tick
    53	/// (hysteresis by construction).
    54	pub const DIAL_TUNER_TICK: std::time::Duration = std::time::Duration::from_millis(500);
    55	pub const DIAL_STEP_UP_BLOCKED_RATIO: f64 = 0.05;
    56	pub const DIAL_STEP_DOWN_BLOCKED_RATIO: f64 = 0.30;
    57	
    58	/// Resize policy (`ue-r2-2`): streams are the EXPENSIVE dial — a step
    59	/// costs a control round-trip plus a TCP connect — so they move only
    60	/// after the cheap dials are pinned at a bound and the signal has held
    61	/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
    62	/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
    63	/// epoch (the wire carries one `sub_token` per ADD).
    64	pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
    65	pub const RESIZE_SUSTAIN_TICKS: i32 = 2;
    66	
    67	/// The capacity profile this host advertises when it is the byte
    68	/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
    69	/// fields). Honest system facts only — fields we cannot measure yet
    70	/// stay 0 (= unknown per the wire contract), never fabricated:
    71	/// ceilings mirror what today's receive paths actually accept.
    72	pub fn local_receiver_capacity() -> CapacityProfile {
    73	    CapacityProfile {
    74	        cpu_cores: num_cpus::get() as u32,
    75	        drain_class: 0,
    76	        load_percent: 0,
    77	        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
    78	        drain_rate_bytes_per_sec: 0,
    79	        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
    80	        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    81	    }
    82	}
    83	
    84	/// Resolve the receiver's advertised stream ceiling with the wire
    85	/// contract's `0 = unknown` semantics. Both the SOURCE-owned dial and the
    86	/// DESTINATION's resize admission must call this one function; otherwise a
    87	/// destination-initiated session can interpret the same profile as a
    88	/// one-stream cap while its source interprets it as the default ceiling.
    89	pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
    90	    profile
    91	        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
    92	        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
    93	        .clamp(1, DIAL_CEILING_MAX_STREAMS)
    94	}
    95	
    96	/// The one mutable tuning object for a transfer.
    97	#[derive(Debug)]
    98	pub struct TransferDial {
    99	    chunk_bytes: AtomicUsize,
   100	    prefetch_count: AtomicUsize,
   101	    /// 0 = unset (kernel default), matching the old `Option<usize>`.
   102	    tcp_buffer_bytes: AtomicUsize,
   103	    initial_streams: AtomicUsize,
   104	    max_streams: AtomicUsize,
   105	    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
   106	    // resize ids; 0 is reserved for the initial stream set) ──────────
   107	    /// Settled live stream count. Epoch-0 write is
   108	    /// `set_negotiated_streams`; later writes come from
   109	    /// `resize_settled` on an accepted epoch.
   110	    live_streams: AtomicUsize,
   111	    /// Last settled epoch (0 until the first accepted resize).
   112	    resize_epoch: AtomicU32,
   113	    /// In-flight proposal's epoch; 0 = none. While non-zero no new
   114	    /// proposal is produced (the wire is idempotent but overlapping
   115	    /// epochs would complicate sub-token registration).
   116	    pending_epoch: AtomicU32,
   117	    /// Resize-eligible ticks since the last settle (cooldown clock).
   118	    ticks_since_settle: AtomicU32,
   119	    /// Consecutive same-direction tick counter: positive = "pipe clean
   120	    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
   121	    /// dials floored" streak. Any other tick resets it.
   122	    resize_sustain: AtomicI32,
   123	    // Profile-clamped bounds, fixed at construction.
   124	    ceiling_chunk_bytes: usize,
   125	    ceiling_prefetch: usize,
   126	    ceiling_max_streams: usize,
   127	    ceiling_tcp_buffer_bytes: usize,
   128	}
   129	
   130	/// One engine resize decision (`ue-r2-2`). The adapter that owns the
   131	/// control stream turns this into a wire `DataPlaneResize` (the engine
   132	/// stays wire-type-free here on purpose) and MUST eventually call
   133	/// [`TransferDial::resize_settled`] for the epoch — with what actually
   134	/// happened — or no further proposals are produced.
   135	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
   136	pub struct ResizeProposal {
   137	    /// The wire epoch for this change (`resize_epoch() + 1`).
   138	    pub epoch: u32,
   139	    /// Absolute desired live count (idempotent, per the proto).
   140	    pub target_streams: usize,
   141	    /// Convenience: `target_streams > live` at proposal time.
   142	    pub add: bool,
   143	}
   144	
   145	impl TransferDial {
   146	    /// Conservative start with default ceilings (no receiver profile).
   147	    pub fn conservative() -> Self {
   148	        Self::conservative_within(None)
   149	    }
   150	
   151	    /// Conservative start bounded by the receiver's advertised
   152	    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
   153	    /// fields mean UNKNOWN and keep the (already conservative)
   154	    /// default ceiling — never "unlimited". A profile can only lower
   155	    /// ceilings, never raise them above the defaults this slice.
   156	    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
   157	        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
   158	        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
   159	        let ceiling_streams = receiver_stream_ceiling(profile);
   160	        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
   161	        if let Some(profile) = profile {
   162	            if profile.max_chunk_bytes > 0 {
   163	                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
   164	            }
   165	            if profile.max_inflight_bytes > 0 {
   166	                // The in-flight budget bounds the CHUNK ceiling first
   167	                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
   168	                // smaller than one chunk must still be honored — floor
   169	                // 64 KiB, matching the session's minimum buffer), then
   170	                // prefetch so prefetch × chunk stays within budget
   171	                // (floor of 1 so work still moves).
   172	                let inflight = profile.max_inflight_bytes as usize;
   173	                ceiling_chunk =
   174	                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
   175	                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
   176	                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
   177	            }
   178	        }
   179	        Self {
   180	            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
   181	            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
   182	            tcp_buffer_bytes: AtomicUsize::new(0),
   183	            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   184	            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
   185	            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   186	            resize_epoch: AtomicU32::new(0),
   187	            pending_epoch: AtomicU32::new(0),
   188	            ticks_since_settle: AtomicU32::new(0),
   189	            resize_sustain: AtomicI32::new(0),
   190	            ceiling_chunk_bytes: ceiling_chunk,
   191	            ceiling_prefetch,
   192	            ceiling_max_streams: ceiling_streams,
   193	            ceiling_tcp_buffer_bytes: ceiling_tcp,
   194	        }
   195	    }
   196	
   197	    pub fn shared(self) -> Arc<Self> {
   198	        Arc::new(self)
   199	    }
   200	
   201	    // ── live reads ───────────────────────────────────────────────────
   202	    pub fn chunk_bytes(&self) -> usize {
   203	        self.chunk_bytes.load(Ordering::Relaxed)
   204	    }
   205	    pub fn prefetch_count(&self) -> usize {
   206	        self.prefetch_count.load(Ordering::Relaxed)
   207	    }
   208	    /// `None` = leave the kernel default (old `tcp_buffer_size`
   209	    /// semantics). Connect-time dial.
   210	    pub fn tcp_buffer_bytes(&self) -> Option<usize> {
   211	        match self.tcp_buffer_bytes.load(Ordering::Relaxed) {
   212	            0 => None,
   213	            n => Some(n),
   214	        }
   215	    }
   216	    pub fn initial_streams(&self) -> usize {
   217	        self.initial_streams.load(Ordering::Relaxed)
   218	    }
   219	    /// Ceiling on the negotiated stream count (profile-clamped).
   220	    pub fn max_streams(&self) -> usize {
   221	        self.max_streams.load(Ordering::Relaxed)
   222	    }
   223	    pub fn ceiling_max_streams(&self) -> usize {
   224	        self.ceiling_max_streams
   225	    }
   226	
   227	    /// Record the stream count the negotiation actually settled on
   228	    /// (clamped to the dial's ceiling). This is the epoch-0 settle:
   229	    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
   230	    /// resize proposal steps from.
   231	    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
   232	        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
   233	        self.initial_streams.store(clamped, Ordering::Relaxed);
   234	        self.live_streams.store(clamped, Ordering::Relaxed);
   235	        clamped
   236	    }
   237	
   238	    // ── ue-r2-2 resize policy ────────────────────────────────────────
   239	
   240	    /// The settled live stream count (epoch-0 negotiation, then each
   241	    /// accepted resize).
   242	    pub fn live_streams(&self) -> usize {
   243	        self.live_streams.load(Ordering::Relaxed)
   244	    }
   245	
   246	    /// Last settled resize epoch (0 = only the initial stream set).
   247	    pub fn resize_epoch(&self) -> u32 {
   248	        self.resize_epoch.load(Ordering::Relaxed)
   249	    }
   250	
   251	    /// True while a proposal is awaiting `resize_settled`.
   252	    pub fn resize_pending(&self) -> bool {
   253	        self.pending_epoch.load(Ordering::Relaxed) != 0
   254	    }
   255	
   256	    fn cheap_dials_maxed(&self) -> bool {
   257	        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
   258	            && self.prefetch_count.load(Ordering::Relaxed) >= self.ceiling_prefetch
   259	    }
   260	
   261	    fn cheap_dials_floored(&self) -> bool {
   262	        self.chunk_bytes.load(Ordering::Relaxed)
   263	            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
   264	            && self.prefetch_count.load(Ordering::Relaxed)
   265	                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
   266	    }
   267	
   268	    /// One resize-eligible tuner tick. Streams move only as the LAST
   269	    /// escalation step in either direction: the cheap dials must
   270	    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
   271	    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
   272	    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
   273	    /// since the last settle, and no proposal may be in flight. Idle
   274	    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
   275	    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
   276	    /// folded in at construction — `CapacityProfile.max_streams` is
   277	    /// authoritative per the proto). One stream per epoch.
   278	    ///
   279	    /// The caller must forward the returned proposal to the peer and
   280	    /// call [`Self::resize_settled`] with the outcome; until then
   281	    /// every subsequent tick returns `None`.
   282	    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
   283	        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
   284	            return None;
   285	        }
   286	        let ticks = self
   287	            .ticks_since_settle
   288	            .fetch_add(1, Ordering::Relaxed)
   289	            .saturating_add(1);
   290	        if delta_bytes == 0 {
   291	            self.resize_sustain.store(0, Ordering::Relaxed);
   292	            return None;
   293	        }
   294	        let live = self.live_streams.load(Ordering::Relaxed).max(1);
   295	        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
   296	            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
   297	            let next = prev.saturating_add(1);
   298	            self.resize_sustain.store(next, Ordering::Relaxed);
   299	            next
   300	        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO && self.cheap_dials_floored() {

 succeeded in 0ms:
  2500	            ),
  2501	        )
  2502	        .await),
  2503	    }
  2504	}
  2505	
  2506	fn violation(message: String) -> eyre::Report {
  2507	    eyre::Report::new(SessionFault::protocol_violation(message))
  2508	}
  2509	
  2510	/// A protocol violation that names the file it concerns (otp-7b-2):
  2511	/// the path rides `SessionFault.relative_path` so the end-of-operation
  2512	/// summary can name it structurally.
  2513	fn violation_for(path: &str, message: String) -> eyre::Report {
  2514	    eyre::Report::new(SessionFault::protocol_violation(message).with_path(path))
  2515	}
  2516	
  2517	/// Attach `path` to a non-fault error (otp-7b-2). A report already
  2518	/// carrying a `SessionFault` is left untouched — the fault owns its
  2519	/// own identity, and wrapping it would bury the downcast
  2520	/// `fault_from_report` depends on.
  2521	fn tag_path(report: eyre::Report, path: &str) -> eyre::Report {
  2522	    if report.downcast_ref::<SessionFault>().is_some() {
  2523	        report
  2524	    } else {
  2525	        report.wrap_err(FaultedPath(path.to_string()))
  2526	    }
  2527	}
  2528	
  2529	/// otp-6b: the DESTINATION's mirror delete pass — the session's single
  2530	/// delete rule. Plans (enumerate dest + diff against the complete source
  2531	/// file set) and executes the extraneous deletions, all blocking FS work,
  2532	/// so it runs on the blocking pool. Returns `(files, dirs)` deleted —
  2533	/// split so the local carrier's summary (otp-11) can report both; wire
  2534	/// summaries carry the sum. `execute: false` (local `--dry-run` only)
  2535	/// plans and counts without touching the filesystem.
  2536	///
  2537	/// Every target is containment-checked against the canonical destination
  2538	/// root before any filesystem op (the same chokepoint the sink write paths
  2539	/// use). Missing entries are tolerated — the pass is idempotent. Deletion
  2540	/// order is files then dirs deepest-first (the plan sorts them). `remove_dir`
  2541	/// (not `remove_dir_all`) is used so out-of-scope content is never removed:
  2542	/// under `FilteredSubset` an extraneous dir that still holds filter-excluded
  2543	/// files fails with ENOTEMPTY and is left alone; under `All` the tree was
  2544	/// enumerated unfiltered, so a dir reaching here is empty and a non-empty one
  2545	/// is a genuine error.
  2546	fn mirror_delete_pass(
  2547	    dst_root: &Path,
  2548	    source_files: &HashSet<String>,
  2549	    filter: &crate::fs_enum::FileFilter,
  2550	    tolerate_nonempty_dirs: bool,
  2551	    canonical_dst_root: Option<&Path>,
  2552	    abort: &AtomicBool,
  2553	    execute: bool,
  2554	) -> Result<(u64, u64)> {
  2555	    let plan = crate::mirror_planner::MirrorPlanner::new(false).plan_session_deletions(
  2556	        dst_root,
  2557	        source_files,
  2558	        filter,
  2559	    )?;
  2560	
  2561	    let contained = |target: &Path| -> Result<()> {
  2562	        if let Some(root) = canonical_dst_root {
  2563	            crate::path_safety::verify_contained(root, target).map_err(|e| {
  2564	                eyre::eyre!("mirror delete containment {}: {e:#}", target.display())
  2565	            })?;
  2566	        }
  2567	        Ok(())
  2568	    };
  2569	
  2570	    // codex otp-9b F2: a dropped session future (client disconnect,
  2571	    // CancelJob) cannot abort a running blocking task — the caller's
  2572	    // drop-guard flips this flag instead, and the pass stops deleting
  2573	    // at the next filesystem op rather than running to completion
  2574	    // behind a job already recorded cancelled.
  2575	    let check_abort = || -> Result<()> {
  2576	        if abort.load(Ordering::Acquire) {
  2577	            return Err(eyre::eyre!("mirror delete pass aborted: session cancelled"));
  2578	        }
  2579	        Ok(())
  2580	    };
  2581	
  2582	    let mut deleted_files = 0u64;
  2583	    let mut deleted_dirs = 0u64;
  2584	    for file in &plan.files {
  2585	        check_abort()?;
  2586	        contained(file)?;
  2587	        if !execute {
  2588	            deleted_files += 1;
  2589	            continue;
  2590	        }
  2591	        // Windows refuses to delete a read-only file; clear the attribute
  2592	        // first, matching the daemon purge (admin.rs) and local mirror
  2593	        // (engine/mirror.rs) executors (codex otp-6b F2).
  2594	        #[cfg(windows)]
  2595	        crate::win_fs::clear_readonly_recursive(file);
  2596	        match std::fs::remove_file(file) {
  2597	            Ok(()) => deleted_files += 1,
  2598	            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
  2599	            Err(e) => return Err(eyre::eyre!("mirror delete {}: {e}", file.display())),
  2600	        }
  2601	    }
  2602	    for dir in &plan.dirs {
  2603	        check_abort()?;
  2604	        contained(dir)?;
  2605	        if !execute {
  2606	            deleted_dirs += 1;
  2607	            continue;
  2608	        }
  2609	        #[cfg(windows)]
  2610	        crate::win_fs::clear_readonly_recursive(dir);
  2611	        match std::fs::remove_dir(dir) {
  2612	            Ok(()) => deleted_dirs += 1,
  2613	            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
  2614	            // FilteredSubset: the dir still holds out-of-scope files the
  2615	            // filter excluded from enumeration; leaving it is the scope
  2616	            // contract, not a failure (engine/mirror.rs R58-F6). `Some(66)`
  2617	            // is ENOTEMPTY on macOS/BSD, which maps to a different ErrorKind.
  2618	            Err(e)
  2619	                if tolerate_nonempty_dirs
  2620	                    && (e.kind() == std::io::ErrorKind::DirectoryNotEmpty
  2621	                        || e.raw_os_error() == Some(66)) => {}
  2622	            Err(e) => return Err(eyre::eyre!("mirror delete dir {}: {e}", dir.display())),
  2623	        }
  2624	    }
  2625	    Ok((deleted_files, deleted_dirs))
  2626	}
  2627	
  2628	async fn destination_session(
  2629	    transport: &mut FrameTransport,
  2630	    negotiated: Negotiated,
  2631	    dst_root: &Path,
  2632	    data_plane_host: Option<&str>,
  2633	    instruments: DestinationInstruments,
  2634	    local_apply: Option<local::LocalApply>,
  2635	) -> Result<DestinationOutcome> {
  2636	    // otp-10b-2: the receive side's w6-1 progress lane. Need batches are
  2637	    // the denominator (reported where they're emitted, below); per-file
  2638	    // events ride each carrier's record handling.
  2639	    let progress = instruments.progress;
  2640	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  2641	        .unwrap_or(ComparisonMode::Unspecified);
  2642	    // Session deletions run via the otp-6b mirror pass (a whole-tree
  2643	    // diff at SourceDone), never a per-entry flag.
  2644	    let compare_opts = CompareOptions {
  2645	        mode: compare_mode.into(),
  2646	        ignore_existing: negotiated.open.ignore_existing,
  2647	    };
  2648	    // src_root is only consumed by local File payloads, which never
  2649	    // occur on a WIRE session destination (payload bytes arrive as
  2650	    // records and go through the stream/tar write paths); the LOCAL
  2651	    // carrier (otp-11) brings its own fully-configured sink, where
  2652	    // File payloads are the point. `Arc` so the data-plane receive
  2653	    // task (otp-4b) can share the one sink across sockets.
  2654	    let sink: Arc<dyn TransferSink> = match &local_apply {
  2655	        Some(la) => Arc::clone(&la.sink),
  2656	        None => {
  2657	            let mut sink = FsTransferSink::new(
  2658	                PathBuf::new(),
  2659	                dst_root.to_path_buf(),
  2660	                FsSinkConfig {
  2661	                    preserve_times: true,
  2662	                    dry_run: false,
  2663	                    checksum: None,
  2664	                    resume: false,
  2665	                    compare_mode,
  2666	                },
  2667	            );
  2668	            // otp-9a: applied payload bytes report against the caller's live
  2669	            // counter (the delegated dst daemon's jobs row) through the sink's
  2670	            // existing ByteProgressSink contract.
  2671	            if let Some(bp) = instruments.byte_progress {
  2672	                sink = sink.with_byte_progress(bp);
  2673	            }
  2674	            Arc::new(sink)
  2675	        }
  2676	    };
  2677	    // Same canonical-containment chokepoint the sink write paths use
  2678	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  2679	    // make the destination stat outside its root.
  2680	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  2681	
  2682	    // otp-6b: mirror config. The DESTINATION owns the delete pass (it holds
  2683	    // the tree). `mirror_filter` scopes the dest enumeration — the user
  2684	    // filter for FilteredSubset (out-of-scope dest entries are never
  2685	    // candidates), the whole-tree default for All. Globs were validated at
  2686	    // OPEN. `source_files` accumulates the COMPLETE source file set (only
  2687	    // when mirroring) so the pass can diff it against the dest at SourceDone.
  2688	    let mirror_enabled = negotiated.open.mirror_enabled;
  2689	    let mirror_kind = MirrorMode::try_from(negotiated.open.mirror_kind).unwrap_or(MirrorMode::Off);
  2690	    let mirror_filter: crate::fs_enum::FileFilter =
  2691	        if mirror_enabled && mirror_kind == MirrorMode::FilteredSubset {
  2692	            // otp-11: the local carrier threads the user's FileFilter
  2693	            // directly (process-local; no wire FilterSpec round-trip) —
  2694	            // same type, same delete pass, same scope semantics.
  2695	            if let Some(la) = &local_apply {
  2696	                la.mirror_scope_filter.clone_without_cache()
  2697	            } else {
  2698	                match negotiated.open.filter.as_ref() {
  2699	                    Some(spec) if *spec != FilterSpec::default() => {
  2700	                        crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  2701	                            .map_err(|e| {
  2702	                                eyre::Report::new(SessionFault::internal(format!(
  2703	                                    "invalid filter: {e:#}"
  2704	                                )))
  2705	                            })?
  2706	                    }
  2707	                    _ => crate::fs_enum::FileFilter::default(),
  2708	                }
  2709	            }
  2710	        } else {
  2711	            crate::fs_enum::FileFilter::default()
  2712	        };
  2713	    let mut source_files: HashSet<String> = HashSet::new();
  2714	
  2715	    // otp-7a: resume. Headers of resume-granted needs are retained so a
  2716	    // record's completion can finalize with the manifest's
  2717	    // size/mtime/permissions and be validated against the grant. Both
  2718	    // the header map and the resumed counter are SHARED with the
  2719	    // data-plane receive (otp-7b) exactly as `outstanding` is: on the
  2720	    // data plane the control loop never sees block records, so the
  2721	    // NeedListSink claims resume grants and counts completions as they
  2722	    // land on the sockets. The block size is chosen below, once the
  2723	    // carrier is known (the ceiling is per carrier).
  2724	    let resume_enabled = resume_negotiated(&negotiated.open);
  2725	    let resume_headers: data_plane::ResumeHeaders = Arc::default();
  2726	    let files_resumed = Arc::new(std::sync::atomic::AtomicU64::new(0));
  2727	
  2728	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  2729	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  2730	    // insert-only, never removed, so a concurrent data-plane claim can
  2731	    // never re-open a grant (a duplicate manifest path is granted at
  2732	    // most once regardless of delivery timing). `outstanding` is the
  2733	    // not-yet-delivered COMPLETION set — inserted for each freshly
  2734	    // granted path before its NeedBatch, claimed by both carriers (the
  2735	    // in-stream arms inline, the data-plane NeedListSink as payloads
  2736	    // land), and empty at SourceDone. A count proxy was insufficient
  2737	    // (F1); merging the two into one set raced the data-plane claim
  2738	    // against the diff (fix-review F1).
  2739	    let mut granted: HashSet<String> = HashSet::new();
  2740	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  2741	
  2742	    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
  2743	    // bytes arrive on sockets (not the control lane). Set it up NOW —
  2744	    // concurrent with the diff loop below, and before the peer sends — so
  2745	    // the connections are established promptly. Which end connects depends
  2746	    // on connection role (otp-5b): a DESTINATION **responder** (push)
  2747	    // accepts sockets off its bound listener; a DESTINATION **initiator**
  2748	    // (pull) dials the grant it received on `data_plane_host`. Byte
  2749	    // direction is the same either way (DESTINATION receives). The
  2750	    // NeedListSink gives the socket receive the same need-list strictness
  2751	    // the in-stream control loop applies inline; AbortOnDrop (inside the
  2752	    // responder run) bounds the accept task to this future. `resize_live`
  2753	    // tracks the stream count this end has grown to (epoch-0 plus each
  2754	    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
  2755	    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
  2756	    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
  2757	    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  2758	        Arc::clone(&sink),
  2759	        Arc::clone(&outstanding),
  2760	        // otp-7b: only a resume session accepts block records on the
  2761	        // data plane; the sink validates + claims them against the same
  2762	        // shared grant state the in-stream arms use.
  2763	        resume_enabled.then(|| data_plane::ResumeRecv {
  2764	            headers: Arc::clone(&resume_headers),
  2765	            resumed: Arc::clone(&files_resumed),
  2766	        }),
  2767	    ));
  2768	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  2769	        match negotiated.responder_data_plane {
  2770	            // DESTINATION responder (push, otp-4b): accept + receive.
  2771	            Some(rdp) => {
  2772	                let initial = rdp.initial_streams() as usize;
  2773	                let run = rdp.spawn(recv_sink, progress.clone());
  2774	                let ceiling = run.ceiling;
  2775	                (
  2776	                    Some(data_plane::DestRecvPlane::Responder(run)),
  2777	                    initial,
  2778	                    ceiling,
  2779	                )
  2780	            }
  2781	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2782	            // SOURCE responder granted a data plane and we have a host to dial.
  2783	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2784	                (Some(grant), Some(host)) => {
  2785	                    let initial = grant.initial_streams.max(1) as usize;
  2786	                    let run = data_plane::dial_destination_data_plane(
  2787	                        host,
  2788	                        grant,
  2789	                        recv_sink,
  2790	                        progress.clone(),
  2791	                        instruments.trace_data_plane,
  2792	                    )
  2793	                    .await?;
  2794	                    // otp-5b-2: the pull data plane resizes too. Seed
  2795	                    // `resize_live` from the epoch-0 streams dialed and bound
  2796	                    // growth by the capacity THIS end advertised in its open
  2797	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2798	                    // responder's dial already clamps to, so both ends agree
  2799	                    // even when the caller advertised a max_streams below this
  2800	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2801	                    // Resize frame the initiator dials the epoch-N socket (vs
  2802	                    // the responder path's arm).
  2803	                    let ceiling = crate::dial::receiver_stream_ceiling(
  2804	                        negotiated.open.receiver_capacity.as_ref(),
  2805	                    );
  2806	                    (
  2807	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2808	                        initial,
  2809	                        ceiling,
  2810	                    )
  2811	                }
  2812	                // A grant with no host to dial is an inconsistent initiator
  2813	                // config: fail fast, mirroring the SOURCE initiator
  2814	                // (`source_send_half`). The SOURCE responder has already bound
  2815	                // and blocks accepting the socket this end would dial, so
  2816	                // silently taking the in-stream branch cannot fall back — it
  2817	                // would deadlock until the responder's accept times out. A
  2818	                // grant means the initiator MUST dial (contract §Transport).
  2819	                // (codex otp-5b-1 finding.)
  2820	                (Some(_), None) => {
  2821	                    return Err(eyre::Report::new(SessionFault::internal(
  2822	                        "responder granted a TCP data plane but this DESTINATION \
  2823	                     initiator has no host to dial",
  2824	                    )))
  2825	                }
  2826	                // No grant (the responder could not bind, or the initiator
  2827	                // asked for in-stream): the in-stream carrier.
  2828	                (None, _) => (None, 0usize, 0usize),
  2829	            },
  2830	        };
  2831	
  2832	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  2833	    // — it hashes first; the SOURCE reads the size from each
  2834	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  2835	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
  2836	    // here, after the carrier is settled.
  2837	    let resume_block_size = {
  2838	        let ceiling = if data_plane_recv.is_some() {
  2839	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2840	        } else {
  2841	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2842	        };
  2843	        match negotiated
  2844	            .open
  2845	            .resume
  2846	            .as_ref()
  2847	            .map(|r| r.block_size as usize)
  2848	            .unwrap_or(0)
  2849	        {
  2850	            0 => DEFAULT_BLOCK_SIZE,
  2851	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2852	        }
  2853	    };
  2854	
  2855	    let mut pending: Vec<FileHeader> = Vec::new();
  2856	    let mut needed_paths: Vec<String> = Vec::new();
  2857	    let mut manifest_complete = false;
  2858	    let mut files_written: u64 = 0;
  2859	    let mut bytes_written: u64 = 0;
  2860	
  2861	    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
  2862	    // loop so applies run concurrent with the diff, exactly as the
  2863	    // data-plane receive does.
  2864	    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));
  2865	
  2866	    loop {
  2867	        let received = match transport.recv().await? {
  2868	            Some(f) => f,
  2869	            None => {
  2870	                return Err(eyre::Report::new(SessionFault::internal(
  2871	                    "peer closed mid-session",
  2872	                )))
  2873	            }
  2874	        };
  2875	        match received.frame {
  2876	            Some(Frame::ManifestEntry(header)) => {
  2877	                if manifest_complete {
  2878	                    return Err(violation(format!(
  2879	                        "manifest entry '{}' after ManifestComplete",
  2880	                        header.relative_path
  2881	                    )));
  2882	                }
  2883	                // otp-6b: retain the full source path set for the mirror
  2884	                // diff (the need list keeps only files needing transfer).
  2885	                if mirror_enabled {
  2886	                    source_files.insert(header.relative_path.clone());
  2887	                }
  2888	                pending.push(header);
  2889	                if pending.len() >= DEST_DIFF_CHUNK {
  2890	                    let chunk = std::mem::take(&mut pending);
  2891	                    if let Some(la) = &local_apply {
  2892	                        diff_chunk_and_apply_local(
  2893	                            la,
  2894	                            &mut local_run,
  2895	                            chunk,
  2896	                            dst_root,
  2897	                            canonical_dst_root.as_deref(),
  2898	                            &compare_opts,
  2899	                            &mut granted,
  2900	                            &mut needed_paths,
  2901	                            progress.as_ref(),
  2902	                        )
  2903	                        .await?;
  2904	                    } else {
  2905	                        diff_chunk_and_send_needs(
  2906	                            transport,
  2907	                            chunk,
  2908	                            dst_root,
  2909	                            canonical_dst_root.as_deref(),
  2910	                            &compare_opts,
  2911	                            resume_enabled,
  2912	                            resume_block_size,
  2913	                            &resume_headers,
  2914	                            &mut granted,
  2915	                            &outstanding,
  2916	                            &mut needed_paths,
  2917	                            progress.as_ref(),
  2918	                        )
  2919	                        .await?;
  2920	                    }
  2921	                }
  2922	            }
  2923	            Some(Frame::ManifestComplete(complete)) => {
  2924	                if manifest_complete {
  2925	                    return Err(violation("duplicate ManifestComplete".into()));

 succeeded in 0ms:
  1220	/// HELD until its `BlockHashList` arrives (the contract's strict
  1221	/// ordering — the source must not send a byte of that file first); the
  1222	/// correlated pair then queues for the block phase.
  1223	#[derive(Default)]
  1224	struct ResumeSendState {
  1225	    held: HashMap<String, FileHeader>,
  1226	    ready: Vec<(FileHeader, BlockHashList)>,
  1227	}
  1228	
  1229	#[allow(clippy::too_many_arguments)]
  1230	async fn source_send_half(
  1231	    plan_options: PlanOptions,
  1232	    data_plane_host: Option<&str>,
  1233	    instruments: SourceInstruments,
  1234	    negotiated: &Negotiated,
  1235	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
  1236	    tx: &mut Box<dyn FrameTx>,
  1237	    source: Arc<dyn TransferSource>,
  1238	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1239	    manifest_sent: &AtomicBool,
  1240	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
  1241	    mut fault_signal: watch::Receiver<Option<SessionFault>>,
  1242	) -> Result<TransferSummary> {
  1243	    let mut pending: Vec<FileHeader> = Vec::new();
  1244	    let mut resume: ResumeSendState = ResumeSendState::default();
  1245	    let mut need_complete = false;
  1246	
  1247	    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
  1248	    // streaming the manifest — so the peer sees the connections promptly
  1249	    // rather than waiting out a bounded-accept/connect timeout while a long
  1250	    // manifest streams. Which end connects depends on connection role
  1251	    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
  1252	    // listener; a SOURCE **initiator** (push) dials the grant it received.
  1253	    // Byte direction is the same either way (SOURCE sends), so both yield a
  1254	    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
  1255	    // in-stream carrier (fallback), which needs no early setup.
  1256	    let mut data_plane = match responder_data_plane {
  1257	        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
  1258	        // initiator advertised its capacity in the open (byte RECEIVER
  1259	        // advertises, wherever it initiates); the accept plane is single-
  1260	        // stream (otp-5b-1).
  1261	        Some(bound) => Some(
  1262	            data_plane::accept_source_data_plane(
  1263	                bound,
  1264	                negotiated.open.receiver_capacity.as_ref(),
  1265	                Arc::clone(&source),
  1266	                &instruments,
  1267	            )
  1268	            .await?,
  1269	        ),
  1270	        // SOURCE initiator (push, otp-4b): dial the grant if the responder
  1271	        // granted a data plane; else in-stream.
  1272	        None => match &negotiated.accept.data_plane {
  1273	            Some(grant) => {
  1274	                let host = data_plane_host.ok_or_else(|| {
  1275	                    eyre::Report::new(SessionFault::internal(
  1276	                        "responder granted a TCP data plane but this initiator has no host to dial",
  1277	                    ))
  1278	                })?;
  1279	                Some(
  1280	                    data_plane::dial_source_data_plane(
  1281	                        host,
  1282	                        grant,
  1283	                        negotiated.accept.receiver_capacity.as_ref(),
  1284	                        Arc::clone(&source),
  1285	                        &instruments,
  1286	                    )
  1287	                    .await?,
  1288	                )
  1289	            }
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch. Settle the whole shape-derived target before
  1417	                    // handing payloads to the pipeline: otherwise the
  1418	                    // one-ADD-per-epoch ramp races NeedComplete/payload
  1419	                    // drain, so a fast transfer can finish at a different
  1420	                    // worker count depending on which endpoint initiated.
  1421	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1422	                        .await?;
  1423	                    settle_shape_resizes(
  1424	                        &mut events,
  1425	                        &mut pending,
  1426	                        &mut resume,
  1427	                        &mut need_complete,
  1428	                        &mut needed_bytes,
  1429	                        &mut needed_count,
  1430	                        dp,
  1431	                        tx,
  1432	                        &mut pending_resize,
  1433	                    )
  1434	                    .await?;
  1435	                    let payloads =
  1436	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1437	                    // A cancel while earlier batches are actively moving
  1438	                    // closes the send pipeline under backpressure, so this
  1439	                    // queue fails with a data-plane error — prefer the
  1440	                    // peer's framed reason (CANCELLED) the same way the
  1441	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1442	                    // against events like finish(): live `Need`s still
  1443	                    // arrive here, and `recv_peer_fault` would consume them.
  1444	                    if let Err(dp_err) = dp.queue(payloads).await {
  1445	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1446	                    }
  1447	                }
  1448	                None => {
  1449	                    // codex otp-8 F1: race the record sends against the
  1450	                    // receive half's fault signal — the in-stream twin of
  1451	                    // the data-plane drain's `recv_peer_fault` arm. A peer
  1452	                    // cancel (framed CANCELLED, then RPC teardown) must
  1453	                    // interrupt a send blocked in `reader.read()` or in
  1454	                    // flow-controlled `tx.send()` and surface the framed
  1455	                    // reason, not hang or decay to INTERNAL. Biased:
  1456	                    // when both are ready, the framed fault wins.
  1457	                    tokio::select! {
  1458	                        biased;
  1459	                        fault = peer_fault_signalled(&mut fault_signal) => {
  1460	                            return Err(eyre::Report::new(fault));
  1461	                        }
  1462	                        res = send_payload_records(
  1463	                            tx,
  1464	                            &source,
  1465	                            plan_options,
  1466	                            batch,
  1467	                            &mut read_buf,
  1468	                            instruments.progress.as_ref(),
  1469	                        ) => {
  1470	                            res?;
  1471	                        }
  1472	                    }
  1473	                }
  1474	            }
  1475	            continue;
  1476	        }
  1477	        if !resume.ready.is_empty() {
  1478	            // The block phase for correlated (need, hash-list) pairs.
  1479	            // Data plane (otp-7b): each pair becomes ONE composite
  1480	            // ResumeFile work item, so one pipeline worker runs the
  1481	            // whole record on one socket — strict per-file serialization
  1482	            // without cross-socket reorder hazards. In-stream (otp-7a):
  1483	            // control-lane BlockTransfer/Complete frames, as before.
  1484	            let ready = std::mem::take(&mut resume.ready);
  1485	            match &mut data_plane {
  1486	                Some(dp) => {
  1487	                    // codex 7b-1 F4: resume batches drive the sf-2 shape
  1488	                    // correction exactly as plain batches do — a
  1489	                    // resume-heavy need list must not stay pinned to the
  1490	                    // zero-knowledge single stream.
  1491	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1492	                        .await?;
  1493	                    settle_shape_resizes(
  1494	                        &mut events,
  1495	                        &mut pending,
  1496	                        &mut resume,
  1497	                        &mut need_complete,
  1498	                        &mut needed_bytes,
  1499	                        &mut needed_count,
  1500	                        dp,
  1501	                        tx,
  1502	                        &mut pending_resize,
  1503	                    )
  1504	                    .await?;
  1505	                    let payloads = ready
  1506	                        .into_iter()
  1507	                        .map(|(header, hashes)| TransferPayload::ResumeFile {
  1508	                            header,
  1509	                            block_size: hashes.block_size,
  1510	                            dest_hashes: hashes.hashes,
  1511	                        })
  1512	                        .collect();
  1513	                    // Same cancel posture as the plain-batch queue above:
  1514	                    // prefer the peer's framed reason over the transport
  1515	                    // break a cancel also causes (otp-4b-3 codex F1).
  1516	                    if let Err(dp_err) = dp.queue(payloads).await {
  1517	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1518	                    }
  1519	                }
  1520	                None => {
  1521	                    for (header, hashes) in ready {
  1522	                        // codex 7b-2 G2: the whole in-stream record names
  1523	                        // its file on failure, matching the data-plane
  1524	                        // carrier's outer wrap. Same fault race as the
  1525	                        // plain-batch send above (codex otp-8 F1).
  1526	                        tokio::select! {
  1527	                            biased;
  1528	                            fault = peer_fault_signalled(&mut fault_signal) => {
  1529	                                return Err(eyre::Report::new(fault));
  1530	                            }
  1531	                            res = send_resume_block_records(
  1532	                                tx,
  1533	                                &source,
  1534	                                &header,
  1535	                                &hashes,
  1536	                                instruments.progress.as_ref(),
  1537	                            ) => {
  1538	                                res.map_err(|e| tag_path(e, &header.relative_path))?;
  1539	                            }
  1540	                        }
  1541	                    }
  1542	                }
  1543	            }
  1544	            continue;
  1545	        }
  1546	        if need_complete {
  1547	            break;
  1548	        }
  1549	        match events.recv().await {
  1550	            Some(event) => {
  1551	                process_source_event(
  1552	                    event,
  1553	                    &mut pending,
  1554	                    &mut resume,
  1555	                    &mut need_complete,
  1556	                    &mut needed_bytes,
  1557	                    &mut needed_count,
  1558	                    data_plane.as_ref(),
  1559	                    tx,
  1560	                    &mut pending_resize,
  1561	                )
  1562	                .await?;
  1563	            }
  1564	            None => {
  1565	                return Err(eyre::Report::new(SessionFault::internal(
  1566	                    "source receive half ended before NeedComplete",
  1567	                )))
  1568	            }
  1569	        }
  1570	    }
  1571	
  1572	    // A resize proposed on the last batch may still be in flight. Resolve
  1573	    // it BEFORE finishing so the destination's armed slot is consumed by
  1574	    // the dialed socket — an armed-but-never-dialed credential would hang
  1575	    // its accept loop (which waits for every arm to be claimed). We do not
  1576	    // propose further here: exactly the one in-flight resize is drained.
  1577	    if let Some(dp) = &data_plane {
  1578	        if let Some(pending) = pending_resize.take() {
  1579	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1580	        }
  1581	    }
  1582	
  1583	    // Close the data plane BEFORE SourceDone so the destination's receive
  1584	    // pipeline sees each socket's END record and completes; SourceDone on
  1585	    // the control lane then lets the destination score and summarize.
  1586	    //
  1587	    // The drain is the byte-transfer phase's wall-time sink, so a
  1588	    // mid-transfer cancel almost always lands here. Race it against a
  1589	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
  1590	    // the served session frames `SessionError{CANCELLED}`, and the source
  1591	    // must surface THAT — not the data-plane transport break it also
  1592	    // causes. Two orderings, both covered:
  1593	    //   * fault arrives while the drain is still pending (e.g. a worker
  1594	    //     blocked reading a slow file, so the socket break never unblocks
  1595	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
  1596	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
  1597	    //     stops the in-flight workers.
  1598	    //   * the socket break makes `finish()` return `Err` first → prefer
  1599	    //     the framed reason if the control lane delivers one within the
  1600	    //     stall window (`prefer_peer_fault`).
  1601	    if let Some(dp) = data_plane.take() {
  1602	        tokio::select! {
  1603	            biased;
  1604	            fault = recv_peer_fault(&mut events) => {
  1605	                return Err(eyre::Report::new(fault));
  1606	            }
  1607	            res = dp.finish() => {
  1608	                if let Err(dp_err) = res {
  1609	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
  1610	                }
  1611	            }
  1612	        }
  1613	    }
  1614	
  1615	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
  1616	
  1617	    // CLOSING: the destination is the scorer; the next event must be
  1618	    // its summary (the receive half ends after forwarding it).
  1619	    match events.recv().await {
  1620	        Some(SourceEvent::Summary(summary)) => Ok(summary),
  1621	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
  1622	        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1623	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1624	                format!("need for '{}' after NeedComplete", h.relative_path),
  1625	            )))
  1626	        }
  1627	        Some(SourceEvent::BlockHashes(l)) => {
  1628	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1629	                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
  1630	            )))
  1631	        }
  1632	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
  1633	            SessionFault::protocol_violation("duplicate NeedComplete"),
  1634	        )),
  1635	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
  1636	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
  1637	        )),
  1638	        None => Err(eyre::Report::new(SessionFault::internal(
  1639	            "source receive half ended before TransferSummary",
  1640	        ))),
  1641	    }
  1642	}
  1643	
  1644	/// Process every event ready right now (needs accumulating, resize acks
  1645	/// dialing their epoch-N socket) without blocking. Called between
  1646	/// manifest sends and at the top of the payload loop.
  1647	#[allow(clippy::too_many_arguments)]
  1648	async fn drain_ready_source_events(
  1649	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1650	    pending: &mut Vec<FileHeader>,
  1651	    resume: &mut ResumeSendState,
  1652	    need_complete: &mut bool,
  1653	    needed_bytes: &mut u64,
  1654	    needed_count: &mut usize,
  1655	    data_plane: Option<&data_plane::SourceDataPlane>,
  1656	    tx: &mut Box<dyn FrameTx>,
  1657	    pending_resize: &mut Option<data_plane::PendingResize>,
  1658	) -> Result<()> {
  1659	    while let Ok(event) = events.try_recv() {
  1660	        process_source_event(
  1661	            event,
  1662	            pending,
  1663	            resume,
  1664	            need_complete,
  1665	            needed_bytes,
  1666	            needed_count,
  1667	            data_plane,
  1668	            tx,
  1669	            pending_resize,
  1670	        )
  1671	        .await?;
  1672	    }
  1673	    Ok(())
  1674	}
  1675	
  1676	/// Handle one source event. Needs accumulate into `pending` and the
  1677	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1678	/// next ADD (the one-per-epoch ramp).
  1679	#[allow(clippy::too_many_arguments)]
  1680	async fn process_source_event(
  1681	    event: SourceEvent,
  1682	    pending: &mut Vec<FileHeader>,
  1683	    resume: &mut ResumeSendState,
  1684	    need_complete: &mut bool,
  1685	    needed_bytes: &mut u64,
  1686	    needed_count: &mut usize,
  1687	    data_plane: Option<&data_plane::SourceDataPlane>,
  1688	    tx: &mut Box<dyn FrameTx>,
  1689	    pending_resize: &mut Option<data_plane::PendingResize>,
  1690	) -> Result<()> {
  1691	    match event {
  1692	        SourceEvent::Need(header) => {
  1693	            if *need_complete {
  1694	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1695	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1696	                )));
  1697	            }
  1698	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1699	            *needed_count += 1;
  1700	            pending.push(header);
  1701	            Ok(())
  1702	        }
  1703	        SourceEvent::ResumeNeed(header) => {
  1704	            if *need_complete {
  1705	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1706	                    format!(
  1707	                        "resume need for '{}' after NeedComplete",
  1708	                        header.relative_path
  1709	                    ),
  1710	                )));
  1711	            }
  1712	            // Shape totals count the whole file — the diff hasn't run
  1713	            // yet, so the need list's implied workload is the honest
  1714	            // upper bound (same accounting a plain need gets).
  1715	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1716	            *needed_count += 1;
  1717	            // HELD until its BlockHashList arrives; no duplicate is
  1718	            // possible (the receive half's sent-map removal already
  1719	            // faults a second need for the same path).
  1720	            resume.held.insert(header.relative_path.clone(), header);
  1721	            Ok(())
  1722	        }
  1723	        SourceEvent::BlockHashes(list) => {
  1724	            // Validate the wire block size at ARRIVAL (codex F5), not
  1725	            // when the record is eventually sent — pending plain files
  1726	            // go out first, and an already-invalid frame must fail fast.
  1727	            // A conforming destination clamps into this range (D5 /
  1728	            // D-2026-07-10-1); same-build peers make a mismatch a
  1729	            // violation, never a negotiation. The ceiling is the
  1730	            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
  1731	            // records take up to the wire block cap; in-stream frames
  1732	            // must stay under the gRPC frame limit.
  1733	            let ceiling = if data_plane.is_some() {
  1734	                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  1735	            } else {
  1736	                MAX_IN_STREAM_RESUME_BLOCK_SIZE
  1737	            };
  1738	            let bs = list.block_size as usize;
  1739	            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
  1740	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1741	                    format!(
  1742	                        "BlockHashList for '{}' block_size {bs} outside \
  1743	                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
  1744	                        list.relative_path
  1745	                    ),
  1746	                )));
  1747	            }
  1748	            match resume.held.remove(&list.relative_path) {
  1749	                Some(header) => {
  1750	                    resume.ready.push((header, list));
  1751	                    Ok(())
  1752	                }
  1753	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1754	                    format!(
  1755	                        "BlockHashList for '{}' without a held resume need",
  1756	                        list.relative_path
  1757	                    ),
  1758	                ))),
  1759	            }
  1760	        }
  1761	        SourceEvent::NeedComplete => {
  1762	            if *need_complete {
  1763	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1764	                    "duplicate NeedComplete",
  1765	                )));
  1766	            }
  1767	            // Ordered lane: the destination sends every BlockHashList
  1768	            // before its NeedComplete, so a still-held resume need here
  1769	            // means the peer broke the choreography — fail fast rather
  1770	            // than hang waiting for a list that can no longer arrive.
  1771	            if !resume.held.is_empty() {
  1772	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1773	                    format!(
  1774	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1775	                        resume.held.len()
  1776	                    ),
  1777	                )));
  1778	            }
  1779	            *need_complete = true;
  1780	            Ok(())
  1781	        }
  1782	        SourceEvent::ResizeAck(ack) => {
  1783	            let dp = data_plane.ok_or_else(|| {
  1784	                eyre::Report::new(SessionFault::protocol_violation(
  1785	                    "DataPlaneResizeAck on a session with no data plane",
  1786	                ))
  1787	            })?;
  1788	            // Match the ack to the in-flight proposal; stale/unsolicited
  1789	            // acks (wrong epoch, or none pending) are ignored, matching
  1790	            // old push. `take()` + restore keeps the borrow simple.
  1791	            let pending_r = match pending_resize.take() {
  1792	                Some(p) if p.epoch == ack.epoch => p,
  1793	                restored => {
  1794	                    *pending_resize = restored;
  1795	                    return Ok(());
  1796	                }
  1797	            };
  1798	            if ack.accepted {
  1799	                dp.add_stream(&pending_r.sub_token).await?;
  1800	                dp.dial()
  1801	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1802	                // Ramp one stream per accepted epoch: propose the next ADD.
  1803	                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1804	            } else {
  1805	                dp.dial()
  1806	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1807	                // A refusal is terminal for this shape ramp. Retrying the
  1808	                // same unattainable target under a fresh epoch would loop
  1809	                // forever; the settled live set still carries the transfer.
  1810	                Ok(())
  1811	            }
  1812	        }
  1813	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1814	            "TransferSummary before SourceDone",
  1815	        ))),
  1816	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1817	    }
  1818	}
  1819	
  1820	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1821	/// the stream count the accumulated need list implies, if none is in
  1822	/// flight. A no-op when the shape wants no more than the live count (the
  1823	/// dial returns `None`). Sends the frame and records the in-flight
  1824	/// proposal for the ack to match.
  1825	async fn maybe_propose_resize(
  1826	    dp: &data_plane::SourceDataPlane,
  1827	    tx: &mut Box<dyn FrameTx>,
  1828	    needed_bytes: u64,
  1829	    needed_count: usize,
  1830	    pending_resize: &mut Option<data_plane::PendingResize>,
  1831	) -> Result<()> {
  1832	    if pending_resize.is_some() {
  1833	        return Ok(());
  1834	    }
  1835	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1836	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1837	            op: DataPlaneResizeOp::Add as i32,
  1838	            epoch: proposal.epoch,
  1839	            target_stream_count: proposal.target_streams,
  1840	            sub_token: proposal.sub_token.clone(),
  1841	        })))
  1842	        .await?;
  1843	        *pending_resize = Some(proposal);
  1844	    }
  1845	    Ok(())
  1846	}
  1847	
  1848	/// Drive the one-stream-per-epoch shape ramp to its currently known target
  1849	/// before payload dispatch. Needs and resume hashes may continue arriving
  1850	/// while an ack is in flight, so process the shared SOURCE event lane rather
  1851	/// than waiting for only an ack. Each accepted ack proposes the next epoch
  1852	/// from the latest accumulated shape; the loop ends only when no proposal is
  1853	/// outstanding (target reached or the destination refused growth).
  1854	#[allow(clippy::too_many_arguments)]
  1855	async fn settle_shape_resizes(
  1856	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1857	    pending: &mut Vec<FileHeader>,
  1858	    resume: &mut ResumeSendState,
  1859	    need_complete: &mut bool,
  1860	    needed_bytes: &mut u64,
  1861	    needed_count: &mut usize,
  1862	    data_plane: &data_plane::SourceDataPlane,
  1863	    tx: &mut Box<dyn FrameTx>,
  1864	    pending_resize: &mut Option<data_plane::PendingResize>,
  1865	) -> Result<()> {
  1866	    while pending_resize.is_some() {
  1867	        let event = events.recv().await.ok_or_else(|| {
  1868	            eyre::Report::new(SessionFault::internal(
  1869	                "source receive half ended during data-plane shape resize",
  1870	            ))
  1871	        })?;
  1872	        process_source_event(
  1873	            event,
  1874	            pending,
  1875	            resume,
  1876	            need_complete,
  1877	            needed_bytes,
  1878	            needed_count,
  1879	            Some(data_plane),
  1880	            tx,
  1881	            pending_resize,
  1882	        )
  1883	        .await?;
  1884	    }
  1885	    Ok(())
  1886	}
  1887	
  1888	/// Block for the ack of the one in-flight resize and dial its socket (or
  1889	/// settle it refused). Does NOT propose further — it resolves exactly the
  1890	/// pending proposal so the destination's armed slot is consumed before we
  1891	/// finish the data plane.
  1892	async fn resolve_in_flight_resize(
  1893	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1894	    dp: &data_plane::SourceDataPlane,
  1895	    pending: data_plane::PendingResize,
  1896	) -> Result<()> {
  1897	    loop {
  1898	        match events.recv().await {
  1899	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1900	                if ack.accepted {
  1901	                    dp.add_stream(&pending.sub_token).await?;
  1902	                    dp.dial()
  1903	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1904	                } else {
  1905	                    dp.dial()
  1906	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1907	                }
  1908	                return Ok(());
  1909	            }
  1910	            // A stale ack for an already-settled epoch: ignore, keep
  1911	            // waiting for ours.
  1912	            Some(SourceEvent::ResizeAck(_)) => continue,
  1913	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1914	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1915	                return Err(eyre::Report::new(SessionFault::protocol_violation(

exec
/bin/zsh -lc "rg -n \"resize_(settled|propos|ack|request)|maybe_propose_resize|settle_shape_resizes|PendingResize|DataPlaneResize|ResizeAck\" crates/blit-core/src crates/blit-core/tests -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/tests/proto_wire_compat.rs:2://! (`CapacityProfile`, `DataPlaneResize`, `DataPlaneResizeAck`).
crates/blit-core/tests/proto_wire_compat.rs:16:    CapacityProfile, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, DrainClass,
crates/blit-core/tests/proto_wire_compat.rs:42:    let add = DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:43:        op: DataPlaneResizeOp::Add as i32,
crates/blit-core/tests/proto_wire_compat.rs:48:    let decoded = DataPlaneResize::decode(add.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/proto_wire_compat.rs:50:    assert_eq!(decoded.op(), DataPlaneResizeOp::Add);
crates/blit-core/tests/proto_wire_compat.rs:52:    let remove = DataPlaneResize {
crates/blit-core/tests/proto_wire_compat.rs:53:        op: DataPlaneResizeOp::Remove as i32,
crates/blit-core/tests/proto_wire_compat.rs:58:    let decoded = DataPlaneResize::decode(remove.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/proto_wire_compat.rs:61:    let ack = DataPlaneResizeAck {
crates/blit-core/tests/proto_wire_compat.rs:66:    let decoded = DataPlaneResizeAck::decode(ack.encode_to_vec().as_slice()).unwrap();
crates/blit-core/tests/transfer_session_roles.rs:1166:    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
crates/blit-core/tests/transfer_session_roles.rs:1316:    // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
crates/blit-core/tests/transfer_session_roles.rs:1317:    // `DataPlaneResizeAck` frames are identical to push; only the transport
crates/blit-core/src/transfer_session/mod.rs:41:    DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, FileData, FileHeader, FilterSpec,
crates/blit-core/src/transfer_session/mod.rs:463:        Some(Frame::Resize(_)) => "DataPlaneResize",
crates/blit-core/src/transfer_session/mod.rs:464:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
crates/blit-core/src/transfer_session/mod.rs:929:    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
crates/blit-core/src/transfer_session/mod.rs:931:    ResizeAck(DataPlaneResizeAck),
crates/blit-core/src/transfer_session/mod.rs:1195:            Some(Frame::ResizeAck(ack)) => {
crates/blit-core/src/transfer_session/mod.rs:1199:                let _ = events.send(SourceEvent::ResizeAck(ack));
crates/blit-core/src/transfer_session/mod.rs:1301:    let mut pending_resize: Option<data_plane::PendingResize> = None;
crates/blit-core/src/transfer_session/mod.rs:1421:                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
crates/blit-core/src/transfer_session/mod.rs:1423:                    settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs:1491:                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
crates/blit-core/src/transfer_session/mod.rs:1493:                    settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs:1635:        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1636:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
crates/blit-core/src/transfer_session/mod.rs:1657:    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs:1689:    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs:1782:        SourceEvent::ResizeAck(ack) => {
crates/blit-core/src/transfer_session/mod.rs:1785:                    "DataPlaneResizeAck on a session with no data plane",
crates/blit-core/src/transfer_session/mod.rs:1801:                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
crates/blit-core/src/transfer_session/mod.rs:1803:                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
crates/blit-core/src/transfer_session/mod.rs:1806:                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
crates/blit-core/src/transfer_session/mod.rs:1820:/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
crates/blit-core/src/transfer_session/mod.rs:1825:async fn maybe_propose_resize(
crates/blit-core/src/transfer_session/mod.rs:1830:    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs:1836:        tx.send(frame(Frame::Resize(DataPlaneResize {
crates/blit-core/src/transfer_session/mod.rs:1837:            op: DataPlaneResizeOp::Add as i32,
crates/blit-core/src/transfer_session/mod.rs:1855:async fn settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs:1864:    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs:1895:    pending: data_plane::PendingResize,
crates/blit-core/src/transfer_session/mod.rs:1899:            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
crates/blit-core/src/transfer_session/mod.rs:1903:                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
crates/blit-core/src/transfer_session/mod.rs:1906:                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
crates/blit-core/src/transfer_session/mod.rs:1912:            Some(SourceEvent::ResizeAck(_)) => continue,
crates/blit-core/src/transfer_session/mod.rs:1953:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
crates/blit-core/src/transfer_session/mod.rs:1976:        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1977:            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
crates/blit-core/src/transfer_session/mod.rs:1996:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
crates/blit-core/src/transfer_session/mod.rs:3159:                        "DataPlaneResize on a session with no data plane".into(),
crates/blit-core/src/transfer_session/mod.rs:3162:                let op = DataPlaneResizeOp::try_from(resize.op)
crates/blit-core/src/transfer_session/mod.rs:3163:                    .unwrap_or(DataPlaneResizeOp::Unspecified);
crates/blit-core/src/transfer_session/mod.rs:3164:                if op != DataPlaneResizeOp::Add {
crates/blit-core/src/transfer_session/mod.rs:3172:                        "DataPlaneResize sub_token must be 16 bytes".into(),
crates/blit-core/src/transfer_session/mod.rs:3208:                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
crates/blit-core/src/transfer_session/mod.rs:3407:                // destination-lane frames echoed back (a ResizeAck or
crates/blit-core/src/dial.rs:20://!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
crates/blit-core/src/dial.rs:109:    /// `resize_settled` on an accepted epoch.
crates/blit-core/src/dial.rs:131:/// control stream turns this into a wire `DataPlaneResize` (the engine
crates/blit-core/src/dial.rs:133:/// [`TransferDial::resize_settled`] for the epoch — with what actually
crates/blit-core/src/dial.rs:251:    /// True while a proposal is awaiting `resize_settled`.
crates/blit-core/src/dial.rs:280:    /// call [`Self::resize_settled`] with the outcome; until then
crates/blit-core/src/dial.rs:385:    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
crates/blit-core/src/dial.rs:608:                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs:838:        dial.resize_settled(1, 5, true);
crates/blit-core/src/dial.rs:870:        dial.resize_settled(1, 1, true);
crates/blit-core/src/dial.rs:908:        dial.resize_settled(proposal.epoch + 7, 9, true);
crates/blit-core/src/dial.rs:912:        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs:984:        dial.resize_settled(1, 2, true);
crates/blit-core/src/dial.rs:988:        dial.resize_settled(2, 3, true);
crates/blit-core/src/dial.rs:994:        dial.resize_settled(p3.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs:1010:        dial.resize_settled(p.epoch, 2, true);
crates/blit-core/src/dial.rs:1019:    async fn tuner_forwards_resize_proposals_over_the_shared_registry() {
crates/blit-core/src/transfer_session/data_plane.rs:28://! re-runs the shape table and proposes `DataPlaneResize{ADD}` (one stream
crates/blit-core/src/transfer_session/data_plane.rs:30://! `DataPlaneResizeAck` and grows its receive set. The control-lane frames
crates/blit-core/src/transfer_session/data_plane.rs:467:/// dialed instead of accepted. Resize (otp-5b-2): on a `DataPlaneResize`
crates/blit-core/src/transfer_session/data_plane.rs:626:/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
crates/blit-core/src/transfer_session/data_plane.rs:627:/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
crates/blit-core/src/transfer_session/data_plane.rs:630:pub(super) struct PendingResize {
crates/blit-core/src/transfer_session/data_plane.rs:862:    /// `live_streams()` for observability and calls `resize_settled` as
crates/blit-core/src/transfer_session/data_plane.rs:871:    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
crates/blit-core/src/transfer_session/data_plane.rs:877:    ) -> Result<Option<PendingResize>> {
crates/blit-core/src/transfer_session/data_plane.rs:886:        Ok(Some(PendingResize {
crates/blit-core/src/remote/transfer/data_plane.rs:25:/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).

exec
/bin/zsh -lc "rg -n \"source_receive_half|SourceEvent|NeedComplete|NeedBatch|BlockHash|receiver_stream_ceiling|admit_resize|handle.*resize\" crates/blit-core/src/transfer_session/mod.rs crates/blit-core/src/transfer_session -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/transfer_session/mod.rs:40:    session_error, BlockHashList, BlockTransfer, BlockTransferComplete, ComparisonMode,
crates/blit-core/src/transfer_session/mod.rs:42:    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept, SessionError,
crates/blit-core/src/transfer_session/mod.rs:94:/// Floor: a `BlockHashList` costs 32 bytes per block, so absurdly small
crates/blit-core/src/transfer_session/mod.rs:119:/// One `BlockHashList` frame carries a partial's whole list; capped at
crates/blit-core/src/transfer_session/mod.rs:252:    /// `ManifestBatch` per NeedBatch emitted (the pull-direction
crates/blit-core/src/transfer_session/mod.rs:453:        Some(Frame::NeedBatch(_)) => "NeedBatch",
crates/blit-core/src/transfer_session/mod.rs:454:        Some(Frame::NeedComplete(_)) => "NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:455:        Some(Frame::BlockHashes(_)) => "BlockHashList",
crates/blit-core/src/transfer_session/mod.rs:919:enum SourceEvent {
crates/blit-core/src/transfer_session/mod.rs:922:    /// destination's `BlockHashList` for the same path arrives — the
crates/blit-core/src/transfer_session/mod.rs:927:    BlockHashes(BlockHashList),
crates/blit-core/src/transfer_session/mod.rs:928:    NeedComplete,
crates/blit-core/src/transfer_session/mod.rs:944:struct SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:945:    tx: mpsc::UnboundedSender<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:949:impl SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:950:    fn send(&self, event: SourceEvent) -> Result<(), mpsc::error::SendError<SourceEvent>> {
crates/blit-core/src/transfer_session/mod.rs:951:        if let SourceEvent::Fault(fault) = &event {
crates/blit-core/src/transfer_session/mod.rs:1040:    // an ordered transport, a NeedComplete arriving while this is
crates/blit-core/src/transfer_session/mod.rs:1042:    // received what we have not sent (contract: NeedComplete only
crates/blit-core/src/transfer_session/mod.rs:1048:    // `SourceEventSender`.
crates/blit-core/src/transfer_session/mod.rs:1061:        SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:1104:    events: SourceEventSender,
crates/blit-core/src/transfer_session/mod.rs:1110:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1116:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
crates/blit-core/src/transfer_session/mod.rs:1123:            Some(Frame::NeedBatch(batch)) => {
crates/blit-core/src/transfer_session/mod.rs:1136:                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1150:                            let _ = events.send(SourceEvent::ResumeNeed(h));
crates/blit-core/src/transfer_session/mod.rs:1153:                            let _ = events.send(SourceEvent::Need(h));
crates/blit-core/src/transfer_session/mod.rs:1156:                            let _ = events.send(SourceEvent::Fault(
crates/blit-core/src/transfer_session/mod.rs:1167:            Some(Frame::BlockHashes(list)) => {
crates/blit-core/src/transfer_session/mod.rs:1172:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1174:                            "BlockHashList for '{}' in a session opened without resume",
crates/blit-core/src/transfer_session/mod.rs:1180:                let _ = events.send(SourceEvent::BlockHashes(list));
crates/blit-core/src/transfer_session/mod.rs:1182:            Some(Frame::NeedComplete(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1186:                    // NeedComplete be processed late and pass as
crates/blit-core/src/transfer_session/mod.rs:1188:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1189:                        "NeedComplete before the source's ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:1193:                let _ = events.send(SourceEvent::NeedComplete);
crates/blit-core/src/transfer_session/mod.rs:1199:                let _ = events.send(SourceEvent::ResizeAck(ack));
crates/blit-core/src/transfer_session/mod.rs:1202:                let _ = events.send(SourceEvent::Summary(summary));
crates/blit-core/src/transfer_session/mod.rs:1206:                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs:1210:                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1220:/// HELD until its `BlockHashList` arrives (the contract's strict
crates/blit-core/src/transfer_session/mod.rs:1226:    ready: Vec<(FileHeader, BlockHashList)>,
crates/blit-core/src/transfer_session/mod.rs:1240:    mut events: mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1418:                    // one-ADD-per-epoch ramp races NeedComplete/payload
crates/blit-core/src/transfer_session/mod.rs:1566:                    "source receive half ended before NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1620:        Some(SourceEvent::Summary(summary)) => Ok(summary),
crates/blit-core/src/transfer_session/mod.rs:1621:        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1622:        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1624:                format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1627:        Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs:1629:                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1632:        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1633:            SessionFault::protocol_violation("duplicate NeedComplete"),
crates/blit-core/src/transfer_session/mod.rs:1635:        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1649:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1681:    event: SourceEvent,
crates/blit-core/src/transfer_session/mod.rs:1692:        SourceEvent::Need(header) => {
crates/blit-core/src/transfer_session/mod.rs:1695:                    format!("need for '{}' after NeedComplete", header.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1703:        SourceEvent::ResumeNeed(header) => {
crates/blit-core/src/transfer_session/mod.rs:1707:                        "resume need for '{}' after NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1717:            // HELD until its BlockHashList arrives; no duplicate is
crates/blit-core/src/transfer_session/mod.rs:1723:        SourceEvent::BlockHashes(list) => {
crates/blit-core/src/transfer_session/mod.rs:1742:                        "BlockHashList for '{}' block_size {bs} outside \
crates/blit-core/src/transfer_session/mod.rs:1755:                        "BlockHashList for '{}' without a held resume need",
crates/blit-core/src/transfer_session/mod.rs:1761:        SourceEvent::NeedComplete => {
crates/blit-core/src/transfer_session/mod.rs:1764:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1767:            // Ordered lane: the destination sends every BlockHashList
crates/blit-core/src/transfer_session/mod.rs:1768:            // before its NeedComplete, so a still-held resume need here
crates/blit-core/src/transfer_session/mod.rs:1774:                        "NeedComplete with {} resume need(s) missing their BlockHashList",
crates/blit-core/src/transfer_session/mod.rs:1782:        SourceEvent::ResizeAck(ack) => {
crates/blit-core/src/transfer_session/mod.rs:1813:        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1816:        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1856:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1893:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1899:            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
crates/blit-core/src/transfer_session/mod.rs:1912:            Some(SourceEvent::ResizeAck(_)) => continue,
crates/blit-core/src/transfer_session/mod.rs:1913:            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1914:            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1916:                    format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1919:            Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs:1922:                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
crates/blit-core/src/transfer_session/mod.rs:1927:            Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1929:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1932:            Some(SourceEvent::Summary(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1953:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
crates/blit-core/src/transfer_session/mod.rs:1960:async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1962:        Some(SourceEvent::Fault(fault)) => fault,
crates/blit-core/src/transfer_session/mod.rs:1963:        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1965:                "need for '{}' during the data-plane drain (after NeedComplete)",
crates/blit-core/src/transfer_session/mod.rs:1969:        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:1970:            "BlockHashList for '{}' during the data-plane drain",
crates/blit-core/src/transfer_session/mod.rs:1973:        Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1974:            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
crates/blit-core/src/transfer_session/mod.rs:1976:        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1979:        Some(SourceEvent::Summary(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1996:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
crates/blit-core/src/transfer_session/mod.rs:2002:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:2008:                Some(SourceEvent::Fault(fault)) => break Some(fault),
crates/blit-core/src/transfer_session/mod.rs:2188:    hashes: &BlockHashList,
crates/blit-core/src/transfer_session/mod.rs:2192:    // block_size was range-validated when the BlockHashList arrived
crates/blit-core/src/transfer_session/mod.rs:2734:    // granted path before its NeedBatch, claimed by both carriers (the
crates/blit-core/src/transfer_session/mod.rs:2803:                    let ceiling = crate::dial::receiver_stream_ceiling(
crates/blit-core/src/transfer_session/mod.rs:2834:    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
crates/blit-core/src/transfer_session/mod.rs:2985:                // NeedComplete only after ManifestComplete received
crates/blit-core/src/transfer_session/mod.rs:2988:                    .send(frame(Frame::NeedComplete(NeedComplete {})))
crates/blit-core/src/transfer_session/mod.rs:3408:                // BlockHashList the destination would never receive),
crates/blit-core/src/transfer_session/mod.rs:3502:/// by a `BlockHashList` for each resume-flagged entry in it (otp-7a).
crates/blit-core/src/transfer_session/mod.rs:3523:    // otp-10b-2: w6-1 denominator — each NeedBatch sent reports a
crates/blit-core/src/transfer_session/mod.rs:3525:    // the push SOURCE reports per NeedBatch received.
crates/blit-core/src/transfer_session/mod.rs:3543:    // completion set BEFORE the NeedBatch goes out. The source can only
crates/blit-core/src/transfer_session/mod.rs:3571:        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
crates/blit-core/src/transfer_session/mod.rs:3576:    // end's eventual NeedComplete.
crates/blit-core/src/transfer_session/mod.rs:3598:            .send(frame(Frame::BlockHashes(BlockHashList {
crates/blit-core/src/transfer_session/mod.rs:3772:/// wire shape of `BlockHashList.hashes`). Pure blocking FS work, so it
crates/blit-core/src/transfer_session/mod.rs:4322:        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs:4324:        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs:4357:        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs:4359:        tx.send(SourceEvent::Need(FileHeader {
crates/blit-core/src/transfer_session/mod.rs:4364:        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs:40:    session_error, BlockHashList, BlockTransfer, BlockTransferComplete, ComparisonMode,
crates/blit-core/src/transfer_session/mod.rs:42:    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept, SessionError,
crates/blit-core/src/transfer_session/mod.rs:94:/// Floor: a `BlockHashList` costs 32 bytes per block, so absurdly small
crates/blit-core/src/transfer_session/mod.rs:119:/// One `BlockHashList` frame carries a partial's whole list; capped at
crates/blit-core/src/transfer_session/mod.rs:252:    /// `ManifestBatch` per NeedBatch emitted (the pull-direction
crates/blit-core/src/transfer_session/mod.rs:453:        Some(Frame::NeedBatch(_)) => "NeedBatch",
crates/blit-core/src/transfer_session/mod.rs:454:        Some(Frame::NeedComplete(_)) => "NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:455:        Some(Frame::BlockHashes(_)) => "BlockHashList",
crates/blit-core/src/transfer_session/mod.rs:919:enum SourceEvent {
crates/blit-core/src/transfer_session/mod.rs:922:    /// destination's `BlockHashList` for the same path arrives — the
crates/blit-core/src/transfer_session/mod.rs:927:    BlockHashes(BlockHashList),
crates/blit-core/src/transfer_session/mod.rs:928:    NeedComplete,
crates/blit-core/src/transfer_session/mod.rs:944:struct SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:945:    tx: mpsc::UnboundedSender<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:949:impl SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:950:    fn send(&self, event: SourceEvent) -> Result<(), mpsc::error::SendError<SourceEvent>> {
crates/blit-core/src/transfer_session/mod.rs:951:        if let SourceEvent::Fault(fault) = &event {
crates/blit-core/src/transfer_session/mod.rs:1040:    // an ordered transport, a NeedComplete arriving while this is
crates/blit-core/src/transfer_session/mod.rs:1042:    // received what we have not sent (contract: NeedComplete only
crates/blit-core/src/transfer_session/mod.rs:1048:    // `SourceEventSender`.
crates/blit-core/src/transfer_session/mod.rs:1061:        SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs:1104:    events: SourceEventSender,
crates/blit-core/src/transfer_session/mod.rs:1110:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1116:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
crates/blit-core/src/transfer_session/mod.rs:1123:            Some(Frame::NeedBatch(batch)) => {
crates/blit-core/src/transfer_session/mod.rs:1136:                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1150:                            let _ = events.send(SourceEvent::ResumeNeed(h));
crates/blit-core/src/transfer_session/mod.rs:1153:                            let _ = events.send(SourceEvent::Need(h));
crates/blit-core/src/transfer_session/mod.rs:1156:                            let _ = events.send(SourceEvent::Fault(
crates/blit-core/src/transfer_session/mod.rs:1167:            Some(Frame::BlockHashes(list)) => {
crates/blit-core/src/transfer_session/mod.rs:1172:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1174:                            "BlockHashList for '{}' in a session opened without resume",
crates/blit-core/src/transfer_session/mod.rs:1180:                let _ = events.send(SourceEvent::BlockHashes(list));
crates/blit-core/src/transfer_session/mod.rs:1182:            Some(Frame::NeedComplete(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1186:                    // NeedComplete be processed late and pass as
crates/blit-core/src/transfer_session/mod.rs:1188:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1189:                        "NeedComplete before the source's ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs:1193:                let _ = events.send(SourceEvent::NeedComplete);
crates/blit-core/src/transfer_session/mod.rs:1199:                let _ = events.send(SourceEvent::ResizeAck(ack));
crates/blit-core/src/transfer_session/mod.rs:1202:                let _ = events.send(SourceEvent::Summary(summary));
crates/blit-core/src/transfer_session/mod.rs:1206:                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs:1210:                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1220:/// HELD until its `BlockHashList` arrives (the contract's strict
crates/blit-core/src/transfer_session/mod.rs:1226:    ready: Vec<(FileHeader, BlockHashList)>,
crates/blit-core/src/transfer_session/mod.rs:1240:    mut events: mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1418:                    // one-ADD-per-epoch ramp races NeedComplete/payload
crates/blit-core/src/transfer_session/mod.rs:1566:                    "source receive half ended before NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1620:        Some(SourceEvent::Summary(summary)) => Ok(summary),
crates/blit-core/src/transfer_session/mod.rs:1621:        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1622:        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1624:                format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1627:        Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs:1629:                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1632:        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1633:            SessionFault::protocol_violation("duplicate NeedComplete"),
crates/blit-core/src/transfer_session/mod.rs:1635:        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1649:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1681:    event: SourceEvent,
crates/blit-core/src/transfer_session/mod.rs:1692:        SourceEvent::Need(header) => {
crates/blit-core/src/transfer_session/mod.rs:1695:                    format!("need for '{}' after NeedComplete", header.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1703:        SourceEvent::ResumeNeed(header) => {
crates/blit-core/src/transfer_session/mod.rs:1707:                        "resume need for '{}' after NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1717:            // HELD until its BlockHashList arrives; no duplicate is
crates/blit-core/src/transfer_session/mod.rs:1723:        SourceEvent::BlockHashes(list) => {
crates/blit-core/src/transfer_session/mod.rs:1742:                        "BlockHashList for '{}' block_size {bs} outside \
crates/blit-core/src/transfer_session/mod.rs:1755:                        "BlockHashList for '{}' without a held resume need",
crates/blit-core/src/transfer_session/mod.rs:1761:        SourceEvent::NeedComplete => {
crates/blit-core/src/transfer_session/mod.rs:1764:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1767:            // Ordered lane: the destination sends every BlockHashList
crates/blit-core/src/transfer_session/mod.rs:1768:            // before its NeedComplete, so a still-held resume need here
crates/blit-core/src/transfer_session/mod.rs:1774:                        "NeedComplete with {} resume need(s) missing their BlockHashList",
crates/blit-core/src/transfer_session/mod.rs:1782:        SourceEvent::ResizeAck(ack) => {
crates/blit-core/src/transfer_session/mod.rs:1813:        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1816:        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1856:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1893:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:1899:            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
crates/blit-core/src/transfer_session/mod.rs:1912:            Some(SourceEvent::ResizeAck(_)) => continue,
crates/blit-core/src/transfer_session/mod.rs:1913:            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs:1914:            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1916:                    format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs:1919:            Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs:1922:                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
crates/blit-core/src/transfer_session/mod.rs:1927:            Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1929:                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs:1932:            Some(SourceEvent::Summary(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1953:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
crates/blit-core/src/transfer_session/mod.rs:1960:async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1962:        Some(SourceEvent::Fault(fault)) => fault,
crates/blit-core/src/transfer_session/mod.rs:1963:        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs:1965:                "need for '{}' during the data-plane drain (after NeedComplete)",
crates/blit-core/src/transfer_session/mod.rs:1969:        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:1970:            "BlockHashList for '{}' during the data-plane drain",
crates/blit-core/src/transfer_session/mod.rs:1973:        Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs:1974:            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
crates/blit-core/src/transfer_session/mod.rs:1976:        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1979:        Some(SourceEvent::Summary(_)) => {
crates/blit-core/src/transfer_session/mod.rs:1996:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
crates/blit-core/src/transfer_session/mod.rs:2002:    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs:2008:                Some(SourceEvent::Fault(fault)) => break Some(fault),
crates/blit-core/src/transfer_session/mod.rs:2188:    hashes: &BlockHashList,
crates/blit-core/src/transfer_session/mod.rs:2192:    // block_size was range-validated when the BlockHashList arrived
crates/blit-core/src/transfer_session/mod.rs:2734:    // granted path before its NeedBatch, claimed by both carriers (the
crates/blit-core/src/transfer_session/mod.rs:2803:                    let ceiling = crate::dial::receiver_stream_ceiling(
crates/blit-core/src/transfer_session/mod.rs:2834:    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
crates/blit-core/src/transfer_session/mod.rs:2985:                // NeedComplete only after ManifestComplete received
crates/blit-core/src/transfer_session/mod.rs:2988:                    .send(frame(Frame::NeedComplete(NeedComplete {})))
crates/blit-core/src/transfer_session/mod.rs:3408:                // BlockHashList the destination would never receive),
crates/blit-core/src/transfer_session/mod.rs:3502:/// by a `BlockHashList` for each resume-flagged entry in it (otp-7a).
crates/blit-core/src/transfer_session/mod.rs:3523:    // otp-10b-2: w6-1 denominator — each NeedBatch sent reports a
crates/blit-core/src/transfer_session/mod.rs:3525:    // the push SOURCE reports per NeedBatch received.
crates/blit-core/src/transfer_session/mod.rs:3543:    // completion set BEFORE the NeedBatch goes out. The source can only
crates/blit-core/src/transfer_session/mod.rs:3571:        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
crates/blit-core/src/transfer_session/mod.rs:3576:    // end's eventual NeedComplete.
crates/blit-core/src/transfer_session/mod.rs:3598:            .send(frame(Frame::BlockHashes(BlockHashList {
crates/blit-core/src/transfer_session/mod.rs:3772:/// wire shape of `BlockHashList.hashes`). Pure blocking FS work, so it
crates/blit-core/src/transfer_session/mod.rs:4322:        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs:4324:        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs:4357:        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs:4359:        tx.send(SourceEvent::Need(FileHeader {
crates/blit-core/src/transfer_session/mod.rs:4364:        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/data_plane.rs:72:/// its `NeedBatch`) and the data-plane receive (which claims each path
crates/blit-core/src/transfer_session/data_plane.rs:80:/// before sending that file's `BlockHashList`, and claims it inline on

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2710,3060p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  2710	        } else {
  2711	            crate::fs_enum::FileFilter::default()
  2712	        };
  2713	    let mut source_files: HashSet<String> = HashSet::new();
  2714	
  2715	    // otp-7a: resume. Headers of resume-granted needs are retained so a
  2716	    // record's completion can finalize with the manifest's
  2717	    // size/mtime/permissions and be validated against the grant. Both
  2718	    // the header map and the resumed counter are SHARED with the
  2719	    // data-plane receive (otp-7b) exactly as `outstanding` is: on the
  2720	    // data plane the control loop never sees block records, so the
  2721	    // NeedListSink claims resume grants and counts completions as they
  2722	    // land on the sockets. The block size is chosen below, once the
  2723	    // carrier is known (the ceiling is per carrier).
  2724	    let resume_enabled = resume_negotiated(&negotiated.open);
  2725	    let resume_headers: data_plane::ResumeHeaders = Arc::default();
  2726	    let files_resumed = Arc::new(std::sync::atomic::AtomicU64::new(0));
  2727	
  2728	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  2729	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  2730	    // insert-only, never removed, so a concurrent data-plane claim can
  2731	    // never re-open a grant (a duplicate manifest path is granted at
  2732	    // most once regardless of delivery timing). `outstanding` is the
  2733	    // not-yet-delivered COMPLETION set — inserted for each freshly
  2734	    // granted path before its NeedBatch, claimed by both carriers (the
  2735	    // in-stream arms inline, the data-plane NeedListSink as payloads
  2736	    // land), and empty at SourceDone. A count proxy was insufficient
  2737	    // (F1); merging the two into one set raced the data-plane claim
  2738	    // against the diff (fix-review F1).
  2739	    let mut granted: HashSet<String> = HashSet::new();
  2740	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  2741	
  2742	    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
  2743	    // bytes arrive on sockets (not the control lane). Set it up NOW —
  2744	    // concurrent with the diff loop below, and before the peer sends — so
  2745	    // the connections are established promptly. Which end connects depends
  2746	    // on connection role (otp-5b): a DESTINATION **responder** (push)
  2747	    // accepts sockets off its bound listener; a DESTINATION **initiator**
  2748	    // (pull) dials the grant it received on `data_plane_host`. Byte
  2749	    // direction is the same either way (DESTINATION receives). The
  2750	    // NeedListSink gives the socket receive the same need-list strictness
  2751	    // the in-stream control loop applies inline; AbortOnDrop (inside the
  2752	    // responder run) bounds the accept task to this future. `resize_live`
  2753	    // tracks the stream count this end has grown to (epoch-0 plus each
  2754	    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
  2755	    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
  2756	    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
  2757	    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  2758	        Arc::clone(&sink),
  2759	        Arc::clone(&outstanding),
  2760	        // otp-7b: only a resume session accepts block records on the
  2761	        // data plane; the sink validates + claims them against the same
  2762	        // shared grant state the in-stream arms use.
  2763	        resume_enabled.then(|| data_plane::ResumeRecv {
  2764	            headers: Arc::clone(&resume_headers),
  2765	            resumed: Arc::clone(&files_resumed),
  2766	        }),
  2767	    ));
  2768	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  2769	        match negotiated.responder_data_plane {
  2770	            // DESTINATION responder (push, otp-4b): accept + receive.
  2771	            Some(rdp) => {
  2772	                let initial = rdp.initial_streams() as usize;
  2773	                let run = rdp.spawn(recv_sink, progress.clone());
  2774	                let ceiling = run.ceiling;
  2775	                (
  2776	                    Some(data_plane::DestRecvPlane::Responder(run)),
  2777	                    initial,
  2778	                    ceiling,
  2779	                )
  2780	            }
  2781	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2782	            // SOURCE responder granted a data plane and we have a host to dial.
  2783	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2784	                (Some(grant), Some(host)) => {
  2785	                    let initial = grant.initial_streams.max(1) as usize;
  2786	                    let run = data_plane::dial_destination_data_plane(
  2787	                        host,
  2788	                        grant,
  2789	                        recv_sink,
  2790	                        progress.clone(),
  2791	                        instruments.trace_data_plane,
  2792	                    )
  2793	                    .await?;
  2794	                    // otp-5b-2: the pull data plane resizes too. Seed
  2795	                    // `resize_live` from the epoch-0 streams dialed and bound
  2796	                    // growth by the capacity THIS end advertised in its open
  2797	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2798	                    // responder's dial already clamps to, so both ends agree
  2799	                    // even when the caller advertised a max_streams below this
  2800	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2801	                    // Resize frame the initiator dials the epoch-N socket (vs
  2802	                    // the responder path's arm).
  2803	                    let ceiling = crate::dial::receiver_stream_ceiling(
  2804	                        negotiated.open.receiver_capacity.as_ref(),
  2805	                    );
  2806	                    (
  2807	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2808	                        initial,
  2809	                        ceiling,
  2810	                    )
  2811	                }
  2812	                // A grant with no host to dial is an inconsistent initiator
  2813	                // config: fail fast, mirroring the SOURCE initiator
  2814	                // (`source_send_half`). The SOURCE responder has already bound
  2815	                // and blocks accepting the socket this end would dial, so
  2816	                // silently taking the in-stream branch cannot fall back — it
  2817	                // would deadlock until the responder's accept times out. A
  2818	                // grant means the initiator MUST dial (contract §Transport).
  2819	                // (codex otp-5b-1 finding.)
  2820	                (Some(_), None) => {
  2821	                    return Err(eyre::Report::new(SessionFault::internal(
  2822	                        "responder granted a TCP data plane but this DESTINATION \
  2823	                     initiator has no host to dial",
  2824	                    )))
  2825	                }
  2826	                // No grant (the responder could not bind, or the initiator
  2827	                // asked for in-stream): the in-stream carrier.
  2828	                (None, _) => (None, 0usize, 0usize),
  2829	            },
  2830	        };
  2831	
  2832	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  2833	    // — it hashes first; the SOURCE reads the size from each
  2834	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  2835	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
  2836	    // here, after the carrier is settled.
  2837	    let resume_block_size = {
  2838	        let ceiling = if data_plane_recv.is_some() {
  2839	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2840	        } else {
  2841	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2842	        };
  2843	        match negotiated
  2844	            .open
  2845	            .resume
  2846	            .as_ref()
  2847	            .map(|r| r.block_size as usize)
  2848	            .unwrap_or(0)
  2849	        {
  2850	            0 => DEFAULT_BLOCK_SIZE,
  2851	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2852	        }
  2853	    };
  2854	
  2855	    let mut pending: Vec<FileHeader> = Vec::new();
  2856	    let mut needed_paths: Vec<String> = Vec::new();
  2857	    let mut manifest_complete = false;
  2858	    let mut files_written: u64 = 0;
  2859	    let mut bytes_written: u64 = 0;
  2860	
  2861	    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
  2862	    // loop so applies run concurrent with the diff, exactly as the
  2863	    // data-plane receive does.
  2864	    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));
  2865	
  2866	    loop {
  2867	        let received = match transport.recv().await? {
  2868	            Some(f) => f,
  2869	            None => {
  2870	                return Err(eyre::Report::new(SessionFault::internal(
  2871	                    "peer closed mid-session",
  2872	                )))
  2873	            }
  2874	        };
  2875	        match received.frame {
  2876	            Some(Frame::ManifestEntry(header)) => {
  2877	                if manifest_complete {
  2878	                    return Err(violation(format!(
  2879	                        "manifest entry '{}' after ManifestComplete",
  2880	                        header.relative_path
  2881	                    )));
  2882	                }
  2883	                // otp-6b: retain the full source path set for the mirror
  2884	                // diff (the need list keeps only files needing transfer).
  2885	                if mirror_enabled {
  2886	                    source_files.insert(header.relative_path.clone());
  2887	                }
  2888	                pending.push(header);
  2889	                if pending.len() >= DEST_DIFF_CHUNK {
  2890	                    let chunk = std::mem::take(&mut pending);
  2891	                    if let Some(la) = &local_apply {
  2892	                        diff_chunk_and_apply_local(
  2893	                            la,
  2894	                            &mut local_run,
  2895	                            chunk,
  2896	                            dst_root,
  2897	                            canonical_dst_root.as_deref(),
  2898	                            &compare_opts,
  2899	                            &mut granted,
  2900	                            &mut needed_paths,
  2901	                            progress.as_ref(),
  2902	                        )
  2903	                        .await?;
  2904	                    } else {
  2905	                        diff_chunk_and_send_needs(
  2906	                            transport,
  2907	                            chunk,
  2908	                            dst_root,
  2909	                            canonical_dst_root.as_deref(),
  2910	                            &compare_opts,
  2911	                            resume_enabled,
  2912	                            resume_block_size,
  2913	                            &resume_headers,
  2914	                            &mut granted,
  2915	                            &outstanding,
  2916	                            &mut needed_paths,
  2917	                            progress.as_ref(),
  2918	                        )
  2919	                        .await?;
  2920	                    }
  2921	                }
  2922	            }
  2923	            Some(Frame::ManifestComplete(complete)) => {
  2924	                if manifest_complete {
  2925	                    return Err(violation("duplicate ManifestComplete".into()));
  2926	                }
  2927	                // otp-6b: mirror deletions are data-loss-dangerous when the
  2928	                // source scan was incomplete — a source file missing from an
  2929	                // aborted scan would be misclassified extraneous and deleted
  2930	                // at the dest. Refuse here (before any transfer or deletion)
  2931	                // rather than partial-mirror. Matches the old paths'
  2932	                // require-complete-scan guard.
  2933	                if mirror_enabled && !complete.scan_complete {
  2934	                    return Err(eyre::Report::new(SessionFault::internal(
  2935	                        "mirror refused: the source scan did not complete \
  2936	                         (unreadable paths) — deleting now could remove files \
  2937	                         the source still has",
  2938	                    )));
  2939	                }
  2940	                // codex otp-9b F1 (R49-F2 on the session): an initiator
  2941	                // that declared "the source will be deleted after this
  2942	                // transfer" (`blit move`) must NOT get a success out of
  2943	                // an incomplete source scan — files the scan could not
  2944	                // read would be silently lost when the caller deletes
  2945	                // the source. Same abort point as the mirror guard.
  2946	                if negotiated.open.require_complete_scan && !complete.scan_complete {
  2947	                    return Err(eyre::Report::new(SessionFault::refusal(
  2948	                        session_error::Code::ScanIncomplete,
  2949	                        "transfer refused: the source scan did not complete \
  2950	                         (unreadable paths) and the operation requires a \
  2951	                         complete scan (move deletes the source afterwards)",
  2952	                    )));
  2953	                }
  2954	                let chunk = std::mem::take(&mut pending);
  2955	                if let Some(la) = &local_apply {
  2956	                    diff_chunk_and_apply_local(
  2957	                        la,
  2958	                        &mut local_run,
  2959	                        chunk,
  2960	                        dst_root,
  2961	                        canonical_dst_root.as_deref(),
  2962	                        &compare_opts,
  2963	                        &mut granted,
  2964	                        &mut needed_paths,
  2965	                        progress.as_ref(),
  2966	                    )
  2967	                    .await?;
  2968	                } else {
  2969	                    diff_chunk_and_send_needs(
  2970	                        transport,
  2971	                        chunk,
  2972	                        dst_root,
  2973	                        canonical_dst_root.as_deref(),
  2974	                        &compare_opts,
  2975	                        resume_enabled,
  2976	                        resume_block_size,
  2977	                        &resume_headers,
  2978	                        &mut granted,
  2979	                        &outstanding,
  2980	                        &mut needed_paths,
  2981	                        progress.as_ref(),
  2982	                    )
  2983	                    .await?;
  2984	                }
  2985	                // NeedComplete only after ManifestComplete received
  2986	                // AND every entry diffed — both true here.
  2987	                transport
  2988	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  2989	                    .await?;
  2990	                manifest_complete = true;
  2991	            }
  2992	            Some(Frame::FileBegin(header)) => {
  2993	                // Payload records ride the control lane only under the
  2994	                // in-stream carrier; with a TCP data plane active they
  2995	                // flow over the sockets, so one here is a violation.
  2996	                if data_plane_recv.is_some() {
  2997	                    return Err(violation(format!(
  2998	                        "file record '{}' on the control lane while a TCP data plane is active",
  2999	                        header.relative_path
  3000	                    )));
  3001	                }
  3002	                if !manifest_complete {
  3003	                    return Err(violation(format!(
  3004	                        "payload record for '{}' before ManifestComplete",
  3005	                        header.relative_path
  3006	                    )));
  3007	                }
  3008	                // A resume-flagged grant may be satisfied ONLY by its
  3009	                // block record — a whole-file record for it bypasses the
  3010	                // hash choreography this end committed to (codex F3).
  3011	                if resume_headers
  3012	                    .lock()
  3013	                    .expect("resume-headers lock poisoned")
  3014	                    .contains_key(&header.relative_path)
  3015	                {
  3016	                    return Err(violation(format!(
  3017	                        "file record for resume-flagged '{}' — the contract requires \
  3018	                         its block record",
  3019	                        header.relative_path
  3020	                    )));
  3021	                }
  3022	                if !outstanding
  3023	                    .lock()
  3024	                    .expect("outstanding-needs lock poisoned")
  3025	                    .remove(&header.relative_path)
  3026	                {
  3027	                    return Err(violation(format!(
  3028	                        "payload for '{}' which is not on the need list",
  3029	                        header.relative_path
  3030	                    )));
  3031	                }
  3032	                let outcome = receive_file_record(transport, sink.as_ref(), &header).await?;
  3033	                files_written += outcome.files_written as u64;
  3034	                bytes_written += outcome.bytes_written;
  3035	                // otp-10b-2: in-stream per-file progress, same convention
  3036	                // as the data-plane receive (`execute_receive_pipeline`):
  3037	                // bytes ride Payload, FileComplete is byteless.
  3038	                if let Some(p) = &progress {
  3039	                    p.report_payload(0, outcome.bytes_written);
  3040	                    p.report_file_complete(header.relative_path.clone());
  3041	                }
  3042	            }
  3043	            Some(Frame::Block(block)) => {
  3044	                // otp-7a: a resume block record opens with its first
  3045	                // BlockTransfer (no begin frame). Claim the need and run
  3046	                // the strictly-serialized record to its completion frame.
  3047	                let header = claim_resume_record(
  3048	                    &block.relative_path,
  3049	                    resume_enabled,
  3050	                    data_plane_recv.is_some(),
  3051	                    manifest_complete,
  3052	                    &resume_headers,
  3053	                    &outstanding,
  3054	                )?;
  3055	                let outcome =
  3056	                    receive_block_record(transport, sink.as_ref(), &header, block).await?;
  3057	                files_written += outcome.files_written as u64;
  3058	                bytes_written += outcome.bytes_written;
  3059	                files_resumed.fetch_add(1, Ordering::Relaxed);
  3060	                // The whole block record (patch bytes + completion) ran

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1640,1950p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/dial.rs | sed -n '268,430p'; nl -ba crates/blit-core/src/dial.rs | sed -n '560,780p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1640	        ))),
  1641	    }
  1642	}
  1643	
  1644	/// Process every event ready right now (needs accumulating, resize acks
  1645	/// dialing their epoch-N socket) without blocking. Called between
  1646	/// manifest sends and at the top of the payload loop.
  1647	#[allow(clippy::too_many_arguments)]
  1648	async fn drain_ready_source_events(
  1649	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1650	    pending: &mut Vec<FileHeader>,
  1651	    resume: &mut ResumeSendState,
  1652	    need_complete: &mut bool,
  1653	    needed_bytes: &mut u64,
  1654	    needed_count: &mut usize,
  1655	    data_plane: Option<&data_plane::SourceDataPlane>,
  1656	    tx: &mut Box<dyn FrameTx>,
  1657	    pending_resize: &mut Option<data_plane::PendingResize>,
  1658	) -> Result<()> {
  1659	    while let Ok(event) = events.try_recv() {
  1660	        process_source_event(
  1661	            event,
  1662	            pending,
  1663	            resume,
  1664	            need_complete,
  1665	            needed_bytes,
  1666	            needed_count,
  1667	            data_plane,
  1668	            tx,
  1669	            pending_resize,
  1670	        )
  1671	        .await?;
  1672	    }
  1673	    Ok(())
  1674	}
  1675	
  1676	/// Handle one source event. Needs accumulate into `pending` and the
  1677	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1678	/// next ADD (the one-per-epoch ramp).
  1679	#[allow(clippy::too_many_arguments)]
  1680	async fn process_source_event(
  1681	    event: SourceEvent,
  1682	    pending: &mut Vec<FileHeader>,
  1683	    resume: &mut ResumeSendState,
  1684	    need_complete: &mut bool,
  1685	    needed_bytes: &mut u64,
  1686	    needed_count: &mut usize,
  1687	    data_plane: Option<&data_plane::SourceDataPlane>,
  1688	    tx: &mut Box<dyn FrameTx>,
  1689	    pending_resize: &mut Option<data_plane::PendingResize>,
  1690	) -> Result<()> {
  1691	    match event {
  1692	        SourceEvent::Need(header) => {
  1693	            if *need_complete {
  1694	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1695	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1696	                )));
  1697	            }
  1698	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1699	            *needed_count += 1;
  1700	            pending.push(header);
  1701	            Ok(())
  1702	        }
  1703	        SourceEvent::ResumeNeed(header) => {
  1704	            if *need_complete {
  1705	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1706	                    format!(
  1707	                        "resume need for '{}' after NeedComplete",
  1708	                        header.relative_path
  1709	                    ),
  1710	                )));
  1711	            }
  1712	            // Shape totals count the whole file — the diff hasn't run
  1713	            // yet, so the need list's implied workload is the honest
  1714	            // upper bound (same accounting a plain need gets).
  1715	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1716	            *needed_count += 1;
  1717	            // HELD until its BlockHashList arrives; no duplicate is
  1718	            // possible (the receive half's sent-map removal already
  1719	            // faults a second need for the same path).
  1720	            resume.held.insert(header.relative_path.clone(), header);
  1721	            Ok(())
  1722	        }
  1723	        SourceEvent::BlockHashes(list) => {
  1724	            // Validate the wire block size at ARRIVAL (codex F5), not
  1725	            // when the record is eventually sent — pending plain files
  1726	            // go out first, and an already-invalid frame must fail fast.
  1727	            // A conforming destination clamps into this range (D5 /
  1728	            // D-2026-07-10-1); same-build peers make a mismatch a
  1729	            // violation, never a negotiation. The ceiling is the
  1730	            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
  1731	            // records take up to the wire block cap; in-stream frames
  1732	            // must stay under the gRPC frame limit.
  1733	            let ceiling = if data_plane.is_some() {
  1734	                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  1735	            } else {
  1736	                MAX_IN_STREAM_RESUME_BLOCK_SIZE
  1737	            };
  1738	            let bs = list.block_size as usize;
  1739	            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
  1740	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1741	                    format!(
  1742	                        "BlockHashList for '{}' block_size {bs} outside \
  1743	                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
  1744	                        list.relative_path
  1745	                    ),
  1746	                )));
  1747	            }
  1748	            match resume.held.remove(&list.relative_path) {
  1749	                Some(header) => {
  1750	                    resume.ready.push((header, list));
  1751	                    Ok(())
  1752	                }
  1753	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1754	                    format!(
  1755	                        "BlockHashList for '{}' without a held resume need",
  1756	                        list.relative_path
  1757	                    ),
  1758	                ))),
  1759	            }
  1760	        }
  1761	        SourceEvent::NeedComplete => {
  1762	            if *need_complete {
  1763	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1764	                    "duplicate NeedComplete",
  1765	                )));
  1766	            }
  1767	            // Ordered lane: the destination sends every BlockHashList
  1768	            // before its NeedComplete, so a still-held resume need here
  1769	            // means the peer broke the choreography — fail fast rather
  1770	            // than hang waiting for a list that can no longer arrive.
  1771	            if !resume.held.is_empty() {
  1772	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1773	                    format!(
  1774	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1775	                        resume.held.len()
  1776	                    ),
  1777	                )));
  1778	            }
  1779	            *need_complete = true;
  1780	            Ok(())
  1781	        }
  1782	        SourceEvent::ResizeAck(ack) => {
  1783	            let dp = data_plane.ok_or_else(|| {
  1784	                eyre::Report::new(SessionFault::protocol_violation(
  1785	                    "DataPlaneResizeAck on a session with no data plane",
  1786	                ))
  1787	            })?;
  1788	            // Match the ack to the in-flight proposal; stale/unsolicited
  1789	            // acks (wrong epoch, or none pending) are ignored, matching
  1790	            // old push. `take()` + restore keeps the borrow simple.
  1791	            let pending_r = match pending_resize.take() {
  1792	                Some(p) if p.epoch == ack.epoch => p,
  1793	                restored => {
  1794	                    *pending_resize = restored;
  1795	                    return Ok(());
  1796	                }
  1797	            };
  1798	            if ack.accepted {
  1799	                dp.add_stream(&pending_r.sub_token).await?;
  1800	                dp.dial()
  1801	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1802	                // Ramp one stream per accepted epoch: propose the next ADD.
  1803	                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1804	            } else {
  1805	                dp.dial()
  1806	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1807	                // A refusal is terminal for this shape ramp. Retrying the
  1808	                // same unattainable target under a fresh epoch would loop
  1809	                // forever; the settled live set still carries the transfer.
  1810	                Ok(())
  1811	            }
  1812	        }
  1813	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1814	            "TransferSummary before SourceDone",
  1815	        ))),
  1816	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1817	    }
  1818	}
  1819	
  1820	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1821	/// the stream count the accumulated need list implies, if none is in
  1822	/// flight. A no-op when the shape wants no more than the live count (the
  1823	/// dial returns `None`). Sends the frame and records the in-flight
  1824	/// proposal for the ack to match.
  1825	async fn maybe_propose_resize(
  1826	    dp: &data_plane::SourceDataPlane,
  1827	    tx: &mut Box<dyn FrameTx>,
  1828	    needed_bytes: u64,
  1829	    needed_count: usize,
  1830	    pending_resize: &mut Option<data_plane::PendingResize>,
  1831	) -> Result<()> {
  1832	    if pending_resize.is_some() {
  1833	        return Ok(());
  1834	    }
  1835	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1836	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1837	            op: DataPlaneResizeOp::Add as i32,
  1838	            epoch: proposal.epoch,
  1839	            target_stream_count: proposal.target_streams,
  1840	            sub_token: proposal.sub_token.clone(),
  1841	        })))
  1842	        .await?;
  1843	        *pending_resize = Some(proposal);
  1844	    }
  1845	    Ok(())
  1846	}
  1847	
  1848	/// Drive the one-stream-per-epoch shape ramp to its currently known target
  1849	/// before payload dispatch. Needs and resume hashes may continue arriving
  1850	/// while an ack is in flight, so process the shared SOURCE event lane rather
  1851	/// than waiting for only an ack. Each accepted ack proposes the next epoch
  1852	/// from the latest accumulated shape; the loop ends only when no proposal is
  1853	/// outstanding (target reached or the destination refused growth).
  1854	#[allow(clippy::too_many_arguments)]
  1855	async fn settle_shape_resizes(
  1856	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1857	    pending: &mut Vec<FileHeader>,
  1858	    resume: &mut ResumeSendState,
  1859	    need_complete: &mut bool,
  1860	    needed_bytes: &mut u64,
  1861	    needed_count: &mut usize,
  1862	    data_plane: &data_plane::SourceDataPlane,
  1863	    tx: &mut Box<dyn FrameTx>,
  1864	    pending_resize: &mut Option<data_plane::PendingResize>,
  1865	) -> Result<()> {
  1866	    while pending_resize.is_some() {
  1867	        let event = events.recv().await.ok_or_else(|| {
  1868	            eyre::Report::new(SessionFault::internal(
  1869	                "source receive half ended during data-plane shape resize",
  1870	            ))
  1871	        })?;
  1872	        process_source_event(
  1873	            event,
  1874	            pending,
  1875	            resume,
  1876	            need_complete,
  1877	            needed_bytes,
  1878	            needed_count,
  1879	            Some(data_plane),
  1880	            tx,
  1881	            pending_resize,
  1882	        )
  1883	        .await?;
  1884	    }
  1885	    Ok(())
  1886	}
  1887	
  1888	/// Block for the ack of the one in-flight resize and dial its socket (or
  1889	/// settle it refused). Does NOT propose further — it resolves exactly the
  1890	/// pending proposal so the destination's armed slot is consumed before we
  1891	/// finish the data plane.
  1892	async fn resolve_in_flight_resize(
  1893	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1894	    dp: &data_plane::SourceDataPlane,
  1895	    pending: data_plane::PendingResize,
  1896	) -> Result<()> {
  1897	    loop {
  1898	        match events.recv().await {
  1899	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1900	                if ack.accepted {
  1901	                    dp.add_stream(&pending.sub_token).await?;
  1902	                    dp.dial()
  1903	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1904	                } else {
  1905	                    dp.dial()
  1906	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1907	                }
  1908	                return Ok(());
  1909	            }
  1910	            // A stale ack for an already-settled epoch: ignore, keep
  1911	            // waiting for ours.
  1912	            Some(SourceEvent::ResizeAck(_)) => continue,
  1913	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1914	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1915	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1916	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1917	                )))
  1918	            }
  1919	            Some(SourceEvent::BlockHashes(l)) => {
  1920	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1921	                    format!(
  1922	                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
  1923	                        l.relative_path
  1924	                    ),
  1925	                )))
  1926	            }
  1927	            Some(SourceEvent::NeedComplete) => {
  1928	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1929	                    "duplicate NeedComplete",
  1930	                )))
  1931	            }
  1932	            Some(SourceEvent::Summary(_)) => {
  1933	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1934	                    "TransferSummary before SourceDone",
  1935	                )))
  1936	            }
  1937	            None => {
  1938	                return Err(eyre::Report::new(SessionFault::internal(
  1939	                    "source receive half ended with a resize in flight",
  1940	                )))
  1941	            }
  1942	        }
  1943	    }
  1944	}
  1945	
  1946	/// Await the next terminal signal the receive half forwards while the
  1947	/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
  1948	/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
  1949	/// abort the send and surface as the fault.
  1950	///

 succeeded in 0ms:
   268	    /// One resize-eligible tuner tick. Streams move only as the LAST
   269	    /// escalation step in either direction: the cheap dials must
   270	    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
   271	    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
   272	    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
   273	    /// since the last settle, and no proposal may be in flight. Idle
   274	    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
   275	    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
   276	    /// folded in at construction — `CapacityProfile.max_streams` is
   277	    /// authoritative per the proto). One stream per epoch.
   278	    ///
   279	    /// The caller must forward the returned proposal to the peer and
   280	    /// call [`Self::resize_settled`] with the outcome; until then
   281	    /// every subsequent tick returns `None`.
   282	    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
   283	        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
   284	            return None;
   285	        }
   286	        let ticks = self
   287	            .ticks_since_settle
   288	            .fetch_add(1, Ordering::Relaxed)
   289	            .saturating_add(1);
   290	        if delta_bytes == 0 {
   291	            self.resize_sustain.store(0, Ordering::Relaxed);
   292	            return None;
   293	        }
   294	        let live = self.live_streams.load(Ordering::Relaxed).max(1);
   295	        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
   296	            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
   297	            let next = prev.saturating_add(1);
   298	            self.resize_sustain.store(next, Ordering::Relaxed);
   299	            next
   300	        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO && self.cheap_dials_floored() {
   301	            let prev = self.resize_sustain.load(Ordering::Relaxed).min(0);
   302	            let next = prev.saturating_sub(1);
   303	            self.resize_sustain.store(next, Ordering::Relaxed);
   304	            next
   305	        } else {
   306	            self.resize_sustain.store(0, Ordering::Relaxed);
   307	            0
   308	        };
   309	        if ticks < RESIZE_COOLDOWN_TICKS {
   310	            return None;
   311	        }
   312	        let target = if sustain >= RESIZE_SUSTAIN_TICKS {
   313	            (live + 1).min(self.ceiling_max_streams.max(1))
   314	        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
   315	            live.saturating_sub(1).max(1)
   316	        } else {
   317	            return None;
   318	        };
   319	        if target == live {
   320	            // Already at the bound in the wanted direction.
   321	            self.resize_sustain.store(0, Ordering::Relaxed);
   322	            return None;
   323	        }
   324	        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
   325	        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
   326	        // another task, and a plain store here could stack two live
   327	        // proposals onto one epoch number.
   328	        if self
   329	            .pending_epoch
   330	            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
   331	            .is_err()
   332	        {
   333	            return None;
   334	        }
   335	        self.resize_sustain.store(0, Ordering::Relaxed);
   336	        Some(ResizeProposal {
   337	            epoch,
   338	            target_streams: target,
   339	            add: target > live,
   340	        })
   341	    }
   342	
   343	    /// sf-2: shape-correction proposal. On push the daemon proposes the
   344	    /// epoch-0 stream count from whatever manifest prefix it has seen at
   345	    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
   346	    /// many-tiny-file push can negotiate far fewer streams than
   347	    /// [`initial_stream_proposal`] assigns the full workload. As the
   348	    /// need list accumulates client-side, the client re-runs the shape
   349	    /// table and corrects upward through the normal resize wire.
   350	    ///
   351	    /// Unlike [`Self::resize_tick`] this is a definite signal — the
   352	    /// shape is known, not inferred from throughput — so there is no
   353	    /// sustain/cooldown discipline. It still honors one-in-flight and
   354	    /// the receiver-profile ceiling, still moves ONE stream per epoch
   355	    /// (the wire carries one `sub_token` per ADD), and never proposes
   356	    /// REMOVE: shrinking below a live count is throughput evidence and
   357	    /// stays the tuner's call.
   358	    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
   359	        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
   360	        let live = self.live_streams.load(Ordering::Relaxed).max(1);
   361	        if desired <= live {
   362	            return None;
   363	        }
   364	        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
   365	        if self
   366	            .pending_epoch
   367	            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
   368	            .is_err()
   369	        {
   370	            return None;
   371	        }
   372	        Some(ResizeProposal {
   373	            epoch,
   374	            target_streams: live + 1,
   375	            add: true,
   376	        })
   377	    }
   378	
   379	    /// Settle the in-flight proposal with what ACTUALLY happened:
   380	    /// `effective_streams` is the live count now in effect (from the
   381	    /// peer's ack, or the local count if a post-ack dial failed and
   382	    /// nothing changed). `accepted = false` leaves the live count
   383	    /// untouched. Stale epochs (not the pending one) are ignored.
   384	    /// Either way the cooldown clock restarts.
   385	    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
   386	        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
   387	            return;
   388	        }
   389	        self.pending_epoch.store(0, Ordering::Relaxed);
   390	        self.ticks_since_settle.store(0, Ordering::Relaxed);
   391	        self.resize_sustain.store(0, Ordering::Relaxed);
   392	        if accepted {
   393	            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
   394	            self.live_streams.store(clamped, Ordering::Relaxed);
   395	            self.resize_epoch.store(epoch, Ordering::Relaxed);
   396	        }
   397	    }
   398	
   399	    /// Raise max_streams toward the ceiling (used when a peer's
   400	    /// negotiation allows more than the floor; still profile-bounded).
   401	    pub fn allow_streams_up_to(&self, streams: usize) {
   402	        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
   403	        self.max_streams.store(clamped, Ordering::Relaxed);
   404	    }
   405	
   406	    // ── tuner steps ──────────────────────────────────────────────────
   407	    /// One upward step of the cheap dials: chunk ×2 toward the
   408	    /// ceiling, prefetch +50% (at least +1) toward the ceiling, and
   409	    /// the tcp buffer to its ceiling (affects future sockets).
   410	    /// Returns true if anything moved.
   411	    pub fn step_up_cheap_dials(&self) -> bool {
   412	        let mut moved = false;
   413	        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
   414	        let next = (chunk.saturating_mul(2)).min(self.ceiling_chunk_bytes);
   415	        if next > chunk {
   416	            self.chunk_bytes.store(next, Ordering::Relaxed);
   417	            moved = true;
   418	        }
   419	        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
   420	        let next = (prefetch + (prefetch / 2).max(1)).min(self.ceiling_prefetch);
   421	        if next > prefetch {
   422	            self.prefetch_count.store(next, Ordering::Relaxed);
   423	            moved = true;
   424	        }
   425	        let tcp = self.tcp_buffer_bytes.load(Ordering::Relaxed);
   426	        if tcp < self.ceiling_tcp_buffer_bytes {
   427	            self.tcp_buffer_bytes
   428	                .store(self.ceiling_tcp_buffer_bytes, Ordering::Relaxed);
   429	            moved = true;
   430	        }
   560	            let Some(dial) = weak.upgrade() else { return };
   561	            let (blocked, bytes, streams) = {
   562	                let probes = probes.lock().expect("probe registry poisoned");
   563	                let (b, n) = probes.iter().fold((0u64, 0u64), |(b, n), p| {
   564	                    let snap = p.snapshot();
   565	                    (b + snap.write_blocked_nanos, n + snap.bytes_sent)
   566	                });
   567	                (b, n, probes.len())
   568	            };
   569	            let elapsed = last_tick.elapsed();
   570	            last_tick = tokio::time::Instant::now();
   571	            // A retired stream leaves the registry, so the monotonic
   572	            // sums can shrink across a REMOVE. Re-baseline and treat
   573	            // the tick as no-signal rather than reading a bogus delta.
   574	            if blocked < last_blocked || bytes < last_bytes {
   575	                last_blocked = blocked;
   576	                last_bytes = bytes;
   577	                if let Some(tx) = &resize_tx {
   578	                    let _ = tx; // no proposal possible on a no-signal tick
   579	                    dial.resize_tick(0, 0.0);
   580	                }
   581	                continue;
   582	            }
   583	            let delta_blocked = blocked.saturating_sub(last_blocked);
   584	            let delta_bytes = bytes.saturating_sub(last_bytes);
   585	            last_blocked = blocked;
   586	            last_bytes = bytes;
   587	            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
   588	            // SIGNAL, not a clean pipe — stepping up during manifest /
   589	            // preparation stalls would ramp without evidence and break
   590	            // the conservative-start contract. ue-r2-2 review (panel
   591	            // F3): the idle tick must still reach `resize_tick` so a
   592	            // sustain streak cannot survive a stall — "consecutive
   593	            // busy ticks" means consecutive.
   594	            if delta_bytes == 0 {
   595	                if resize_tx.is_some() {
   596	                    dial.resize_tick(0, 0.0);
   597	                }
   598	                continue;
   599	            }
   600	            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
   601	            dial.apply_tick(ratio);
   602	            if let Some(tx) = &resize_tx {
   603	                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
   604	                    if tx.send(proposal).is_err() {
   605	                        // Controller gone (transfer tearing down):
   606	                        // release the pending slot so the dial state
   607	                        // stays honest for late readers.
   608	                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
   609	                    }
   610	                }
   611	            }
   612	        }
   613	    })
   614	}
   615	
   616	#[cfg(test)]
   617	mod tests {
   618	    use super::*;
   619	
   620	    fn profile(max_streams: u32, max_chunk: u64, max_inflight: u64) -> CapacityProfile {
   621	        CapacityProfile {
   622	            cpu_cores: 0,
   623	            drain_class: 0,
   624	            load_percent: 0,
   625	            max_streams,
   626	            drain_rate_bytes_per_sec: 0,
   627	            max_chunk_bytes: max_chunk,
   628	            max_inflight_bytes: max_inflight,
   629	        }
   630	    }
   631	
   632	    #[test]
   633	    fn conservative_start_is_the_old_floor_tier() {
   634	        let dial = TransferDial::conservative();
   635	        assert_eq!(dial.chunk_bytes(), 16 * MIB);
   636	        assert_eq!(dial.prefetch_count(), 4);
   637	        assert_eq!(dial.tcp_buffer_bytes(), None);
   638	        assert_eq!(dial.initial_streams(), 4);
   639	        assert_eq!(dial.max_streams(), 8);
   640	    }
   641	
   642	    #[test]
   643	    fn unknown_profile_fields_keep_default_ceilings() {
   644	        let dial = TransferDial::conservative_within(Some(&profile(0, 0, 0)));
   645	        // Ramp fully: unknown (0) fields must not lower — or lift —
   646	        // anything relative to the defaults.
   647	        while dial.step_up_cheap_dials() {}
   648	        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
   649	        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
   650	        assert_eq!(dial.tcp_buffer_bytes(), Some(DIAL_CEILING_TCP_BUFFER_BYTES));
   651	        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
   652	    }
   653	
   654	    #[test]
   655	    fn profile_lowers_ceilings_but_never_raises_them() {
   656	        let dial =
   657	            TransferDial::conservative_within(Some(&profile(4, 32 * MIB as u64, 64 * MIB as u64)));
   658	        while dial.step_up_cheap_dials() {}
   659	        assert_eq!(dial.chunk_bytes(), 32 * MIB, "chunk ceiling from profile");
   660	        // 64 MiB in-flight ÷ 32 MiB chunk ceiling = 2 payload budget.
   661	        assert_eq!(dial.prefetch_count(), 2, "prefetch bounded by max_inflight");
   662	        assert_eq!(dial.ceiling_max_streams(), 4);
   663	
   664	        // codex F1: an in-flight budget smaller than one chunk bounds
   665	        // the chunk ceiling itself, even with max_chunk unknown (0).
   666	        let tight = TransferDial::conservative_within(Some(&profile(0, 0, 8 * MIB as u64)));
   667	        while tight.step_up_cheap_dials() {}
   668	        assert_eq!(tight.chunk_bytes(), 8 * MIB);
   669	        assert_eq!(tight.prefetch_count(), 1);
   670	
   671	        let generous = TransferDial::conservative_within(Some(&profile(999, u64::MAX, u64::MAX)));
   672	        while generous.step_up_cheap_dials() {}
   673	        assert_eq!(generous.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
   674	        assert_eq!(generous.prefetch_count(), DIAL_CEILING_PREFETCH);
   675	        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
   676	    }
   677	
   678	    #[test]
   679	    fn steps_respect_floor_and_ceiling_with_hysteresis_band() {
   680	        let dial = TransferDial::conservative();
   681	        assert!(!dial.step_down_cheap_dials(), "already at the floor");
   682	        assert!(dial.apply_tick(0.0), "clean telemetry steps up");
   683	        assert_eq!(dial.chunk_bytes(), 32 * MIB);
   684	        assert!(
   685	            !dial.apply_tick(0.15),
   686	            "inside the hysteresis band nothing moves"
   687	        );
   688	        assert!(dial.apply_tick(0.9), "blocked telemetry steps down");
   689	        assert_eq!(dial.chunk_bytes(), 16 * MIB);
   690	        while dial.apply_tick(0.0) {}
   691	        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
   692	        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);
   693	    }
   694	
   695	    #[test]
   696	    fn initial_stream_proposal_matches_the_retired_daemon_table() {
   697	        const MIB64: u64 = 1024 * 1024;
   698	        const GIB: u64 = 1024 * MIB64;
   699	        // Empty need-list → 1 (the old ladder's empty-guard).
   700	        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
   701	        // Byte-keyed tiers: exact lower boundaries AND just-below each
   702	        // (codex ue-r2-1f: representative values would miss a doubled
   703	        // threshold).
   704	        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
   705	        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
   706	        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
   707	        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
   708	        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
   709	        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
   710	        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
   711	        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
   712	        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
   713	        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
   714	        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
   715	        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
   716	        // File-count keys fire independently of bytes.
   717	        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
   718	        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
   719	        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
   720	        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
   721	        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
   722	        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
   723	        // Ceiling clamps the proposal (receiver profile authority).
   724	        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
   725	        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
   726	    }
   727	
   728	    #[test]
   729	    fn blocked_ratio_handles_edges() {
   730	        use std::time::Duration;
   731	        assert_eq!(blocked_ratio(0, Duration::from_millis(500), 4), 0.0);
   732	        assert_eq!(blocked_ratio(1_000, Duration::ZERO, 4), 0.0, "no signal");
   733	        assert_eq!(blocked_ratio(1_000, Duration::from_millis(500), 0), 0.0);
   734	        let half = blocked_ratio(500_000_000, Duration::from_millis(500), 2);
   735	        assert!((half - 0.5).abs() < 1e-9, "got {half}");
   736	        assert_eq!(
   737	            blocked_ratio(u64::MAX, Duration::from_nanos(1), 1),
   738	            1.0,
   739	            "clamped"
   740	        );
   741	    }
   742	
   743	    #[tokio::test(start_paused = true)]
   744	    async fn tuner_steps_up_on_clean_telemetry_and_exits_when_dial_drops() {
   745	        use crate::remote::transfer::progress::{StreamId, StreamProbe};
   746	        let dial = TransferDial::conservative().shared();
   747	        let probes = [StreamProbe::new(StreamId(0)), StreamProbe::new(StreamId(1))];
   748	        let tuner_view: Vec<StreamProbe> = probes
   749	            .iter()
   750	            .map(|p| StreamProbe::from_telemetry(p.id(), p.telemetry()))
   751	            .collect();
   752	        let handle = spawn_dial_tuner(&dial, tuner_view);
   753	        // Let the spawned task run to its first sleep so the timer is
   754	        // registered before the clock moves.
   755	        tokio::task::yield_now().await;
   756	
   757	        // codex F2: an idle tick (no bytes moved) must NOT step.
   758	        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
   759	        for _ in 0..16 {
   760	            tokio::task::yield_now().await;
   761	        }
   762	        assert_eq!(dial.chunk_bytes(), 16 * MIB, "idle tick is no signal");
   763	
   764	        // One tick WITH byte progress and zero blocked time: step up.
   765	        probes[0].record_bytes(1024);
   766	        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
   767	        for _ in 0..16 {
   768	            if dial.chunk_bytes() > 16 * MIB {
   769	                break;
   770	            }
   771	            tokio::task::yield_now().await;
   772	        }
   773	        assert_eq!(dial.chunk_bytes(), 32 * MIB, "stepped up once");
   774	
   775	        // Drop the transfer's dial: the tuner must self-terminate.
   776	        drop(dial);
   777	        tokio::time::advance(DIAL_TUNER_TICK + std::time::Duration::from_millis(10)).await;
   778	        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
   779	            .await
   780	            .expect("tuner exits after the dial drops")

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '900,1220p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1290,1645p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1,360p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '360,760p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   900	}
   901	
   902	/// Send the fault to the peer (best effort), mark it notified, and
   903	/// wrap it for return.
   904	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   905	    let _ = transport.send(error_frame(&fault)).await;
   906	    fault.peer_notified = true;
   907	    eyre::Report::new(fault)
   908	}
   909	
   910	// ---------------------------------------------------------------------------
   911	// SOURCE driver
   912	// ---------------------------------------------------------------------------
   913	
   914	/// Events the source's receive half forwards to its send half. The
   915	/// channel is unbounded but bounded by construction: every `Need`
   916	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   917	/// fault the session), so the queue never exceeds the source's own
   918	/// manifest size — the contract's bounded-buffering rule holds.
   919	enum SourceEvent {
   920	    Need(FileHeader),
   921	    /// A resume-flagged need (otp-7a). The send half HOLDS it until the
   922	    /// destination's `BlockHashList` for the same path arrives — the
   923	    /// contract's RELIABLE ordering guarantee: no byte of a resume file
   924	    /// moves before its hash list.
   925	    ResumeNeed(FileHeader),
   926	    /// The destination's block hashes for a held resume need (otp-7a).
   927	    BlockHashes(BlockHashList),
   928	    NeedComplete,
   929	    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
   930	    /// send half dials the epoch-N socket on `accepted`.
   931	    ResizeAck(DataPlaneResizeAck),
   932	    Summary(TransferSummary),
   933	    Fault(SessionFault),
   934	}
   935	
   936	/// The receive half's event sender, mirroring every `Fault` onto a
   937	/// `watch` signal as it is queued. The in-stream send path races this
   938	/// signal against its (potentially blocked) record sends — codex otp-8
   939	/// F1: a peer fault (CANCELLED above all) must interrupt a send half
   940	/// stuck inside `reader.read()`/`tx.send()`, exactly as the data-plane
   941	/// drain's `recv_peer_fault` arm does for socket sends. The mpsc queue
   942	/// still carries the fault for the between-send paths; the watch is a
   943	/// non-consuming side channel, so mid-send `Need`s stay queued.
   944	struct SourceEventSender {
   945	    tx: mpsc::UnboundedSender<SourceEvent>,
   946	    fault_signal: watch::Sender<Option<SessionFault>>,
   947	}
   948	
   949	impl SourceEventSender {
   950	    fn send(&self, event: SourceEvent) -> Result<(), mpsc::error::SendError<SourceEvent>> {
   951	        if let SourceEvent::Fault(fault) = &event {
   952	            let _ = self.fault_signal.send(Some(fault.clone()));
   953	        }
   954	        self.tx.send(event)
   955	    }
   956	}
   957	
   958	/// Resolves to the peer/receive-half fault the moment one is signalled;
   959	/// never resolves otherwise (the racing send future decides the
   960	/// outcome, mirroring `recv_peer_fault`'s closed-channel posture).
   961	async fn peer_fault_signalled(signal: &mut watch::Receiver<Option<SessionFault>>) -> SessionFault {
   962	    loop {
   963	        if let Some(fault) = signal.borrow_and_update().clone() {
   964	            return fault;
   965	        }
   966	        if signal.changed().await.is_err() {
   967	            // Sender dropped without ever signalling a fault: stay
   968	            // pending so the send future's own result decides.
   969	            std::future::pending::<()>().await;
   970	        }
   971	    }
   972	}
   973	
   974	/// Run the SOURCE role of one transfer session over `transport`.
   975	/// Returns the destination-computed `TransferSummary` (contract: the
   976	/// end that wrote the bytes is the end that attests to them).
   977	pub async fn run_source(
   978	    cfg: SourceSessionConfig,
   979	    transport: FrameTransport,
   980	    source: Arc<dyn TransferSource>,
   981	) -> Result<TransferSummary> {
   982	    let mut transport = transport;
   983	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   984	        // Own-config coherence: a source initiator declares SOURCE.
   985	        let declared = TransferRole::try_from(open.initiator_role);
   986	        if declared != Ok(TransferRole::Source) {
   987	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   988	        }
   989	        if let Err(fault) = source_open_validator(open) {
   990	            eyre::bail!("run_source initiator config unsupported: {fault}");
   991	        }
   992	    }
   993	
   994	    let negotiated = establish(
   995	        &mut transport,
   996	        &cfg.hello,
   997	        &cfg.endpoint,
   998	        TransferRole::Source,
   999	        &source_open_validator,
  1000	        // run_source only ever resolves nothing: a SOURCE *initiator*
  1001	        // owns its own root, and a SOURCE *responder* driven directly
  1002	        // (the in-process role suite) is handed a Fixed source. The
  1003	        // daemon SOURCE responder resolves module→root inside
  1004	        // `run_responder`, not here (otp-5).
  1005	        None,
  1006	    )
  1007	    .await?;
  1008	
  1009	    drive_source(
  1010	        cfg.plan_options,
  1011	        cfg.data_plane_host,
  1012	        cfg.instruments,
  1013	        negotiated,
  1014	        transport,
  1015	        source,
  1016	    )
  1017	    .await
  1018	}
  1019	
  1020	/// The SOURCE session body after establish: spawn the receive half,
  1021	/// run the send half, and map a fault to a peer-notified report. Shared
  1022	/// by [`run_source`] (initiator or direct-responder) and
  1023	/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
  1024	/// choreography is single-sourced.
  1025	async fn drive_source(
  1026	    plan_options: PlanOptions,
  1027	    data_plane_host: Option<String>,
  1028	    instruments: SourceInstruments,
  1029	    mut negotiated: Negotiated,
  1030	    transport: FrameTransport,
  1031	    source: Arc<dyn TransferSource>,
  1032	) -> Result<TransferSummary> {
  1033	    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
  1034	    // its send sockets on; a SOURCE initiator (push) has none and dials the
  1035	    // grant it received instead. Take it here so the send half owns it.
  1036	    let responder_data_plane = negotiated.responder_data_plane.take();
  1037	    let (mut tx, rx) = transport.split();
  1038	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
  1039	    // Set by the send half the moment ManifestComplete goes out. On
  1040	    // an ordered transport, a NeedComplete arriving while this is
  1041	    // still false is provably premature — the peer cannot have
  1042	    // received what we have not sent (contract: NeedComplete only
  1043	    // after ManifestComplete received + all entries diffed).
  1044	    let manifest_sent = Arc::new(AtomicBool::new(false));
  1045	    let (event_tx, event_rx) = mpsc::unbounded_channel();
  1046	    // Fault side-channel (codex otp-8 F1): the in-stream send path
  1047	    // races this signal against blocked record sends; see
  1048	    // `SourceEventSender`.
  1049	    let (fault_tx, fault_rx) = watch::channel(None::<SessionFault>);
  1050	    // AbortOnDrop: an early error return below must abort the receive
  1051	    // half instead of leaking it (same rationale as design-2 / w4-1).
  1052	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
  1053	        rx,
  1054	        Arc::clone(&sent),
  1055	        Arc::clone(&manifest_sent),
  1056	        resume_negotiated(&negotiated.open),
  1057	        // otp-10a: the recv half owns need-batch arrival, which is the
  1058	        // push-direction progress denominator (contract on
  1059	        // `ProgressEvent::ManifestBatch`: "push: need-list batches").
  1060	        instruments.progress.clone(),
  1061	        SourceEventSender {
  1062	            tx: event_tx,
  1063	            fault_signal: fault_tx,
  1064	        },
  1065	    )));
  1066	
  1067	    match source_send_half(
  1068	        plan_options,
  1069	        data_plane_host.as_deref(),
  1070	        instruments,
  1071	        &negotiated,
  1072	        responder_data_plane,
  1073	        &mut tx,
  1074	        source,
  1075	        sent,
  1076	        &manifest_sent,
  1077	        event_rx,
  1078	        fault_rx,
  1079	    )
  1080	    .await
  1081	    {
  1082	        Ok(summary) => Ok(summary),
  1083	        Err(report) => {
  1084	            let mut fault = fault_from_report(report);
  1085	            if !fault.peer_notified {
  1086	                let _ = tx.send(error_frame(&fault)).await;
  1087	                fault.peer_notified = true;
  1088	            }
  1089	            Err(eyre::Report::new(fault))
  1090	        }
  1091	    }
  1092	}
  1093	
  1094	/// Receive half of the source driver: drains the transport for the
  1095	/// whole session so destination sends can never deadlock against a
  1096	/// blocked source send, and routes the destination lane to the send
  1097	/// half. Terminates on summary, error, close, or violation.
  1098	async fn source_recv_half(
  1099	    mut rx: Box<dyn FrameRx>,
  1100	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
  1101	    manifest_sent: Arc<AtomicBool>,
  1102	    resume_session: bool,
  1103	    progress: Option<RemoteTransferProgress>,
  1104	    events: SourceEventSender,
  1105	) {
  1106	    loop {
  1107	        let received = match rx.recv().await {
  1108	            Ok(Some(f)) => f,
  1109	            Ok(None) => {
  1110	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
  1111	                    "peer closed before TransferSummary",
  1112	                )));
  1113	                return;
  1114	            }
  1115	            Err(err) => {
  1116	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
  1117	                    "transport receive failed: {err:#}"
  1118	                ))));
  1119	                return;
  1120	            }
  1121	        };
  1122	        match received.frame {
  1123	            Some(Frame::NeedBatch(batch)) => {
  1124	                // otp-10a: the need list is the push-direction progress
  1125	                // denominator ("N of M files"). Entries are unique by
  1126	                // contract (a duplicate need faults below), so every
  1127	                // batch is newly-requested work — same semantics as the
  1128	                // old push driver's `report_manifest_batch`.
  1129	                if let Some(p) = &progress {
  1130	                    if !batch.entries.is_empty() {
  1131	                        p.report_manifest_batch(batch.entries.len());
  1132	                    }
  1133	                }
  1134	                for entry in batch.entries {
  1135	                    if entry.resume && !resume_session {
  1136	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1137	                            format!(
  1138	                                "resume-flagged need for '{}' in a session opened without resume",
  1139	                                entry.relative_path
  1140	                            ),
  1141	                        )));
  1142	                        return;
  1143	                    }
  1144	                    let header = sent
  1145	                        .lock()
  1146	                        .expect("sent-manifest lock poisoned")
  1147	                        .remove(&entry.relative_path);
  1148	                    match header {
  1149	                        Some(h) if entry.resume => {
  1150	                            let _ = events.send(SourceEvent::ResumeNeed(h));
  1151	                        }
  1152	                        Some(h) => {
  1153	                            let _ = events.send(SourceEvent::Need(h));
  1154	                        }
  1155	                        None => {
  1156	                            let _ = events.send(SourceEvent::Fault(
  1157	                                SessionFault::protocol_violation(format!(
  1158	                                    "need for unknown or already-needed path '{}'",
  1159	                                    entry.relative_path
  1160	                                )),
  1161	                            ));
  1162	                            return;
  1163	                        }
  1164	                    }
  1165	                }
  1166	            }
  1167	            Some(Frame::BlockHashes(list)) => {
  1168	                // otp-7a: the destination's hashes for a resume-flagged
  1169	                // need. The send half correlates it with the held need;
  1170	                // in a non-resume session the frame is off-contract.
  1171	                if !resume_session {
  1172	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1173	                        format!(
  1174	                            "BlockHashList for '{}' in a session opened without resume",
  1175	                            list.relative_path
  1176	                        ),
  1177	                    )));
  1178	                    return;
  1179	                }
  1180	                let _ = events.send(SourceEvent::BlockHashes(list));
  1181	            }
  1182	            Some(Frame::NeedComplete(_)) => {
  1183	                if !manifest_sent.load(Ordering::Acquire) {
  1184	                    // Fail fast at arrival time (otp-3 codex F2): the
  1185	                    // event queue would otherwise let an early
  1186	                    // NeedComplete be processed late and pass as
  1187	                    // legitimate.
  1188	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1189	                        "NeedComplete before the source's ManifestComplete",
  1190	                    )));
  1191	                    return;
  1192	                }
  1193	                let _ = events.send(SourceEvent::NeedComplete);
  1194	            }
  1195	            Some(Frame::ResizeAck(ack)) => {
  1196	                // The destination's response to a shape-resize proposal
  1197	                // (otp-4b-2). Forward it to the send half, which owns the
  1198	                // dial and dials the epoch-N socket on `accepted`.
  1199	                let _ = events.send(SourceEvent::ResizeAck(ack));
  1200	            }
  1201	            Some(Frame::Summary(summary)) => {
  1202	                let _ = events.send(SourceEvent::Summary(summary));
  1203	                return;
  1204	            }
  1205	            Some(Frame::Error(err)) => {
  1206	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
  1207	                return;
  1208	            }
  1209	            other => {
  1210	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
  1211	                    format!("{} on the source's receive lane", frame_name(&other)),
  1212	                )));
  1213	                return;
  1214	            }
  1215	        }
  1216	    }
  1217	}
  1218	
  1219	/// otp-7a: the send half's resume bookkeeping. A resume-flagged need is
  1220	/// HELD until its `BlockHashList` arrives (the contract's strict
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch. Settle the whole shape-derived target before
  1417	                    // handing payloads to the pipeline: otherwise the
  1418	                    // one-ADD-per-epoch ramp races NeedComplete/payload
  1419	                    // drain, so a fast transfer can finish at a different
  1420	                    // worker count depending on which endpoint initiated.
  1421	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1422	                        .await?;
  1423	                    settle_shape_resizes(
  1424	                        &mut events,
  1425	                        &mut pending,
  1426	                        &mut resume,
  1427	                        &mut need_complete,
  1428	                        &mut needed_bytes,
  1429	                        &mut needed_count,
  1430	                        dp,
  1431	                        tx,
  1432	                        &mut pending_resize,
  1433	                    )
  1434	                    .await?;
  1435	                    let payloads =
  1436	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1437	                    // A cancel while earlier batches are actively moving
  1438	                    // closes the send pipeline under backpressure, so this
  1439	                    // queue fails with a data-plane error — prefer the
  1440	                    // peer's framed reason (CANCELLED) the same way the
  1441	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1442	                    // against events like finish(): live `Need`s still
  1443	                    // arrive here, and `recv_peer_fault` would consume them.
  1444	                    if let Err(dp_err) = dp.queue(payloads).await {
  1445	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1446	                    }
  1447	                }
  1448	                None => {
  1449	                    // codex otp-8 F1: race the record sends against the
  1450	                    // receive half's fault signal — the in-stream twin of
  1451	                    // the data-plane drain's `recv_peer_fault` arm. A peer
  1452	                    // cancel (framed CANCELLED, then RPC teardown) must
  1453	                    // interrupt a send blocked in `reader.read()` or in
  1454	                    // flow-controlled `tx.send()` and surface the framed
  1455	                    // reason, not hang or decay to INTERNAL. Biased:
  1456	                    // when both are ready, the framed fault wins.
  1457	                    tokio::select! {
  1458	                        biased;
  1459	                        fault = peer_fault_signalled(&mut fault_signal) => {
  1460	                            return Err(eyre::Report::new(fault));
  1461	                        }
  1462	                        res = send_payload_records(
  1463	                            tx,
  1464	                            &source,
  1465	                            plan_options,
  1466	                            batch,
  1467	                            &mut read_buf,
  1468	                            instruments.progress.as_ref(),
  1469	                        ) => {
  1470	                            res?;
  1471	                        }
  1472	                    }
  1473	                }
  1474	            }
  1475	            continue;
  1476	        }
  1477	        if !resume.ready.is_empty() {
  1478	            // The block phase for correlated (need, hash-list) pairs.
  1479	            // Data plane (otp-7b): each pair becomes ONE composite
  1480	            // ResumeFile work item, so one pipeline worker runs the
  1481	            // whole record on one socket — strict per-file serialization
  1482	            // without cross-socket reorder hazards. In-stream (otp-7a):
  1483	            // control-lane BlockTransfer/Complete frames, as before.
  1484	            let ready = std::mem::take(&mut resume.ready);
  1485	            match &mut data_plane {
  1486	                Some(dp) => {
  1487	                    // codex 7b-1 F4: resume batches drive the sf-2 shape
  1488	                    // correction exactly as plain batches do — a
  1489	                    // resume-heavy need list must not stay pinned to the
  1490	                    // zero-knowledge single stream.
  1491	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1492	                        .await?;
  1493	                    settle_shape_resizes(
  1494	                        &mut events,
  1495	                        &mut pending,
  1496	                        &mut resume,
  1497	                        &mut need_complete,
  1498	                        &mut needed_bytes,
  1499	                        &mut needed_count,
  1500	                        dp,
  1501	                        tx,
  1502	                        &mut pending_resize,
  1503	                    )
  1504	                    .await?;
  1505	                    let payloads = ready
  1506	                        .into_iter()
  1507	                        .map(|(header, hashes)| TransferPayload::ResumeFile {
  1508	                            header,
  1509	                            block_size: hashes.block_size,
  1510	                            dest_hashes: hashes.hashes,
  1511	                        })
  1512	                        .collect();
  1513	                    // Same cancel posture as the plain-batch queue above:
  1514	                    // prefer the peer's framed reason over the transport
  1515	                    // break a cancel also causes (otp-4b-3 codex F1).
  1516	                    if let Err(dp_err) = dp.queue(payloads).await {
  1517	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1518	                    }
  1519	                }
  1520	                None => {
  1521	                    for (header, hashes) in ready {
  1522	                        // codex 7b-2 G2: the whole in-stream record names
  1523	                        // its file on failure, matching the data-plane
  1524	                        // carrier's outer wrap. Same fault race as the
  1525	                        // plain-batch send above (codex otp-8 F1).
  1526	                        tokio::select! {
  1527	                            biased;
  1528	                            fault = peer_fault_signalled(&mut fault_signal) => {
  1529	                                return Err(eyre::Report::new(fault));
  1530	                            }
  1531	                            res = send_resume_block_records(
  1532	                                tx,
  1533	                                &source,
  1534	                                &header,
  1535	                                &hashes,
  1536	                                instruments.progress.as_ref(),
  1537	                            ) => {
  1538	                                res.map_err(|e| tag_path(e, &header.relative_path))?;
  1539	                            }
  1540	                        }
  1541	                    }
  1542	                }
  1543	            }
  1544	            continue;
  1545	        }
  1546	        if need_complete {
  1547	            break;
  1548	        }
  1549	        match events.recv().await {
  1550	            Some(event) => {
  1551	                process_source_event(
  1552	                    event,
  1553	                    &mut pending,
  1554	                    &mut resume,
  1555	                    &mut need_complete,
  1556	                    &mut needed_bytes,
  1557	                    &mut needed_count,
  1558	                    data_plane.as_ref(),
  1559	                    tx,
  1560	                    &mut pending_resize,
  1561	                )
  1562	                .await?;
  1563	            }
  1564	            None => {
  1565	                return Err(eyre::Report::new(SessionFault::internal(
  1566	                    "source receive half ended before NeedComplete",
  1567	                )))
  1568	            }
  1569	        }
  1570	    }
  1571	
  1572	    // A resize proposed on the last batch may still be in flight. Resolve
  1573	    // it BEFORE finishing so the destination's armed slot is consumed by
  1574	    // the dialed socket — an armed-but-never-dialed credential would hang
  1575	    // its accept loop (which waits for every arm to be claimed). We do not
  1576	    // propose further here: exactly the one in-flight resize is drained.
  1577	    if let Some(dp) = &data_plane {
  1578	        if let Some(pending) = pending_resize.take() {
  1579	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1580	        }
  1581	    }
  1582	
  1583	    // Close the data plane BEFORE SourceDone so the destination's receive
  1584	    // pipeline sees each socket's END record and completes; SourceDone on
  1585	    // the control lane then lets the destination score and summarize.
  1586	    //
  1587	    // The drain is the byte-transfer phase's wall-time sink, so a
  1588	    // mid-transfer cancel almost always lands here. Race it against a
  1589	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
  1590	    // the served session frames `SessionError{CANCELLED}`, and the source
  1591	    // must surface THAT — not the data-plane transport break it also
  1592	    // causes. Two orderings, both covered:
  1593	    //   * fault arrives while the drain is still pending (e.g. a worker
  1594	    //     blocked reading a slow file, so the socket break never unblocks
  1595	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
  1596	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
  1597	    //     stops the in-flight workers.
  1598	    //   * the socket break makes `finish()` return `Err` first → prefer
  1599	    //     the framed reason if the control lane delivers one within the
  1600	    //     stall window (`prefer_peer_fault`).
  1601	    if let Some(dp) = data_plane.take() {
  1602	        tokio::select! {
  1603	            biased;
  1604	            fault = recv_peer_fault(&mut events) => {
  1605	                return Err(eyre::Report::new(fault));
  1606	            }
  1607	            res = dp.finish() => {
  1608	                if let Err(dp_err) = res {
  1609	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
  1610	                }
  1611	            }
  1612	        }
  1613	    }
  1614	
  1615	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
  1616	
  1617	    // CLOSING: the destination is the scorer; the next event must be
  1618	    // its summary (the receive half ends after forwarding it).
  1619	    match events.recv().await {
  1620	        Some(SourceEvent::Summary(summary)) => Ok(summary),
  1621	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
  1622	        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1623	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1624	                format!("need for '{}' after NeedComplete", h.relative_path),
  1625	            )))
  1626	        }
  1627	        Some(SourceEvent::BlockHashes(l)) => {
  1628	            Err(eyre::Report::new(SessionFault::protocol_violation(
  1629	                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
  1630	            )))
  1631	        }
  1632	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
  1633	            SessionFault::protocol_violation("duplicate NeedComplete"),
  1634	        )),
  1635	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
  1636	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
  1637	        )),
  1638	        None => Err(eyre::Report::new(SessionFault::internal(
  1639	            "source receive half ended before TransferSummary",
  1640	        ))),
  1641	    }
  1642	}
  1643	
  1644	/// Process every event ready right now (needs accumulating, resize acks
  1645	/// dialing their epoch-N socket) without blocking. Called between

 succeeded in 0ms:
     1	//! Session-side TCP data-plane orchestration (otp-4b).
     2	//!
     3	//! The unified session reuses blit-core's data-plane byte plumbing —
     4	//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
     5	//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
     6	//! its OWN choreography here. The per-direction drivers (the old
     7	//! daemon push service loop and `remote::push::client` send driver)
     8	//! were deleted at cutover (otp-10c-2); this file is the one
     9	//! choreography.
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
    22	//! 1 — the session data plane always starts single-stream (otp-4b-1) and
    23	//! grows via resize in BOTH directions (push otp-4b-2, pull otp-5b-2).
    24	//!
    25	//! Mid-transfer growth (otp-4b-2 push, otp-5b-2 pull): the SOURCE owns a
    26	//! [`TransferDial`] (bounded by the receiver's advertised capacity) and
    27	//! drives the sf-2 shape correction — as the need list accumulates it
    28	//! re-runs the shape table and proposes `DataPlaneResize{ADD}` (one stream
    29	//! per epoch) on the control lane; the DESTINATION replies
    30	//! `DataPlaneResizeAck` and grows its receive set. The control-lane frames
    31	//! are identical in both directions — only the transport action flips
    32	//! (the connection-initiating end always dials, the responder always
    33	//! accepts): in push the SOURCE **initiator** dials the epoch-N socket and
    34	//! the DESTINATION **responder** arms+accepts it; in pull the DESTINATION
    35	//! **initiator** dials and the SOURCE **responder** accepts. Either way
    36	//! the SOURCE hands its new send socket to the running elastic pipeline
    37	//! via [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
    38	//! still future work — the resize moves only the stream count.
    39	
    40	use std::collections::{HashMap, HashSet};
    41	use std::path::{Path, PathBuf};
    42	use std::sync::atomic::{AtomicU64, Ordering};
    43	use std::sync::{Arc, Mutex as StdMutex};
    44	
    45	use async_trait::async_trait;
    46	use eyre::Result;
    47	use tokio::io::AsyncReadExt;
    48	use tokio::net::{TcpListener, TcpStream};
    49	use tokio::sync::mpsc;
    50	use tokio::task::JoinSet;
    51	
    52	use crate::buffer::BufferPool;
    53	use crate::dial::{initial_stream_proposal, local_receiver_capacity, TransferDial};
    54	use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
    55	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    56	use crate::remote::transfer::pipeline::execute_receive_pipeline;
    57	use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
    58	use crate::remote::transfer::socket::{
    59	    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    60	};
    61	use crate::remote::transfer::source::TransferSource;
    62	use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    63	use crate::remote::transfer::{
    64	    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession,
    65	    RemoteTransferProgress, SinkControl, SUB_TOKEN_LEN,
    66	};
    67	
    68	use super::{SessionFault, SourceInstruments};
    69	
    70	/// The set of granted-but-not-yet-received needs, shared between the
    71	/// destination's control loop (which inserts each path before sending
    72	/// its `NeedBatch`) and the data-plane receive (which claims each path
    73	/// as its payload lands). Completion is an empty set — the same signal
    74	/// the in-stream carrier uses via its inline `outstanding.remove`.
    75	pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;
    76	
    77	/// Headers of resume-granted needs (otp-7a/7b), keyed by relative path
    78	/// and retained until the grant's block record completes. Shared
    79	/// between the destination's control loop (which inserts each header
    80	/// before sending that file's `BlockHashList`, and claims it inline on
    81	/// the in-stream carrier) and the data-plane receive (which validates
    82	/// and claims it as block records land on the sockets) — the same
    83	/// sharing shape as [`OutstandingNeeds`].
    84	pub(super) type ResumeHeaders = Arc<StdMutex<HashMap<String, FileHeader>>>;
    85	
    86	/// otp-7b: the resume half of the data-plane receive contract — present
    87	/// only when the session negotiated resume. `headers` is the shared
    88	/// grant map above; `resumed` is the destination's `files_resumed`
    89	/// counter, incremented here because the control loop never sees
    90	/// data-plane block records.
    91	pub(super) struct ResumeRecv {
    92	    pub(super) headers: ResumeHeaders,
    93	    pub(super) resumed: Arc<AtomicU64>,
    94	}
    95	
    96	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    97	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    98	}
    99	
   100	/// [`dp_fault`] for failures that stringify an underlying I/O-bearing
   101	/// report (socket dials): carry the `io::ErrorKind` so the retry
   102	/// classifier still sees a transient transport condition (codex
   103	/// otp-10a F5).
   104	fn dp_fault_io(err: &eyre::Report, msg: impl Into<String>) -> eyre::Report {
   105	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg).with_io_kind_from(err))
   106	}
   107	
   108	// ---------------------------------------------------------------------------
   109	// Responder (DESTINATION) — bind, grant, accept, receive
   110	// ---------------------------------------------------------------------------
   111	
   112	/// A bound data-plane listener plus the credentials the responder
   113	/// advertises in its `SessionAccept`. Held by the responder driver
   114	/// across the handshake so the accept loop can run after establish.
   115	pub(super) struct ResponderDataPlane {
   116	    listener: TcpListener,
   117	    session_token: Vec<u8>,
   118	    epoch0_sub_token: Vec<u8>,
   119	    initial_streams: u32,
   120	    port: u16,
   121	}
   122	
   123	/// Bind a data-plane listener and mint credentials for the grant. Any
   124	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
   125	/// issues a grant-less `SessionAccept` and the session falls back to the
   126	/// in-stream carrier (contract §Transport selection: a responder that
   127	/// cannot bind grants no data plane).
   128	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
   129	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
   130	        Ok(listener) => listener,
   131	        Err(err) => {
   132	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
   133	            return None;
   134	        }
   135	    };
   136	    let port = match listener.local_addr() {
   137	        Ok(addr) => addr.port(),
   138	        Err(err) => {
   139	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   140	            return None;
   141	        }
   142	    };
   143	    // Two independent 16-byte credentials (contract §Transport: a socket
   144	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   145	    // is the fallible-RNG minter — a missing system RNG is an error, not
   146	    // a weaker credential.
   147	    let session_token = match generate_sub_token() {
   148	        Ok(token) => token,
   149	        Err(err) => {
   150	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   151	            return None;
   152	        }
   153	    };
   154	    let epoch0_sub_token = match generate_sub_token() {
   155	        Ok(token) => token,
   156	        Err(err) => {
   157	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   158	            return None;
   159	        }
   160	    };
   161	    // The grant is issued before any manifest is seen, so the proposal
   162	    // has zero knowledge: initial_streams == 1. All growth is via resize
   163	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   164	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   165	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   166	    Some(ResponderDataPlane {
   167	        listener,
   168	        session_token,
   169	        epoch0_sub_token,
   170	        initial_streams,
   171	        port,
   172	    })
   173	}
   174	
   175	/// Aggregated destination-side receive result: the write outcome plus
   176	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   177	/// which IS the settled live stream count this end observed. The sf-2
   178	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   179	pub(super) struct ReceiveTotals {
   180	    pub(super) outcome: SinkOutcome,
   181	    pub(super) streams: usize,
   182	}
   183	
   184	/// Live handle to a running responder data plane. The control loop arms
   185	/// resize credentials through [`Self::arm`] and joins the accept loop at
   186	/// `SourceDone` via [`Self::finish`].
   187	pub(super) struct ResponderDataPlaneRun {
   188	    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
   189	    task: AbortOnDrop<Result<ReceiveTotals>>,
   190	    /// The `session_token` half of every socket credential (the control
   191	    /// loop does not need it, but keeping it here documents the shape).
   192	    #[allow(dead_code)]
   193	    session_token: Vec<u8>,
   194	    /// The receiver's advertised `max_streams` — the control loop refuses
   195	    /// a resize that would grow past it (defense in depth; the source's
   196	    /// dial already clamps to the same ceiling).
   197	    pub(super) ceiling: usize,
   198	}
   199	
   200	impl ResponderDataPlane {
   201	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   202	    pub(super) fn grant(&self) -> DataPlaneGrant {
   203	        DataPlaneGrant {
   204	            tcp_port: self.port as u32,
   205	            session_token: self.session_token.clone(),
   206	            initial_streams: self.initial_streams,
   207	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   208	        }
   209	    }
   210	
   211	    /// The epoch-0 stream count this responder granted (always 1 — the
   212	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   213	    /// counter from it.
   214	    pub(super) fn initial_streams(&self) -> u32 {
   215	        self.initial_streams
   216	    }
   217	
   218	    /// Spawn the accept+receive loop and return a live handle. The loop
   219	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   220	    /// socket per armed resize credential until the control loop signals
   221	    /// `SourceDone` (drops the arm sender) and every receive worker has
   222	    /// drained its END. Runs concurrently with the control-stream diff
   223	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   224	    pub(super) fn spawn(
   225	        self,
   226	        sink: Arc<dyn TransferSink>,
   227	        progress: Option<RemoteTransferProgress>,
   228	    ) -> ResponderDataPlaneRun {
   229	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   230	        let session_token = self.session_token.clone();
   231	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
   232	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, progress, arm_rx)));
   233	        ResponderDataPlaneRun {
   234	            arm_tx,
   235	            task,
   236	            session_token,
   237	            ceiling,
   238	        }
   239	    }
   240	
   241	    async fn accept_loop(
   242	        self,
   243	        sink: Arc<dyn TransferSink>,
   244	        progress: Option<RemoteTransferProgress>,
   245	        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
   246	    ) -> Result<ReceiveTotals> {
   247	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   248	        let mut epoch0 = self.session_token.clone();
   249	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   250	
   251	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   252	        let mut total = SinkOutcome::default();
   253	        let mut streams = 0usize;
   254	
   255	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   256	        // grant is always 1; the loop handles N for symmetry).
   257	        for _ in 0..self.initial_streams {
   258	            let socket = accept_authenticated(&self.listener, &epoch0).await?;
   259	            streams += 1;
   260	            spawn_receive(&mut receives, socket, &sink, progress.clone());
   261	        }
   262	
   263	        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
   264	        // whose socket the SOURCE dials right after its ack. `no_more` is
   265	        // set when the control loop drops the arm sender at `SourceDone`;
   266	        // the loop then drains the last armed sockets and workers. Because
   267	        // the SOURCE only dials a credential it was acked for (and a dial
   268	        // failure faults the whole session, aborting this task via
   269	        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
   270	        let mut armed: Vec<Vec<u8>> = Vec::new();
   271	        let mut arm_rx = Some(arm_rx);
   272	        let mut no_more = false;
   273	        loop {
   274	            if no_more && armed.is_empty() && receives.is_empty() {
   275	                break;
   276	            }
   277	            // A closed arm channel resolves `recv()` instantly to `None`
   278	            // every poll; parking it on `pending()` once closed keeps the
   279	            // biased select from starving the accept/join arms (otherwise
   280	            // the None arm wins every race and the loop spins without ever
   281	            // collecting a finished worker).
   282	            let arm_recv = async {
   283	                match arm_rx.as_mut() {
   284	                    Some(rx) => rx.recv().await,
   285	                    None => std::future::pending().await,
   286	                }
   287	            };
   288	            tokio::select! {
   289	                biased;
   290	                // Control FIRST: an arm must register before its socket
   291	                // (which the SOURCE dials only after the ack the control
   292	                // loop sends right after arming), so the accept arm below
   293	                // always sees a populated `armed` set.
   294	                arm = arm_recv => match arm {
   295	                    Some(sub_token) => armed.push(sub_token),
   296	                    // Arm sender dropped at SourceDone: no more resizes.
   297	                    None => {
   298	                        arm_rx = None;
   299	                        no_more = true;
   300	                    }
   301	                },
   302	                // Accept only when a resize credential is armed. `accept`
   303	                // is cancel-safe, so losing this arm to another (its
   304	                // pending connection stays queued) drops no socket. The
   305	                // credential read happens OUTSIDE the select (below) so a
   306	                // select cancel can never truncate a half-read socket.
   307	                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
   308	                    let socket = accepted?;
   309	                    let socket =
   310	                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
   311	                    streams += 1;
   312	                    spawn_receive(&mut receives, socket, &sink, progress.clone());
   313	                }
   314	                joined = receives.join_next(), if !receives.is_empty() => {
   315	                    let outcome = joined
   316	                        .expect("join_next is None only when empty, guarded above")
   317	                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   318	                    total.files_written += outcome.files_written;
   319	                    total.bytes_written += outcome.bytes_written;
   320	                }
   321	            }
   322	        }
   323	        Ok(ReceiveTotals {
   324	            outcome: total,
   325	            streams,
   326	        })
   327	    }
   328	}
   329	
   330	impl ResponderDataPlaneRun {
   331	    /// Arm a resize credential so the next socket presenting
   332	    /// `session_token ‖ sub_token` is accepted. Returns false if the
   333	    /// accept loop is gone (its receiver dropped) — the control loop then
   334	    /// acks the resize as refused.
   335	    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
   336	        self.arm_tx.send(sub_token).is_ok()
   337	    }
   338	
   339	    /// Signal `SourceDone` (no more resizes) and join the accept loop for
   340	    /// the aggregated receive totals.
   341	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   342	        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
   343	        // Dropping the arm sender is the "no more resizes" signal.
   344	        drop(arm_tx);
   345	        task.join()
   346	            .await
   347	            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
   348	    }
   349	}
   350	
   351	/// Spawn one receive worker draining `socket` into `sink` via the shared
   352	/// receive pipeline, guarded by the transfer stall timeout (carried REV4
   353	/// RELIABLE invariant, matching the old push receive: a peer that
   354	/// authenticates then stalls mid-record trips the stall timeout rather
   355	/// than pinning the task until TCP keepalive).
   356	fn spawn_receive(
   357	    receives: &mut JoinSet<Result<SinkOutcome>>,
   358	    socket: TcpStream,
   359	    sink: &Arc<dyn TransferSink>,
   360	    progress: Option<RemoteTransferProgress>,
   360	    progress: Option<RemoteTransferProgress>,
   361	) {
   362	    let sink = Arc::clone(sink);
   363	    receives.spawn(async move {
   364	        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
   365	        execute_receive_pipeline(&mut guarded, sink, progress.as_ref()).await
   366	    });
   367	}
   368	
   369	/// Accept one data socket under the shared bounded-accept timeout and
   370	/// apply the data-plane socket policy. Cancel-safe (the accept itself is;
   371	/// no bytes are read here).
   372	async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
   373	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   374	    let socket = match accept {
   375	        Ok(Ok((socket, _peer))) => socket,
   376	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   377	        Err(_) => {
   378	            return Err(dp_fault(format!(
   379	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   380	        )))
   381	        }
   382	    };
   383	    configure_data_socket(&socket, None)
   384	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   385	    Ok(socket)
   386	}
   387	
   388	/// Read the fixed-length epoch-0 credential and verify it whole. A socket
   389	/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
   390	/// arms exactly the sockets it dials, so a mismatch is fatal here).
   391	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   392	    let mut socket = accept_raw(listener).await?;
   393	    let mut buf = vec![0u8; expected.len()];
   394	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   395	    match read {
   396	        Ok(Ok(_)) => {}
   397	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   398	        Err(_) => {
   399	            return Err(dp_fault(format!(
   400	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   401	            )))
   402	        }
   403	    }
   404	    // Constant-time comparison is not required: the tokens are 16 random
   405	    // bytes read once per socket, single-session; a timing oracle buys
   406	    // nothing against per-transfer secrets (same posture as the old push
   407	    // acceptor's `token == expected_token`).
   408	    if buf != expected {
   409	        return Err(dp_fault(
   410	            "data-plane socket presented an invalid credential",
   411	        ));
   412	    }
   413	    Ok(socket)
   414	}
   415	
   416	/// Read a resize socket's `session_token ‖ sub_token(16)` credential
   417	/// (bounded), verify the session token, and match the sub-token against
   418	/// an armed credential — removing it so each arm is consumed once. Runs
   419	/// in the accept loop body (never a select arm), so a select cancel can
   420	/// never truncate a half-read socket.
   421	async fn authenticate_resize(
   422	    socket: TcpStream,
   423	    session_token: &[u8],
   424	    armed: &mut Vec<Vec<u8>>,
   425	) -> Result<TcpStream> {
   426	    let mut socket = socket;
   427	    let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
   428	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   429	    match read {
   430	        Ok(Ok(_)) => {}
   431	        Ok(Err(err)) => {
   432	            return Err(dp_fault(format!(
   433	                "reading resize data-plane credential: {err}"
   434	            )))
   435	        }
   436	        Err(_) => {
   437	            return Err(dp_fault(format!(
   438	                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   439	            )))
   440	        }
   441	    }
   442	    if buf[..session_token.len()] != *session_token {
   443	        return Err(dp_fault(
   444	            "resize data socket presented a wrong session token",
   445	        ));
   446	    }
   447	    let sub = &buf[session_token.len()..];
   448	    match armed.iter().position(|t| t.as_slice() == sub) {
   449	        Some(idx) => {
   450	            armed.swap_remove(idx);
   451	            Ok(socket)
   452	        }
   453	        None => Err(dp_fault(
   454	            "resize data socket presented an unarmed credential",
   455	        )),
   456	    }
   457	}
   458	
   459	// ---------------------------------------------------------------------------
   460	// Initiator (DESTINATION) — dial, receive (otp-5b-1)
   461	// ---------------------------------------------------------------------------
   462	
   463	/// Live handle to a DESTINATION **initiator** receive data plane (the
   464	/// pull direction): the initiator dials the granted epoch-0 socket(s) and
   465	/// drains each into the sink through the shared receive pipeline — the
   466	/// same byte machinery the DESTINATION responder uses, only the socket is
   467	/// dialed instead of accepted. Resize (otp-5b-2): on a `DataPlaneResize`
   468	/// the control loop dials one more epoch-N socket via
   469	/// [`Self::add_dialed_stream`] (the pull mirror of the SOURCE responder's
   470	/// accept). [`Self::finish`] joins the workers for the aggregated write
   471	/// outcome + settled stream count.
   472	pub(super) struct InitiatorReceivePlaneRun {
   473	    receives: JoinSet<Result<SinkOutcome>>,
   474	    streams: usize,
   475	    /// The responder host+port and session token, retained so a resize can
   476	    /// dial another receive socket to the same listener (otp-5b-2). The
   477	    /// DESTINATION initiator always dials; the SOURCE responder accepts.
   478	    host: String,
   479	    tcp_port: u32,
   480	    session_token: Vec<u8>,
   481	    /// The shared need-list receive sink each dialed worker drains into.
   482	    sink: Arc<dyn TransferSink>,
   483	    /// w6-1 progress lane each receive worker reports into (otp-10b-2);
   484	    /// cloned per worker, including resize-added ones.
   485	    progress: Option<RemoteTransferProgress>,
   486	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   487	    /// otp-10b-2). Applied to the epoch-0 dials at construction and to
   488	    /// each epoch-N resize dial in [`Self::add_dialed_stream`].
   489	    trace: bool,
   490	}
   491	
   492	/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
   493	/// socket. `host` is the responder's host (the initiator reached the
   494	/// control plane there; the data plane rides the same host on the granted
   495	/// port — contract §Transport: the initiator always dials). Each worker
   496	/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
   497	/// in-stream carrier applies inline).
   498	pub(super) async fn dial_destination_data_plane(
   499	    host: &str,
   500	    grant: &DataPlaneGrant,
   501	    sink: Arc<dyn TransferSink>,
   502	    progress: Option<RemoteTransferProgress>,
   503	    trace: bool,
   504	) -> Result<InitiatorReceivePlaneRun> {
   505	    let initial = grant.initial_streams.max(1) as usize;
   506	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   507	    let mut handshake = grant.session_token.clone();
   508	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   509	    let addr = format!("{host}:{}", grant.tcp_port);
   510	
   511	    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   512	    let mut streams = 0usize;
   513	    for _ in 0..initial {
   514	        // `dial_data_plane` connects, applies the data-socket policy, and
   515	        // writes the handshake credential — the same bounded dial the
   516	        // SOURCE initiator uses (design-3: one owner for every client-side
   517	        // data-plane dial, both directions).
   518	        if trace {
   519	            eprintln!("[data-plane-client] connecting to {addr} (receive)");
   520	        }
   521	        let socket = dial_data_plane(&addr, &handshake, None)
   522	            .await
   523	            .map_err(|err| {
   524	                dp_fault_io(
   525	                    &err,
   526	                    format!("dialing session data plane (receive): {err:#}"),
   527	                )
   528	            })?;
   529	        streams += 1;
   530	        spawn_receive(&mut receives, socket, &sink, progress.clone());
   531	    }
   532	    Ok(InitiatorReceivePlaneRun {
   533	        receives,
   534	        streams,
   535	        host: host.to_string(),
   536	        tcp_port: grant.tcp_port,
   537	        session_token: grant.session_token.clone(),
   538	        sink,
   539	        progress,
   540	        trace,
   541	    })
   542	}
   543	
   544	impl InitiatorReceivePlaneRun {
   545	    /// Dial one epoch-N resize socket to the responder and spawn its
   546	    /// receive worker (otp-5b-2 — the pull mirror of the SOURCE
   547	    /// responder's accept). Credential `session_token ‖ sub_token`. A dial
   548	    /// failure is FATAL, matching the SOURCE initiator's `add_stream`: a
   549	    /// same-build peer that granted+bound epoch-0 failing an epoch-N dial
   550	    /// is a transport fault worth surfacing (the DESTINATION dials before
   551	    /// it acks, so a failure faults the session before the SOURCE
   552	    /// responder commits to accepting the socket).
   553	    pub(super) async fn add_dialed_stream(&mut self, sub_token: &[u8]) -> Result<()> {
   554	        let mut handshake = self.session_token.clone();
   555	        handshake.extend_from_slice(sub_token);
   556	        let addr = format!("{}:{}", self.host, self.tcp_port);
   557	        if self.trace {
   558	            eprintln!("[data-plane-client] connecting to {addr} (receive resize)");
   559	        }
   560	        let socket = dial_data_plane(&addr, &handshake, None)
   561	            .await
   562	            .map_err(|err| {
   563	                dp_fault_io(
   564	                    &err,
   565	                    format!("dialing resize data plane (receive): {err:#}"),
   566	                )
   567	            })?;
   568	        self.streams += 1;
   569	        spawn_receive(
   570	            &mut self.receives,
   571	            socket,
   572	            &self.sink,
   573	            self.progress.clone(),
   574	        );
   575	        Ok(())
   576	    }
   577	
   578	    /// Join every receive worker for the aggregated write totals. A worker
   579	    /// error (receive failure / stall) surfaces here; each drains to its
   580	    /// socket's END record on a clean transfer.
   581	    async fn finish(mut self) -> Result<ReceiveTotals> {
   582	        let mut total = SinkOutcome::default();
   583	        while let Some(joined) = self.receives.join_next().await {
   584	            let outcome =
   585	                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   586	            total.files_written += outcome.files_written;
   587	            total.bytes_written += outcome.bytes_written;
   588	        }
   589	        Ok(ReceiveTotals {
   590	            outcome: total,
   591	            streams: self.streams,
   592	        })
   593	    }
   594	}
   595	
   596	/// The DESTINATION end's receive data plane, tagged by connection role.
   597	/// Both drain socket bytes into the sink through the same receive
   598	/// pipeline; they differ only in how sockets are obtained (accept vs dial)
   599	/// and whether resize is armable (push only, otp-4b-2).
   600	pub(super) enum DestRecvPlane {
   601	    /// DESTINATION **responder** (push, otp-4b): accepts sockets; resize
   602	    /// grows the set by arming a credential its accept loop then accepts.
   603	    Responder(ResponderDataPlaneRun),
   604	    /// DESTINATION **initiator** (pull, otp-5b): dials sockets; resize grows
   605	    /// the set by dialing one more epoch-N socket (otp-5b-2).
   606	    Initiator(InitiatorReceivePlaneRun),
   607	}
   608	
   609	impl DestRecvPlane {
   610	    /// Drain the data plane to completion and report the settled stream
   611	    /// count + write outcome (the DESTINATION is the scorer).
   612	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   613	        match self {
   614	            DestRecvPlane::Responder(run) => run.finish().await,
   615	            DestRecvPlane::Initiator(run) => run.finish().await,
   616	        }
   617	    }
   618	}
   619	
   620	// ---------------------------------------------------------------------------
   621	// Initiator (SOURCE) — dial, authenticate, send, resize
   622	// ---------------------------------------------------------------------------
   623	
   624	/// A resize the SOURCE has proposed and minted a credential for but not
   625	/// yet completed: the driver has sent (or will send) the matching
   626	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   627	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   628	/// flight (the dial's `pending_epoch` enforces it; this is the
   629	/// driver-side record the ack is matched against).
   630	pub(super) struct PendingResize {
   631	    pub(super) epoch: u32,
   632	    pub(super) target_streams: u32,
   633	    pub(super) sub_token: Vec<u8>,
   634	}
   635	
   636	/// How the SOURCE acquires each epoch-N data socket for a shape resize —
   637	/// the two connection roles of otp-5b. Byte direction is identical (the
   638	/// SOURCE sends), and `propose_resize` is the same either way; only socket
   639	/// acquisition flips.
   640	enum SourceSockets {
   641	    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
   642	    /// the granted host:port.
   643	    Dial { host: String, tcp_port: u32 },
   644	    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
   645	    /// off the listener it already bound for epoch-0, credential
   646	    /// `session_token ‖ sub_token`.
   647	    Accept { listener: TcpListener },
   648	}
   649	
   650	/// A running source-side data plane: the dialed/accepted socket(s) wrapped
   651	/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
   652	/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
   653	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   654	/// END record, and returns the bytes this end sent.
   655	pub(super) struct SourceDataPlane {
   656	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   657	    control_tx: mpsc::UnboundedSender<SinkControl>,
   658	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   659	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   660	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   661	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   662	    // advertised capacity (contract §Invariants 5). The resize drives only
   663	    // its shape-correction stream count; the cheap-dial tuner is future
   664	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   665	    dial: Arc<TransferDial>,
   666	    source: Arc<dyn TransferSource>,
   667	    session_token: Vec<u8>,
   668	    pool: Arc<BufferPool>,
   669	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   670	    /// otp-10a). Applied to the epoch-0 sockets at construction and to
   671	    /// each epoch-N resize socket in [`Self::add_stream`].
   672	    trace: bool,
   673	    /// How each epoch-N resize socket is acquired (dial for the SOURCE
   674	    /// initiator, accept for the SOURCE responder). The data plane grows
   675	    /// mid-transfer in both cases; the control-lane resize choreography is
   676	    /// identical — only this transport action flips (otp-5b-2).
   677	    sockets: SourceSockets,
   678	}
   679	
   680	/// Dial the granted data plane and start the elastic send pipeline.
   681	/// `host` is the responder's host (the initiator connected the control
   682	/// plane to it; the data plane rides the same host on the granted port —
   683	/// contract §Transport: the initiator always dials). `receiver_capacity`
   684	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   685	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   686	/// never unlimited).
   687	pub(super) async fn dial_source_data_plane(
   688	    host: &str,
   689	    grant: &DataPlaneGrant,
   690	    receiver_capacity: Option<&CapacityProfile>,
   691	    source: Arc<dyn TransferSource>,
   692	    instruments: &SourceInstruments,
   693	) -> Result<SourceDataPlane> {
   694	    let initial = grant.initial_streams.max(1) as usize;
   695	    // The byte sender's dial, bounded by the receiver's advertised
   696	    // capacity. Seed the settled live count to the granted epoch-0
   697	    // streams — every shape-resize proposal steps from here.
   698	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   699	    dial.set_negotiated_streams(initial);
   700	
   701	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   702	    let mut handshake = grant.session_token.clone();
   703	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   704	
   705	    // Provision the pool for the dial ceiling so resize-added sockets
   706	    // draw buffers from the same pool without re-pooling (as old push
   707	    // does — a shared pool sized for the maximum stream count).
   708	    let pool = Arc::new(BufferPool::for_data_plane(
   709	        dial.chunk_bytes(),
   710	        dial.ceiling_max_streams().max(1),
   711	    ));
   712	    let trace = instruments.trace_data_plane;
   713	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   714	    for _ in 0..initial {
   715	        let session = DataPlaneSession::connect(
   716	            host,
   717	            grant.tcp_port,
   718	            &handshake,
   719	            dial.chunk_bytes(),
   720	            dial.prefetch_count(),
   721	            trace,
   722	            dial.tcp_buffer_bytes(),
   723	            Arc::clone(&pool),
   724	        )
   725	        .await
   726	        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
   727	        // The source-side sink never reads its dst_root (it only sends);
   728	        // `root()` is consulted by the relay/receive case, not here.
   729	        sinks.push(Arc::new(DataPlaneSink::new(
   730	            session,
   731	            Arc::clone(&source),
   732	            PathBuf::new(),
   733	        )));
   734	    }
   735	
   736	    let prefetch = dial.prefetch_count().max(1);
   737	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   738	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   739	    let pipe_source = Arc::clone(&source);
   740	    let pipe_progress = instruments.progress.clone();
   741	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   742	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   743	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   744	        execute_sink_pipeline_elastic(
   745	            pipe_source,
   746	            sinks,
   747	            payload_rx,
   748	            prefetch,
   749	            pipe_progress.as_ref(),
   750	            Some(control_rx),
   751	        )
   752	        .await
   753	    }));
   754	    Ok(SourceDataPlane {
   755	        payload_tx: Some(payload_tx),
   756	        control_tx,
   757	        pipeline: Some(pipeline),
   758	        dial,
   759	        source,
   760	        session_token: grant.session_token.clone(),

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1670,1938p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1290,1585p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "rg -n -C 12 \"DataPlaneResize|ResizeAck|ceiling|target_streams|last_resize|resize_epoch|sub_token\" crates/blit-core/src/transfer_session/mod.rs | sed -n '1,500p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1290	            None => None,
  1291	        },
  1292	    };
  1293	
  1294	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
  1295	    // fed to the shape table so the SOURCE grows the data-plane stream
  1296	    // count as the workload's shape becomes known. Append-only (a need is
  1297	    // counted once, when it arrives), and the in-flight resize record the
  1298	    // ack is matched against (at most one — the dial enforces it).
  1299	    let mut needed_bytes: u64 = 0;
  1300	    let mut needed_count: usize = 0;
  1301	    let mut pending_resize: Option<data_plane::PendingResize> = None;
  1302	
  1303	    // Streaming manifest: entries go out as enumeration produces them
  1304	    // (immediate start in every direction — plan §Design 2). The open
  1305	    // carries no source path (the source end owns its local endpoint) but
  1306	    // does carry the include/exclude/size/age filter (otp-6a): only
  1307	    // matching files are manifested and transferred. The filter MUST ride
  1308	    // the wire (not be pre-wrapped by a local caller) because for pull the
  1309	    // SOURCE is the remote daemon responder — it, not the client, owns the
  1310	    // scan. Apply it through the universal `FilteredSource` decorator, the
  1311	    // single filter chokepoint every source impl routes through, rather
  1312	    // than the per-impl `scan(filter)` arg — a source impl is free to
  1313	    // ignore that arg (the since-deleted relay source did; codex otp-6a
  1314	    // F1), and the chokepoint makes filtering independent of it. A
  1315	    // default/absent filter scans everything (unchanged from otp-3). Globs
  1316	    // were validated at OPEN (`source_open_validator`), so the conversion
  1317	    // cannot fail on a validated open; map any error to a fault regardless.
  1318	    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
  1319	        Some(spec) if *spec != FilterSpec::default() => {
  1320	            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
  1321	                .map_err(|e| {
  1322	                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
  1323	                })?;
  1324	            Arc::new(crate::remote::transfer::source::FilteredSource::new(
  1325	                Arc::clone(&source),
  1326	                filter,
  1327	            ))
  1328	        }
  1329	        _ => Arc::clone(&source),
  1330	    };
  1331	    // otp-10b-1: a Checksum session fills each manifest header's
  1332	    // checksum so the DESTINATION can skip content-equal files
  1333	    // regardless of mtime. Wrapped OUTSIDE the filter so only
  1334	    // in-scope files pay the hash; a serving end that refuses to hash
  1335	    // never gets here (CHECKSUM_DISABLED at OPEN).
  1336	    let scan_source: Arc<dyn TransferSource> =
  1337	        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
  1338	            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
  1339	                scan_source,
  1340	            ))
  1341	        } else {
  1342	            scan_source
  1343	        };
  1344	    // otp-10a: callers that must not treat a partial transfer as success
  1345	    // (the push verb, `blit move`'s source-delete gate) supply their own
  1346	    // accumulator via `SourceInstruments` and inspect it after the
  1347	    // session returns; the wire behavior is identical either way.
  1348	    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
  1349	    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
  1350	    while let Some(header) = header_rx.recv().await {
  1351	        sent.lock()
  1352	            .expect("sent-manifest lock poisoned")
  1353	            .insert(header.relative_path.clone(), header.clone());
  1354	        tx.send(frame(Frame::ManifestEntry(header))).await?;
  1355	        // Faults detected by the receive half abort the stream now,
  1356	        // not after the full scan; needs just accumulate. (Resize acks
  1357	        // cannot arrive yet — none is proposed before the payload phase.)
  1358	        drain_ready_source_events(
  1359	            &mut events,
  1360	            &mut pending,
  1361	            &mut resume,
  1362	            &mut need_complete,
  1363	            &mut needed_bytes,
  1364	            &mut needed_count,
  1365	            data_plane.as_ref(),
  1366	            tx,
  1367	            &mut pending_resize,
  1368	        )
  1369	        .await?;
  1370	    }
  1371	    let scanned = scan_handle
  1372	        .await
  1373	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
  1374	    let scan_complete = unreadable
  1375	        .lock()
  1376	        .expect("unreadable list lock poisoned")
  1377	        .is_empty();
  1378	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
  1379	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
  1380	        scan_complete,
  1381	    })))
  1382	    .await?;
  1383	    manifest_sent.store(true, Ordering::Release);
  1384	
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch. Settle the whole shape-derived target before
  1417	                    // handing payloads to the pipeline: otherwise the
  1418	                    // one-ADD-per-epoch ramp races NeedComplete/payload
  1419	                    // drain, so a fast transfer can finish at a different
  1420	                    // worker count depending on which endpoint initiated.
  1421	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1422	                        .await?;
  1423	                    settle_shape_resizes(
  1424	                        &mut events,
  1425	                        &mut pending,
  1426	                        &mut resume,
  1427	                        &mut need_complete,
  1428	                        &mut needed_bytes,
  1429	                        &mut needed_count,
  1430	                        dp,
  1431	                        tx,
  1432	                        &mut pending_resize,
  1433	                    )
  1434	                    .await?;
  1435	                    let payloads =
  1436	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1437	                    // A cancel while earlier batches are actively moving
  1438	                    // closes the send pipeline under backpressure, so this
  1439	                    // queue fails with a data-plane error — prefer the
  1440	                    // peer's framed reason (CANCELLED) the same way the
  1441	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1442	                    // against events like finish(): live `Need`s still
  1443	                    // arrive here, and `recv_peer_fault` would consume them.
  1444	                    if let Err(dp_err) = dp.queue(payloads).await {
  1445	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1446	                    }
  1447	                }
  1448	                None => {
  1449	                    // codex otp-8 F1: race the record sends against the
  1450	                    // receive half's fault signal — the in-stream twin of
  1451	                    // the data-plane drain's `recv_peer_fault` arm. A peer
  1452	                    // cancel (framed CANCELLED, then RPC teardown) must
  1453	                    // interrupt a send blocked in `reader.read()` or in
  1454	                    // flow-controlled `tx.send()` and surface the framed
  1455	                    // reason, not hang or decay to INTERNAL. Biased:
  1456	                    // when both are ready, the framed fault wins.
  1457	                    tokio::select! {
  1458	                        biased;
  1459	                        fault = peer_fault_signalled(&mut fault_signal) => {
  1460	                            return Err(eyre::Report::new(fault));
  1461	                        }
  1462	                        res = send_payload_records(
  1463	                            tx,
  1464	                            &source,
  1465	                            plan_options,
  1466	                            batch,
  1467	                            &mut read_buf,
  1468	                            instruments.progress.as_ref(),
  1469	                        ) => {
  1470	                            res?;
  1471	                        }
  1472	                    }
  1473	                }
  1474	            }
  1475	            continue;
  1476	        }
  1477	        if !resume.ready.is_empty() {
  1478	            // The block phase for correlated (need, hash-list) pairs.
  1479	            // Data plane (otp-7b): each pair becomes ONE composite
  1480	            // ResumeFile work item, so one pipeline worker runs the
  1481	            // whole record on one socket — strict per-file serialization
  1482	            // without cross-socket reorder hazards. In-stream (otp-7a):
  1483	            // control-lane BlockTransfer/Complete frames, as before.
  1484	            let ready = std::mem::take(&mut resume.ready);
  1485	            match &mut data_plane {
  1486	                Some(dp) => {
  1487	                    // codex 7b-1 F4: resume batches drive the sf-2 shape
  1488	                    // correction exactly as plain batches do — a
  1489	                    // resume-heavy need list must not stay pinned to the
  1490	                    // zero-knowledge single stream.
  1491	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1492	                        .await?;
  1493	                    settle_shape_resizes(
  1494	                        &mut events,
  1495	                        &mut pending,
  1496	                        &mut resume,
  1497	                        &mut need_complete,
  1498	                        &mut needed_bytes,
  1499	                        &mut needed_count,
  1500	                        dp,
  1501	                        tx,
  1502	                        &mut pending_resize,
  1503	                    )
  1504	                    .await?;
  1505	                    let payloads = ready
  1506	                        .into_iter()
  1507	                        .map(|(header, hashes)| TransferPayload::ResumeFile {
  1508	                            header,
  1509	                            block_size: hashes.block_size,
  1510	                            dest_hashes: hashes.hashes,
  1511	                        })
  1512	                        .collect();
  1513	                    // Same cancel posture as the plain-batch queue above:
  1514	                    // prefer the peer's framed reason over the transport
  1515	                    // break a cancel also causes (otp-4b-3 codex F1).
  1516	                    if let Err(dp_err) = dp.queue(payloads).await {
  1517	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1518	                    }
  1519	                }
  1520	                None => {
  1521	                    for (header, hashes) in ready {
  1522	                        // codex 7b-2 G2: the whole in-stream record names
  1523	                        // its file on failure, matching the data-plane
  1524	                        // carrier's outer wrap. Same fault race as the
  1525	                        // plain-batch send above (codex otp-8 F1).
  1526	                        tokio::select! {
  1527	                            biased;
  1528	                            fault = peer_fault_signalled(&mut fault_signal) => {
  1529	                                return Err(eyre::Report::new(fault));
  1530	                            }
  1531	                            res = send_resume_block_records(
  1532	                                tx,
  1533	                                &source,
  1534	                                &header,
  1535	                                &hashes,
  1536	                                instruments.progress.as_ref(),
  1537	                            ) => {
  1538	                                res.map_err(|e| tag_path(e, &header.relative_path))?;
  1539	                            }
  1540	                        }
  1541	                    }
  1542	                }
  1543	            }
  1544	            continue;
  1545	        }
  1546	        if need_complete {
  1547	            break;
  1548	        }
  1549	        match events.recv().await {
  1550	            Some(event) => {
  1551	                process_source_event(
  1552	                    event,
  1553	                    &mut pending,
  1554	                    &mut resume,
  1555	                    &mut need_complete,
  1556	                    &mut needed_bytes,
  1557	                    &mut needed_count,
  1558	                    data_plane.as_ref(),
  1559	                    tx,
  1560	                    &mut pending_resize,
  1561	                )
  1562	                .await?;
  1563	            }
  1564	            None => {
  1565	                return Err(eyre::Report::new(SessionFault::internal(
  1566	                    "source receive half ended before NeedComplete",
  1567	                )))
  1568	            }
  1569	        }
  1570	    }
  1571	
  1572	    // A resize proposed on the last batch may still be in flight. Resolve
  1573	    // it BEFORE finishing so the destination's armed slot is consumed by
  1574	    // the dialed socket — an armed-but-never-dialed credential would hang
  1575	    // its accept loop (which waits for every arm to be claimed). We do not
  1576	    // propose further here: exactly the one in-flight resize is drained.
  1577	    if let Some(dp) = &data_plane {
  1578	        if let Some(pending) = pending_resize.take() {
  1579	            resolve_in_flight_resize(&mut events, dp, pending).await?;
  1580	        }
  1581	    }
  1582	
  1583	    // Close the data plane BEFORE SourceDone so the destination's receive
  1584	    // pipeline sees each socket's END record and completes; SourceDone on
  1585	    // the control lane then lets the destination score and summarize.

 succeeded in 0ms:
  1670	        )
  1671	        .await?;
  1672	    }
  1673	    Ok(())
  1674	}
  1675	
  1676	/// Handle one source event. Needs accumulate into `pending` and the
  1677	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1678	/// next ADD (the one-per-epoch ramp).
  1679	#[allow(clippy::too_many_arguments)]
  1680	async fn process_source_event(
  1681	    event: SourceEvent,
  1682	    pending: &mut Vec<FileHeader>,
  1683	    resume: &mut ResumeSendState,
  1684	    need_complete: &mut bool,
  1685	    needed_bytes: &mut u64,
  1686	    needed_count: &mut usize,
  1687	    data_plane: Option<&data_plane::SourceDataPlane>,
  1688	    tx: &mut Box<dyn FrameTx>,
  1689	    pending_resize: &mut Option<data_plane::PendingResize>,
  1690	) -> Result<()> {
  1691	    match event {
  1692	        SourceEvent::Need(header) => {
  1693	            if *need_complete {
  1694	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1695	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1696	                )));
  1697	            }
  1698	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1699	            *needed_count += 1;
  1700	            pending.push(header);
  1701	            Ok(())
  1702	        }
  1703	        SourceEvent::ResumeNeed(header) => {
  1704	            if *need_complete {
  1705	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1706	                    format!(
  1707	                        "resume need for '{}' after NeedComplete",
  1708	                        header.relative_path
  1709	                    ),
  1710	                )));
  1711	            }
  1712	            // Shape totals count the whole file — the diff hasn't run
  1713	            // yet, so the need list's implied workload is the honest
  1714	            // upper bound (same accounting a plain need gets).
  1715	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1716	            *needed_count += 1;
  1717	            // HELD until its BlockHashList arrives; no duplicate is
  1718	            // possible (the receive half's sent-map removal already
  1719	            // faults a second need for the same path).
  1720	            resume.held.insert(header.relative_path.clone(), header);
  1721	            Ok(())
  1722	        }
  1723	        SourceEvent::BlockHashes(list) => {
  1724	            // Validate the wire block size at ARRIVAL (codex F5), not
  1725	            // when the record is eventually sent — pending plain files
  1726	            // go out first, and an already-invalid frame must fail fast.
  1727	            // A conforming destination clamps into this range (D5 /
  1728	            // D-2026-07-10-1); same-build peers make a mismatch a
  1729	            // violation, never a negotiation. The ceiling is the
  1730	            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
  1731	            // records take up to the wire block cap; in-stream frames
  1732	            // must stay under the gRPC frame limit.
  1733	            let ceiling = if data_plane.is_some() {
  1734	                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  1735	            } else {
  1736	                MAX_IN_STREAM_RESUME_BLOCK_SIZE
  1737	            };
  1738	            let bs = list.block_size as usize;
  1739	            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
  1740	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1741	                    format!(
  1742	                        "BlockHashList for '{}' block_size {bs} outside \
  1743	                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
  1744	                        list.relative_path
  1745	                    ),
  1746	                )));
  1747	            }
  1748	            match resume.held.remove(&list.relative_path) {
  1749	                Some(header) => {
  1750	                    resume.ready.push((header, list));
  1751	                    Ok(())
  1752	                }
  1753	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1754	                    format!(
  1755	                        "BlockHashList for '{}' without a held resume need",
  1756	                        list.relative_path
  1757	                    ),
  1758	                ))),
  1759	            }
  1760	        }
  1761	        SourceEvent::NeedComplete => {
  1762	            if *need_complete {
  1763	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1764	                    "duplicate NeedComplete",
  1765	                )));
  1766	            }
  1767	            // Ordered lane: the destination sends every BlockHashList
  1768	            // before its NeedComplete, so a still-held resume need here
  1769	            // means the peer broke the choreography — fail fast rather
  1770	            // than hang waiting for a list that can no longer arrive.
  1771	            if !resume.held.is_empty() {
  1772	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1773	                    format!(
  1774	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1775	                        resume.held.len()
  1776	                    ),
  1777	                )));
  1778	            }
  1779	            *need_complete = true;
  1780	            Ok(())
  1781	        }
  1782	        SourceEvent::ResizeAck(ack) => {
  1783	            let dp = data_plane.ok_or_else(|| {
  1784	                eyre::Report::new(SessionFault::protocol_violation(
  1785	                    "DataPlaneResizeAck on a session with no data plane",
  1786	                ))
  1787	            })?;
  1788	            // Match the ack to the in-flight proposal; stale/unsolicited
  1789	            // acks (wrong epoch, or none pending) are ignored, matching
  1790	            // old push. `take()` + restore keeps the borrow simple.
  1791	            let pending_r = match pending_resize.take() {
  1792	                Some(p) if p.epoch == ack.epoch => p,
  1793	                restored => {
  1794	                    *pending_resize = restored;
  1795	                    return Ok(());
  1796	                }
  1797	            };
  1798	            if ack.accepted {
  1799	                dp.add_stream(&pending_r.sub_token).await?;
  1800	                dp.dial()
  1801	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1802	                // Ramp one stream per accepted epoch: propose the next ADD.
  1803	                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1804	            } else {
  1805	                dp.dial()
  1806	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1807	                // A refusal is terminal for this shape ramp. Retrying the
  1808	                // same unattainable target under a fresh epoch would loop
  1809	                // forever; the settled live set still carries the transfer.
  1810	                Ok(())
  1811	            }
  1812	        }
  1813	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1814	            "TransferSummary before SourceDone",
  1815	        ))),
  1816	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1817	    }
  1818	}
  1819	
  1820	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1821	/// the stream count the accumulated need list implies, if none is in
  1822	/// flight. A no-op when the shape wants no more than the live count (the
  1823	/// dial returns `None`). Sends the frame and records the in-flight
  1824	/// proposal for the ack to match.
  1825	async fn maybe_propose_resize(
  1826	    dp: &data_plane::SourceDataPlane,
  1827	    tx: &mut Box<dyn FrameTx>,
  1828	    needed_bytes: u64,
  1829	    needed_count: usize,
  1830	    pending_resize: &mut Option<data_plane::PendingResize>,
  1831	) -> Result<()> {
  1832	    if pending_resize.is_some() {
  1833	        return Ok(());
  1834	    }
  1835	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1836	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1837	            op: DataPlaneResizeOp::Add as i32,
  1838	            epoch: proposal.epoch,
  1839	            target_stream_count: proposal.target_streams,
  1840	            sub_token: proposal.sub_token.clone(),
  1841	        })))
  1842	        .await?;
  1843	        *pending_resize = Some(proposal);
  1844	    }
  1845	    Ok(())
  1846	}
  1847	
  1848	/// Drive the one-stream-per-epoch shape ramp to its currently known target
  1849	/// before payload dispatch. Needs and resume hashes may continue arriving
  1850	/// while an ack is in flight, so process the shared SOURCE event lane rather
  1851	/// than waiting for only an ack. Each accepted ack proposes the next epoch
  1852	/// from the latest accumulated shape; the loop ends only when no proposal is
  1853	/// outstanding (target reached or the destination refused growth).
  1854	#[allow(clippy::too_many_arguments)]
  1855	async fn settle_shape_resizes(
  1856	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1857	    pending: &mut Vec<FileHeader>,
  1858	    resume: &mut ResumeSendState,
  1859	    need_complete: &mut bool,
  1860	    needed_bytes: &mut u64,
  1861	    needed_count: &mut usize,
  1862	    data_plane: &data_plane::SourceDataPlane,
  1863	    tx: &mut Box<dyn FrameTx>,
  1864	    pending_resize: &mut Option<data_plane::PendingResize>,
  1865	) -> Result<()> {
  1866	    while pending_resize.is_some() {
  1867	        let event = events.recv().await.ok_or_else(|| {
  1868	            eyre::Report::new(SessionFault::internal(
  1869	                "source receive half ended during data-plane shape resize",
  1870	            ))
  1871	        })?;
  1872	        process_source_event(
  1873	            event,
  1874	            pending,
  1875	            resume,
  1876	            need_complete,
  1877	            needed_bytes,
  1878	            needed_count,
  1879	            Some(data_plane),
  1880	            tx,
  1881	            pending_resize,
  1882	        )
  1883	        .await?;
  1884	    }
  1885	    Ok(())
  1886	}
  1887	
  1888	/// Block for the ack of the one in-flight resize and dial its socket (or
  1889	/// settle it refused). Does NOT propose further — it resolves exactly the
  1890	/// pending proposal so the destination's armed slot is consumed before we
  1891	/// finish the data plane.
  1892	async fn resolve_in_flight_resize(
  1893	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1894	    dp: &data_plane::SourceDataPlane,
  1895	    pending: data_plane::PendingResize,
  1896	) -> Result<()> {
  1897	    loop {
  1898	        match events.recv().await {
  1899	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1900	                if ack.accepted {
  1901	                    dp.add_stream(&pending.sub_token).await?;
  1902	                    dp.dial()
  1903	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1904	                } else {
  1905	                    dp.dial()
  1906	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1907	                }
  1908	                return Ok(());
  1909	            }
  1910	            // A stale ack for an already-settled epoch: ignore, keep
  1911	            // waiting for ours.
  1912	            Some(SourceEvent::ResizeAck(_)) => continue,
  1913	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1914	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1915	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1916	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1917	                )))
  1918	            }
  1919	            Some(SourceEvent::BlockHashes(l)) => {
  1920	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1921	                    format!(
  1922	                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
  1923	                        l.relative_path
  1924	                    ),
  1925	                )))
  1926	            }
  1927	            Some(SourceEvent::NeedComplete) => {
  1928	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1929	                    "duplicate NeedComplete",
  1930	                )))
  1931	            }
  1932	            Some(SourceEvent::Summary(_)) => {
  1933	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1934	                    "TransferSummary before SourceDone",
  1935	                )))
  1936	            }
  1937	            None => {
  1938	                return Err(eyre::Report::new(SessionFault::internal(

 succeeded in 0ms:
29-use std::pin::Pin;
30-use std::sync::atomic::{AtomicBool, Ordering};
31-use std::sync::{Arc, Mutex as StdMutex};
32-
33-use eyre::Result;
34-use tokio::io::{AsyncReadExt, AsyncWriteExt};
35-use tokio::sync::{mpsc, watch};
36-
37-use crate::copy::DEFAULT_BLOCK_SIZE;
38-use crate::generated::transfer_frame::Frame;
39-use crate::generated::{
40-    session_error, BlockHashList, BlockTransfer, BlockTransferComplete, ComparisonMode,
41:    DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, FileData, FileHeader, FilterSpec,
42-    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept, SessionError,
43-    SessionHello, SessionOpen, SourceDone, TarShardComplete, TarShardHeader, TransferFrame,
44-    TransferRole, TransferSummary,
45-};
46-use crate::manifest::{header_transfer_status, CompareMode, CompareOptions, FileStatus};
47-use crate::remote::transfer::diff_planner;
48-use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
49-use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
50-use crate::remote::transfer::source::{FsTransferSource, TransferSource};
51-use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
52-use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
53-use crate::remote::transfer::{
--
72-
73-/// Manifest entries buffered per destination diff batch. Mirrors the
74-/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
75-/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
76-/// blocking pool instead of inline per entry.
77-const DEST_DIFF_CHUNK: usize = 128;
78-
79-/// Buffer of the in-memory pipe that feeds wire file-record bytes
80-/// into `FsTransferSink::write_file_stream`. Bounds destination-side
81-/// buffering per file record.
82-const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
83-
84:/// otp-7a resume bounds (codex F1, D-2026-07-10-1; data-plane ceiling
85-/// otp-7b, D-2026-07-10-2). The in-stream carrier rides the gRPC
86-/// `Transfer` RPC when the daemon serves, and tonic's default 4 MiB
87-/// decode limit applies to every frame — so the DESTINATION's
88-/// block-size clamp (plan D5) must keep both resume frame shapes under
89-/// it. The TCP data plane carries blocks as binary records with no
90:/// protobuf envelope, so its ceiling is the wire record bound instead.
91:/// The ceiling is therefore PER CARRIER; both ends know the carrier
92-/// (grant present ⇒ data plane), so they agree without negotiation.
93-///
94-/// Floor: a `BlockHashList` costs 32 bytes per block, so absurdly small
95-/// blocks amplify — a block_size=1 list would be 32× the partial.
96-const MIN_RESUME_BLOCK_SIZE: usize = 64 * 1024;
97-/// Ceiling, in-stream carrier: one `BlockTransfer` frame carries one
98-/// whole block; 2 MiB of content plus the envelope stays well under the
99-/// 4 MiB frame limit.
100-const MAX_IN_STREAM_RESUME_BLOCK_SIZE: usize = 2 * 1024 * 1024;
101-/// Ceiling, in-stream carrier, for one `TarShardHeader` frame's encoded
102-/// member list (codex otp-8 F2). The planner bounds a shard's CONTENT
103-/// bytes and file count (≤ 4096), but not the encoded size of its
104-/// header list — 4096 legally long relative paths can push the single
105-/// protobuf frame past tonic's 4 MiB decode limit. The in-stream send
106-/// path splits an offending shard into consecutive smaller shard
107-/// records under this bound (same grammar, same planner decisions —
108-/// only the record boundaries move). Same 2 MiB posture as the resume
109:/// block ceiling: content plus envelope stays well under the frame
110-/// limit. The data plane is unaffected (binary records, 64 MiB cap).
111-const MAX_IN_STREAM_TAR_HEADER_BYTES: usize = 2 * 1024 * 1024;
112-/// Ceiling, TCP data plane (otp-7b): binary `BLOCK` records have no
113-/// protobuf envelope; the bound is the receive pipeline's per-record
114-/// allocation cap (= the old resume path's `MAX_BLOCK_SIZE`, 64 MiB).
115-/// The hash list still rides the control lane as protobuf, but its
116-/// size is governed by the 65_536-hash cap, not by block size.
117-const MAX_DATA_PLANE_RESUME_BLOCK_SIZE: usize =
118-    crate::remote::transfer::pipeline::MAX_WIRE_BLOCK_BYTES;
119-/// One `BlockHashList` frame carries a partial's whole list; capped at
120-/// 65_536 × 32 B = 2 MiB of hashes. A partial with more blocks than
121-/// this degrades to the empty list — the contract's full-transfer
--
451-        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
452-        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
453-        Some(Frame::NeedBatch(_)) => "NeedBatch",
454-        Some(Frame::NeedComplete(_)) => "NeedComplete",
455-        Some(Frame::BlockHashes(_)) => "BlockHashList",
456-        Some(Frame::FileBegin(_)) => "FileBegin",
457-        Some(Frame::FileData(_)) => "FileData",
458-        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
459-        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
460-        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
461-        Some(Frame::Block(_)) => "BlockTransfer",
462-        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
463:        Some(Frame::Resize(_)) => "DataPlaneResize",
464:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
465-        Some(Frame::SourceDone(_)) => "SourceDone",
466-        Some(Frame::Summary(_)) => "TransferSummary",
467-        Some(Frame::Error(_)) => "SessionError",
468-        None => "empty frame",
469-    }
470-}
471-
472-fn complement(role: TransferRole) -> TransferRole {
473-    match role {
474-        TransferRole::Source => TransferRole::Destination,
475-        TransferRole::Destination => TransferRole::Source,
476-        TransferRole::Unspecified => TransferRole::Unspecified,
--
917-/// fault the session), so the queue never exceeds the source's own
918-/// manifest size — the contract's bounded-buffering rule holds.
919-enum SourceEvent {
920-    Need(FileHeader),
921-    /// A resume-flagged need (otp-7a). The send half HOLDS it until the
922-    /// destination's `BlockHashList` for the same path arrives — the
923-    /// contract's RELIABLE ordering guarantee: no byte of a resume file
924-    /// moves before its hash list.
925-    ResumeNeed(FileHeader),
926-    /// The destination's block hashes for a held resume need (otp-7a).
927-    BlockHashes(BlockHashList),
928-    NeedComplete,
929:    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
930-    /// send half dials the epoch-N socket on `accepted`.
931:    ResizeAck(DataPlaneResizeAck),
932-    Summary(TransferSummary),
933-    Fault(SessionFault),
934-}
935-
936-/// The receive half's event sender, mirroring every `Fault` onto a
937-/// `watch` signal as it is queued. The in-stream send path races this
938-/// signal against its (potentially blocked) record sends — codex otp-8
939-/// F1: a peer fault (CANCELLED above all) must interrupt a send half
940-/// stuck inside `reader.read()`/`tx.send()`, exactly as the data-plane
941-/// drain's `recv_peer_fault` arm does for socket sends. The mpsc queue
942-/// still carries the fault for the between-send paths; the watch is a
943-/// non-consuming side channel, so mid-send `Need`s stay queued.
--
1183-                if !manifest_sent.load(Ordering::Acquire) {
1184-                    // Fail fast at arrival time (otp-3 codex F2): the
1185-                    // event queue would otherwise let an early
1186-                    // NeedComplete be processed late and pass as
1187-                    // legitimate.
1188-                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
1189-                        "NeedComplete before the source's ManifestComplete",
1190-                    )));
1191-                    return;
1192-                }
1193-                let _ = events.send(SourceEvent::NeedComplete);
1194-            }
1195:            Some(Frame::ResizeAck(ack)) => {
1196-                // The destination's response to a shape-resize proposal
1197-                // (otp-4b-2). Forward it to the send half, which owns the
1198-                // dial and dials the epoch-N socket on `accepted`.
1199:                let _ = events.send(SourceEvent::ResizeAck(ack));
1200-            }
1201-            Some(Frame::Summary(summary)) => {
1202-                let _ = events.send(SourceEvent::Summary(summary));
1203-                return;
1204-            }
1205-            Some(Frame::Error(err)) => {
1206-                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
1207-                return;
1208-            }
1209-            other => {
1210-                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
1211-                    format!("{} on the source's receive lane", frame_name(&other)),
--
1623-            Err(eyre::Report::new(SessionFault::protocol_violation(
1624-                format!("need for '{}' after NeedComplete", h.relative_path),
1625-            )))
1626-        }
1627-        Some(SourceEvent::BlockHashes(l)) => {
1628-            Err(eyre::Report::new(SessionFault::protocol_violation(
1629-                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
1630-            )))
1631-        }
1632-        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
1633-            SessionFault::protocol_violation("duplicate NeedComplete"),
1634-        )),
1635:        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
1636:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
1637-        )),
1638-        None => Err(eyre::Report::new(SessionFault::internal(
1639-            "source receive half ended before TransferSummary",
1640-        ))),
1641-    }
1642-}
1643-
1644-/// Process every event ready right now (needs accumulating, resize acks
1645-/// dialing their epoch-N socket) without blocking. Called between
1646-/// manifest sends and at the top of the payload loop.
1647-#[allow(clippy::too_many_arguments)]
1648-async fn drain_ready_source_events(
--
1717-            // HELD until its BlockHashList arrives; no duplicate is
1718-            // possible (the receive half's sent-map removal already
1719-            // faults a second need for the same path).
1720-            resume.held.insert(header.relative_path.clone(), header);
1721-            Ok(())
1722-        }
1723-        SourceEvent::BlockHashes(list) => {
1724-            // Validate the wire block size at ARRIVAL (codex F5), not
1725-            // when the record is eventually sent — pending plain files
1726-            // go out first, and an already-invalid frame must fail fast.
1727-            // A conforming destination clamps into this range (D5 /
1728-            // D-2026-07-10-1); same-build peers make a mismatch a
1729:            // violation, never a negotiation. The ceiling is the
1730-            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
1731-            // records take up to the wire block cap; in-stream frames
1732-            // must stay under the gRPC frame limit.
1733:            let ceiling = if data_plane.is_some() {
1734-                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
1735-            } else {
1736-                MAX_IN_STREAM_RESUME_BLOCK_SIZE
1737-            };
1738-            let bs = list.block_size as usize;
1739:            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
1740-                return Err(eyre::Report::new(SessionFault::protocol_violation(
1741-                    format!(
1742-                        "BlockHashList for '{}' block_size {bs} outside \
1743:                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
1744-                        list.relative_path
1745-                    ),
1746-                )));
1747-            }
1748-            match resume.held.remove(&list.relative_path) {
1749-                Some(header) => {
1750-                    resume.ready.push((header, list));
1751-                    Ok(())
1752-                }
1753-                None => Err(eyre::Report::new(SessionFault::protocol_violation(
1754-                    format!(
1755-                        "BlockHashList for '{}' without a held resume need",
--
1770-            // than hang waiting for a list that can no longer arrive.
1771-            if !resume.held.is_empty() {
1772-                return Err(eyre::Report::new(SessionFault::protocol_violation(
1773-                    format!(
1774-                        "NeedComplete with {} resume need(s) missing their BlockHashList",
1775-                        resume.held.len()
1776-                    ),
1777-                )));
1778-            }
1779-            *need_complete = true;
1780-            Ok(())
1781-        }
1782:        SourceEvent::ResizeAck(ack) => {
1783-            let dp = data_plane.ok_or_else(|| {
1784-                eyre::Report::new(SessionFault::protocol_violation(
1785:                    "DataPlaneResizeAck on a session with no data plane",
1786-                ))
1787-            })?;
1788-            // Match the ack to the in-flight proposal; stale/unsolicited
1789-            // acks (wrong epoch, or none pending) are ignored, matching
1790-            // old push. `take()` + restore keeps the borrow simple.
1791-            let pending_r = match pending_resize.take() {
1792-                Some(p) if p.epoch == ack.epoch => p,
1793-                restored => {
1794-                    *pending_resize = restored;
1795-                    return Ok(());
1796-                }
1797-            };
1798-            if ack.accepted {
1799:                dp.add_stream(&pending_r.sub_token).await?;
1800-                dp.dial()
1801:                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
1802-                // Ramp one stream per accepted epoch: propose the next ADD.
1803-                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
1804-            } else {
1805-                dp.dial()
1806-                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
1807-                // A refusal is terminal for this shape ramp. Retrying the
1808-                // same unattainable target under a fresh epoch would loop
1809-                // forever; the settled live set still carries the transfer.
1810-                Ok(())
1811-            }
1812-        }
1813-        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
1814-            "TransferSummary before SourceDone",
1815-        ))),
1816-        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
1817-    }
1818-}
1819-
1820:/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
1821-/// the stream count the accumulated need list implies, if none is in
1822-/// flight. A no-op when the shape wants no more than the live count (the
1823-/// dial returns `None`). Sends the frame and records the in-flight
1824-/// proposal for the ack to match.
1825-async fn maybe_propose_resize(
1826-    dp: &data_plane::SourceDataPlane,
1827-    tx: &mut Box<dyn FrameTx>,
1828-    needed_bytes: u64,
1829-    needed_count: usize,
1830-    pending_resize: &mut Option<data_plane::PendingResize>,
1831-) -> Result<()> {
1832-    if pending_resize.is_some() {
1833-        return Ok(());
1834-    }
1835-    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
1836:        tx.send(frame(Frame::Resize(DataPlaneResize {
1837:            op: DataPlaneResizeOp::Add as i32,
1838-            epoch: proposal.epoch,
1839:            target_stream_count: proposal.target_streams,
1840:            sub_token: proposal.sub_token.clone(),
1841-        })))
1842-        .await?;
1843-        *pending_resize = Some(proposal);
1844-    }
1845-    Ok(())
1846-}
1847-
1848-/// Drive the one-stream-per-epoch shape ramp to its currently known target
1849-/// before payload dispatch. Needs and resume hashes may continue arriving
1850-/// while an ack is in flight, so process the shared SOURCE event lane rather
1851-/// than waiting for only an ack. Each accepted ack proposes the next epoch
1852-/// from the latest accumulated shape; the loop ends only when no proposal is
--
1887-
1888-/// Block for the ack of the one in-flight resize and dial its socket (or
1889-/// settle it refused). Does NOT propose further — it resolves exactly the
1890-/// pending proposal so the destination's armed slot is consumed before we
1891-/// finish the data plane.
1892-async fn resolve_in_flight_resize(
1893-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
1894-    dp: &data_plane::SourceDataPlane,
1895-    pending: data_plane::PendingResize,
1896-) -> Result<()> {
1897-    loop {
1898-        match events.recv().await {
1899:            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
1900-                if ack.accepted {
1901:                    dp.add_stream(&pending.sub_token).await?;
1902-                    dp.dial()
1903:                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
1904-                } else {
1905-                    dp.dial()
1906-                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
1907-                }
1908-                return Ok(());
1909-            }
1910-            // A stale ack for an already-settled epoch: ignore, keep
1911-            // waiting for ours.
1912:            Some(SourceEvent::ResizeAck(_)) => continue,
1913-            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
1914-            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
1915-                return Err(eyre::Report::new(SessionFault::protocol_violation(
1916-                    format!("need for '{}' after NeedComplete", h.relative_path),
1917-                )))
1918-            }
1919-            Some(SourceEvent::BlockHashes(l)) => {
1920-                return Err(eyre::Report::new(SessionFault::protocol_violation(
1921-                    format!(
1922-                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
1923-                        l.relative_path
1924-                    ),
--
1941-            }
1942-        }
1943-    }
1944-}
1945-
1946-/// Await the next terminal signal the receive half forwards while the
1947-/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
1948-/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
1949-/// abort the send and surface as the fault.
1950-///
1951-/// The drain runs after `resolve_in_flight_resize` and before `SourceDone`
1952-/// goes out, so the event channel is drained and the peer sends nothing
1953:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
1954-/// or `Summary` is legitimate here. So a `Fault` is returned as-is and any
1955-/// OTHER event is surfaced as a protocol violation rather than silently
1956-/// dropped (codex otp-4b-3 F3): dropping it would defer or lose a
1957-/// fail-fast error and, if the drain is itself stuck, hang. Parks forever
1958-/// once the channel closes with no event so the data-plane future it
1959-/// races decides the outcome instead.
1960-async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
1961-    match events.recv().await {
1962-        Some(SourceEvent::Fault(fault)) => fault,
1963-        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
1964-            SessionFault::protocol_violation(format!(
1965-                "need for '{}' during the data-plane drain (after NeedComplete)",
1966-                h.relative_path
1967-            ))
1968-        }
1969-        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
1970-            "BlockHashList for '{}' during the data-plane drain",
1971-            l.relative_path
1972-        )),
1973-        Some(SourceEvent::NeedComplete) => {
1974-            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
1975-        }
1976:        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
1977:            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
1978-        ),
1979-        Some(SourceEvent::Summary(_)) => {
1980-            SessionFault::protocol_violation("TransferSummary before SourceDone")
1981-        }
1982-        None => std::future::pending().await,
1983-    }
1984-}
1985-
1986-/// A data-plane operation (`queue`/`finish`) failed mid-transfer. The
1987-/// break is usually the *symptom* of a peer abort — within
1988-/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
1989-/// its receive workers) always frames the real reason on the control
1990-/// lane. Prefer that framed fault; fall back to the raw data-plane error
1991-/// if the channel closes first or none arrives in that window.
1992-///
1993-/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
1994-/// fast on any stray event), this is called from BOTH error sites,
1995-/// including the `queue()` error inside the payload loop — where a
1996:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
1997-/// ahead of the peer's `SessionError` (codex otp-4b-3 pass-2 F1). So it
1998-/// SKIPS non-fault events rather than treating them as violations: we are
1999-/// already unwinding on a data-plane error, and the framed fault (or the
2000-/// dp error) is the correct outcome, never a spurious protocol violation.
2001-async fn prefer_peer_fault(
2002-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
2003-    dp_err: eyre::Report,
2004-) -> eyre::Report {
2005-    let framed = async {
2006-        loop {
2007-            match events.recv().await {
2008-                Some(SourceEvent::Fault(fault)) => break Some(fault),
--
2711-            crate::fs_enum::FileFilter::default()
2712-        };
2713-    let mut source_files: HashSet<String> = HashSet::new();
2714-
2715-    // otp-7a: resume. Headers of resume-granted needs are retained so a
2716-    // record's completion can finalize with the manifest's
2717-    // size/mtime/permissions and be validated against the grant. Both
2718-    // the header map and the resumed counter are SHARED with the
2719-    // data-plane receive (otp-7b) exactly as `outstanding` is: on the
2720-    // data plane the control loop never sees block records, so the
2721-    // NeedListSink claims resume grants and counts completions as they
2722-    // land on the sockets. The block size is chosen below, once the
2723:    // carrier is known (the ceiling is per carrier).
2724-    let resume_enabled = resume_negotiated(&negotiated.open);
2725-    let resume_headers: data_plane::ResumeHeaders = Arc::default();
2726-    let files_resumed = Arc::new(std::sync::atomic::AtomicU64::new(0));
2727-
2728-    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
2729-    // `granted` is the ever-granted DEDUP set — control-loop-local,
2730-    // insert-only, never removed, so a concurrent data-plane claim can
2731-    // never re-open a grant (a duplicate manifest path is granted at
2732-    // most once regardless of delivery timing). `outstanding` is the
2733-    // not-yet-delivered COMPLETION set — inserted for each freshly
2734-    // granted path before its NeedBatch, claimed by both carriers (the
2735-    // in-stream arms inline, the data-plane NeedListSink as payloads
--
2742-    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
2743-    // bytes arrive on sockets (not the control lane). Set it up NOW —
2744-    // concurrent with the diff loop below, and before the peer sends — so
2745-    // the connections are established promptly. Which end connects depends
2746-    // on connection role (otp-5b): a DESTINATION **responder** (push)
2747-    // accepts sockets off its bound listener; a DESTINATION **initiator**
2748-    // (pull) dials the grant it received on `data_plane_host`. Byte
2749-    // direction is the same either way (DESTINATION receives). The
2750-    // NeedListSink gives the socket receive the same need-list strictness
2751-    // the in-stream control loop applies inline; AbortOnDrop (inside the
2752-    // responder run) bounds the accept task to this future. `resize_live`
2753-    // tracks the stream count this end has grown to (epoch-0 plus each
2754:    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
2755-    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
2756-    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
2757-    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
2758-        Arc::clone(&sink),
2759-        Arc::clone(&outstanding),
2760-        // otp-7b: only a resume session accepts block records on the
2761-        // data plane; the sink validates + claims them against the same
2762-        // shared grant state the in-stream arms use.
2763-        resume_enabled.then(|| data_plane::ResumeRecv {
2764-            headers: Arc::clone(&resume_headers),
2765-            resumed: Arc::clone(&files_resumed),
2766-        }),
2767-    ));
2768:    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
2769-        match negotiated.responder_data_plane {
2770-            // DESTINATION responder (push, otp-4b): accept + receive.
2771-            Some(rdp) => {
2772-                let initial = rdp.initial_streams() as usize;
2773-                let run = rdp.spawn(recv_sink, progress.clone());
2774:                let ceiling = run.ceiling;
2775-                (
2776-                    Some(data_plane::DestRecvPlane::Responder(run)),
2777-                    initial,
2778:                    ceiling,
2779-                )
2780-            }
2781-            // DESTINATION initiator (pull, otp-5b): dial + receive when the
2782-            // SOURCE responder granted a data plane and we have a host to dial.
2783-            None => match (&negotiated.accept.data_plane, data_plane_host) {
2784-                (Some(grant), Some(host)) => {
2785-                    let initial = grant.initial_streams.max(1) as usize;
2786-                    let run = data_plane::dial_destination_data_plane(
2787-                        host,
2788-                        grant,
2789-                        recv_sink,
2790-                        progress.clone(),
2791-                        instruments.trace_data_plane,
2792-                    )
2793-                    .await?;
2794-                    // otp-5b-2: the pull data plane resizes too. Seed

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '620,930p'; nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '930,1120p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   620	// ---------------------------------------------------------------------------
   621	// Initiator (SOURCE) — dial, authenticate, send, resize
   622	// ---------------------------------------------------------------------------
   623	
   624	/// A resize the SOURCE has proposed and minted a credential for but not
   625	/// yet completed: the driver has sent (or will send) the matching
   626	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   627	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   628	/// flight (the dial's `pending_epoch` enforces it; this is the
   629	/// driver-side record the ack is matched against).
   630	pub(super) struct PendingResize {
   631	    pub(super) epoch: u32,
   632	    pub(super) target_streams: u32,
   633	    pub(super) sub_token: Vec<u8>,
   634	}
   635	
   636	/// How the SOURCE acquires each epoch-N data socket for a shape resize —
   637	/// the two connection roles of otp-5b. Byte direction is identical (the
   638	/// SOURCE sends), and `propose_resize` is the same either way; only socket
   639	/// acquisition flips.
   640	enum SourceSockets {
   641	    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
   642	    /// the granted host:port.
   643	    Dial { host: String, tcp_port: u32 },
   644	    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
   645	    /// off the listener it already bound for epoch-0, credential
   646	    /// `session_token ‖ sub_token`.
   647	    Accept { listener: TcpListener },
   648	}
   649	
   650	/// A running source-side data plane: the dialed/accepted socket(s) wrapped
   651	/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
   652	/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
   653	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   654	/// END record, and returns the bytes this end sent.
   655	pub(super) struct SourceDataPlane {
   656	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   657	    control_tx: mpsc::UnboundedSender<SinkControl>,
   658	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   659	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   660	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   661	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   662	    // advertised capacity (contract §Invariants 5). The resize drives only
   663	    // its shape-correction stream count; the cheap-dial tuner is future
   664	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   665	    dial: Arc<TransferDial>,
   666	    source: Arc<dyn TransferSource>,
   667	    session_token: Vec<u8>,
   668	    pool: Arc<BufferPool>,
   669	    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
   670	    /// otp-10a). Applied to the epoch-0 sockets at construction and to
   671	    /// each epoch-N resize socket in [`Self::add_stream`].
   672	    trace: bool,
   673	    /// How each epoch-N resize socket is acquired (dial for the SOURCE
   674	    /// initiator, accept for the SOURCE responder). The data plane grows
   675	    /// mid-transfer in both cases; the control-lane resize choreography is
   676	    /// identical — only this transport action flips (otp-5b-2).
   677	    sockets: SourceSockets,
   678	}
   679	
   680	/// Dial the granted data plane and start the elastic send pipeline.
   681	/// `host` is the responder's host (the initiator connected the control
   682	/// plane to it; the data plane rides the same host on the granted port —
   683	/// contract §Transport: the initiator always dials). `receiver_capacity`
   684	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   685	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   686	/// never unlimited).
   687	pub(super) async fn dial_source_data_plane(
   688	    host: &str,
   689	    grant: &DataPlaneGrant,
   690	    receiver_capacity: Option<&CapacityProfile>,
   691	    source: Arc<dyn TransferSource>,
   692	    instruments: &SourceInstruments,
   693	) -> Result<SourceDataPlane> {
   694	    let initial = grant.initial_streams.max(1) as usize;
   695	    // The byte sender's dial, bounded by the receiver's advertised
   696	    // capacity. Seed the settled live count to the granted epoch-0
   697	    // streams — every shape-resize proposal steps from here.
   698	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   699	    dial.set_negotiated_streams(initial);
   700	
   701	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   702	    let mut handshake = grant.session_token.clone();
   703	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   704	
   705	    // Provision the pool for the dial ceiling so resize-added sockets
   706	    // draw buffers from the same pool without re-pooling (as old push
   707	    // does — a shared pool sized for the maximum stream count).
   708	    let pool = Arc::new(BufferPool::for_data_plane(
   709	        dial.chunk_bytes(),
   710	        dial.ceiling_max_streams().max(1),
   711	    ));
   712	    let trace = instruments.trace_data_plane;
   713	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   714	    for _ in 0..initial {
   715	        let session = DataPlaneSession::connect(
   716	            host,
   717	            grant.tcp_port,
   718	            &handshake,
   719	            dial.chunk_bytes(),
   720	            dial.prefetch_count(),
   721	            trace,
   722	            dial.tcp_buffer_bytes(),
   723	            Arc::clone(&pool),
   724	        )
   725	        .await
   726	        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
   727	        // The source-side sink never reads its dst_root (it only sends);
   728	        // `root()` is consulted by the relay/receive case, not here.
   729	        sinks.push(Arc::new(DataPlaneSink::new(
   730	            session,
   731	            Arc::clone(&source),
   732	            PathBuf::new(),
   733	        )));
   734	    }
   735	
   736	    let prefetch = dial.prefetch_count().max(1);
   737	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   738	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   739	    let pipe_source = Arc::clone(&source);
   740	    let pipe_progress = instruments.progress.clone();
   741	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   742	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   743	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   744	        execute_sink_pipeline_elastic(
   745	            pipe_source,
   746	            sinks,
   747	            payload_rx,
   748	            prefetch,
   749	            pipe_progress.as_ref(),
   750	            Some(control_rx),
   751	        )
   752	        .await
   753	    }));
   754	    Ok(SourceDataPlane {
   755	        payload_tx: Some(payload_tx),
   756	        control_tx,
   757	        pipeline: Some(pipeline),
   758	        dial,
   759	        source,
   760	        session_token: grant.session_token.clone(),
   761	        pool,
   762	        trace,
   763	        // SOURCE initiator: each epoch-N resize socket is dialed to the
   764	        // granted host:port.
   765	        sockets: SourceSockets::Dial {
   766	            host: host.to_string(),
   767	            tcp_port: grant.tcp_port,
   768	        },
   769	    })
   770	}
   771	
   772	/// Accept the granted epoch-0 socket(s) off a bound responder listener and
   773	/// start the elastic SEND pipeline over them — the SOURCE **responder**
   774	/// half of the pull data plane (otp-5b-1). Symmetric with
   775	/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
   776	/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
   777	/// socket acquisition differs (accept here, dial there).
   778	/// `DataPlaneSession::from_stream` builds a send session from an already-
   779	/// accepted socket — the same primitive the old `pull_sync` daemon-send
   780	/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
   781	/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
   782	/// wherever it initiates). The bound listener is retained so each epoch-N
   783	/// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
   784	/// dials, this end accepts, the control-lane frames identical to push.
   785	pub(super) async fn accept_source_data_plane(
   786	    bound: ResponderDataPlane,
   787	    receiver_capacity: Option<&CapacityProfile>,
   788	    source: Arc<dyn TransferSource>,
   789	    instruments: &SourceInstruments,
   790	) -> Result<SourceDataPlane> {
   791	    let initial = bound.initial_streams.max(1) as usize;
   792	    // The byte sender's dial, bounded by the receiver's advertised
   793	    // capacity; seed the live count to the granted epoch-0 streams. Growth
   794	    // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
   795	    // here, one stream per epoch, same as the SOURCE initiator.
   796	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   797	    dial.set_negotiated_streams(initial);
   798	
   799	    // Epoch-0 credential the dialing DESTINATION presents:
   800	    // session_token ‖ epoch0_sub_token (contract §Transport).
   801	    let mut epoch0 = bound.session_token.clone();
   802	    epoch0.extend_from_slice(&bound.epoch0_sub_token);
   803	
   804	    let pool = Arc::new(BufferPool::for_data_plane(
   805	        dial.chunk_bytes(),
   806	        dial.ceiling_max_streams().max(1),
   807	    ));
   808	    let trace = instruments.trace_data_plane;
   809	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   810	    for _ in 0..initial {
   811	        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
   812	        let session = DataPlaneSession::from_stream(
   813	            socket,
   814	            trace,
   815	            dial.chunk_bytes(),
   816	            dial.prefetch_count(),
   817	            Arc::clone(&pool),
   818	        )
   819	        .await;
   820	        sinks.push(Arc::new(DataPlaneSink::new(
   821	            session,
   822	            Arc::clone(&source),
   823	            PathBuf::new(),
   824	        )));
   825	    }
   826	
   827	    let prefetch = dial.prefetch_count().max(1);
   828	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   829	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   830	    let pipe_source = Arc::clone(&source);
   831	    let pipe_progress = instruments.progress.clone();
   832	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   833	        execute_sink_pipeline_elastic(
   834	            pipe_source,
   835	            sinks,
   836	            payload_rx,
   837	            prefetch,
   838	            pipe_progress.as_ref(),
   839	            Some(control_rx),
   840	        )
   841	        .await
   842	    }));
   843	    Ok(SourceDataPlane {
   844	        payload_tx: Some(payload_tx),
   845	        control_tx,
   846	        pipeline: Some(pipeline),
   847	        dial,
   848	        source,
   849	        session_token: bound.session_token,
   850	        pool,
   851	        trace,
   852	        // SOURCE responder: each epoch-N resize socket is accepted off the
   853	        // same listener epoch-0 came in on (otp-5b-2).
   854	        sockets: SourceSockets::Accept {
   855	            listener: bound.listener,
   856	        },
   857	    })
   858	}
   859	
   860	impl SourceDataPlane {
   861	    /// The live dial (the byte sender owns it). The driver reads
   862	    /// `live_streams()` for observability and calls `resize_settled` as
   863	    /// each proposal completes.
   864	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   865	        &self.dial
   866	    }
   867	
   868	    /// sf-2 shape correction: propose one ADD toward the stream count the
   869	    /// accumulated need list implies, if none is in flight and the shape
   870	    /// wants more than the current live count. Mints the resize
   871	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   872	    /// the record back on the matching ack.
   873	    pub(super) fn propose_resize(
   874	        &self,
   875	        needed_bytes: u64,
   876	        needed_count: usize,
   877	    ) -> Result<Option<PendingResize>> {
   878	        let desired =
   879	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   880	                as usize;
   881	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   882	            return Ok(None);
   883	        };
   884	        let sub_token = generate_sub_token()
   885	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   886	        Ok(Some(PendingResize {
   887	            epoch: proposal.epoch,
   888	            target_streams: proposal.target_streams as u32,
   889	            sub_token,
   890	        }))
   891	    }
   892	
   893	    /// Acquire the epoch-N data socket for an accepted resize and hand it
   894	    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
   895	    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
   896	    /// socket the DESTINATION initiator dials after its ack, off the same
   897	    /// listener epoch-0 came in on. A dial/accept failure is FATAL
   898	    /// (fail-fast): a same-build peer that established epoch-0 failing an
   899	    /// epoch-N socket is a transport fault worth surfacing — and faulting
   900	    /// the session aborts the peer's counterpart via AbortOnDrop, so no
   901	    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
   902	    /// session trades that for simplicity — noted in the finding doc.) If
   903	    /// the pipeline is already gone (transfer completing under the ADD),
   904	    /// the just-acquired socket is closed cleanly so the peer's worker sees
   905	    /// its END, not a reset.
   906	    ///
   907	    /// The accept is bounded and unambiguous: at most one resize is in
   908	    /// flight (the driver's `pending_resize`) and epoch-0 is already
   909	    /// accepted, so the next connection off the listener is exactly this
   910	    /// resize's socket — verified against `session_token ‖ sub_token`.
   911	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   912	        let session = match &self.sockets {
   913	            SourceSockets::Dial { host, tcp_port } => {
   914	                let mut handshake = self.session_token.clone();
   915	                handshake.extend_from_slice(sub_token);
   916	                DataPlaneSession::connect(
   917	                    host,
   918	                    *tcp_port,
   919	                    &handshake,
   920	                    self.dial.chunk_bytes(),
   921	                    self.dial.prefetch_count(),
   922	                    self.trace,
   923	                    self.dial.tcp_buffer_bytes(),
   924	                    Arc::clone(&self.pool),
   925	                )
   926	                .await
   927	                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?
   928	            }
   929	            SourceSockets::Accept { listener } => {
   930	                let mut expected = self.session_token.clone();
   930	                let mut expected = self.session_token.clone();
   931	                expected.extend_from_slice(sub_token);
   932	                let socket = accept_authenticated(listener, &expected).await?;
   933	                DataPlaneSession::from_stream(
   934	                    socket,
   935	                    self.trace,
   936	                    self.dial.chunk_bytes(),
   937	                    self.dial.prefetch_count(),
   938	                    Arc::clone(&self.pool),
   939	                )
   940	                .await
   941	            }
   942	        };
   943	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   944	            session,
   945	            Arc::clone(&self.source),
   946	            PathBuf::new(),
   947	        ));
   948	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   949	            if let SinkControl::Add(sink) = returned.0 {
   950	                let _ = sink.finish().await;
   951	            }
   952	        }
   953	        Ok(())
   954	    }
   955	
   956	    /// Feed one planned batch into the send pipeline. The pipeline
   957	    /// prepares each payload (tar-shard/file) and writes it through the
   958	    /// data-plane record framing across the live socket(s).
   959	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   960	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   961	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   962	        })?;
   963	        for payload in payloads {
   964	            tx.send(payload).await.map_err(|_| {
   965	                dp_fault("data-plane send pipeline closed before all payloads sent")
   966	            })?;
   967	        }
   968	        Ok(())
   969	    }
   970	
   971	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   972	    /// socket's END record on drain), and return the bytes sent. Must be
   973	    /// awaited before `SourceDone` goes out so the destination's receive
   974	    /// pipeline sees END and completes.
   975	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   976	        // Drop the sender: workers observe the closed queue, drain what
   977	        // is left, then `finish()` (END record) and exit.
   978	        self.payload_tx = None;
   979	        let pipeline = self
   980	            .pipeline
   981	            .take()
   982	            .expect("SourceDataPlane::finish called once");
   983	        pipeline
   984	            .join()
   985	            .await
   986	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   987	    }
   988	}
   989	
   990	// ---------------------------------------------------------------------------
   991	// Need-list enforcement for the data-plane receive
   992	// ---------------------------------------------------------------------------
   993	
   994	/// Sink decorator that enforces the session's need-list contract on the
   995	/// data-plane receive, giving it the SAME strictness the in-stream
   996	/// carrier applies inline in the control loop (`outstanding.remove`).
   997	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   998	/// without this a peer could substitute an off-need-list path for a
   999	/// needed one (count-preserving), duplicate one, or send resume block
  1000	/// records the session never negotiated (codex otp-4b-1 F1). Every
  1001	/// written path must be a granted, not-yet-received need. Resume
  1002	/// sessions (otp-7b) additionally validate + claim block records
  1003	/// against the shared [`ResumeHeaders`] grant map — with the identical
  1004	/// strictness the in-stream `claim_resume_record` applies — and count
  1005	/// completions into the shared resumed counter; in a non-resume session
  1006	/// block records are rejected outright. The shared [`OutstandingNeeds`]
  1007	/// set makes completion `is_empty()` for both carriers.
  1008	pub(super) struct NeedListSink {
  1009	    inner: Arc<dyn TransferSink>,
  1010	    outstanding: OutstandingNeeds,
  1011	    /// `Some` iff the session negotiated resume (otp-7b): the shared
  1012	    /// grant map + resumed counter block records are validated and
  1013	    /// claimed against. `None` ⇒ any block record is a violation.
  1014	    resume: Option<ResumeRecv>,
  1015	}
  1016	
  1017	impl NeedListSink {
  1018	    pub(super) fn new(
  1019	        inner: Arc<dyn TransferSink>,
  1020	        outstanding: OutstandingNeeds,
  1021	        resume: Option<ResumeRecv>,
  1022	    ) -> Self {
  1023	        Self {
  1024	            inner,
  1025	            outstanding,
  1026	            resume,
  1027	        }
  1028	    }
  1029	
  1030	    /// Remove `path` from the outstanding set, or fault: a path that is
  1031	    /// not present is either off the need list or a duplicate delivery.
  1032	    fn claim(&self, path: &str) -> Result<()> {
  1033	        if self
  1034	            .outstanding
  1035	            .lock()
  1036	            .expect("outstanding-needs lock poisoned")
  1037	            .remove(path)
  1038	        {
  1039	            Ok(())
  1040	        } else {
  1041	            Err(eyre::Report::new(
  1042	                SessionFault::protocol_violation(format!(
  1043	                    "data-plane payload for '{path}' which is not an outstanding need \
  1044	                 (off the need list, or a duplicate delivery)"
  1045	                ))
  1046	                .with_path(path),
  1047	            ))
  1048	        }
  1049	    }
  1050	
  1051	    /// codex otp-7a F3, data-plane parity: a resume-flagged grant may
  1052	    /// be satisfied ONLY by its block record — a whole-file or tar-shard
  1053	    /// delivery for it bypasses the hash choreography this end committed
  1054	    /// to.
  1055	    fn reject_resume_flagged(&self, path: &str) -> Result<()> {
  1056	        if let Some(resume) = &self.resume {
  1057	            if resume
  1058	                .headers
  1059	                .lock()
  1060	                .expect("resume-headers lock poisoned")
  1061	                .contains_key(path)
  1062	            {
  1063	                return Err(eyre::Report::new(
  1064	                    SessionFault::protocol_violation(format!(
  1065	                        "data-plane file payload for resume-flagged '{path}' — the \
  1066	                         contract requires its block record"
  1067	                    ))
  1068	                    .with_path(path),
  1069	                ));
  1070	            }
  1071	        }
  1072	        Ok(())
  1073	    }
  1074	
  1075	    /// otp-7b: validate one mid-record `FileBlock` against its grant —
  1076	    /// the path must hold a live resume grant, still be an outstanding
  1077	    /// need (its completion has not claimed it), and the block must stay
  1078	    /// inside the manifested size. The grant is NOT claimed here;
  1079	    /// [`Self::claim_block_complete`] does that exactly once.
  1080	    fn check_block(&self, path: &str, offset: u64, len: u64) -> Result<()> {
  1081	        let Some(resume) = &self.resume else {
  1082	            return Err(eyre::Report::new(SessionFault::protocol_violation(
  1083	                "resume block record on the data plane of a non-resume session",
  1084	            )));
  1085	        };
  1086	        let size = {
  1087	            let held = resume.headers.lock().expect("resume-headers lock poisoned");
  1088	            match held.get(path) {
  1089	                Some(header) => header.size,
  1090	                None => {
  1091	                    return Err(eyre::Report::new(
  1092	                        SessionFault::protocol_violation(format!(
  1093	                            "data-plane block record for '{path}' which was not granted \
  1094	                             a resume-flagged need"
  1095	                        ))
  1096	                        .with_path(path),
  1097	                    ))
  1098	                }
  1099	            }
  1100	        };
  1101	        if !self
  1102	            .outstanding
  1103	            .lock()
  1104	            .expect("outstanding-needs lock poisoned")
  1105	            .contains(path)
  1106	        {
  1107	            return Err(eyre::Report::new(
  1108	                SessionFault::protocol_violation(format!(
  1109	                    "data-plane block record for '{path}' which is not an outstanding need"
  1110	                ))
  1111	                .with_path(path),
  1112	            ));
  1113	        }
  1114	        if offset.saturating_add(len) > size {
  1115	            return Err(eyre::Report::new(
  1116	                SessionFault::protocol_violation(format!(
  1117	                    "block record '{path}' overran its size: offset {offset} + {len} \
  1118	                     byte(s) > {size}"
  1119	                ))
  1120	                .with_path(path),

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2780,3035p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  2780	            }
  2781	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2782	            // SOURCE responder granted a data plane and we have a host to dial.
  2783	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2784	                (Some(grant), Some(host)) => {
  2785	                    let initial = grant.initial_streams.max(1) as usize;
  2786	                    let run = data_plane::dial_destination_data_plane(
  2787	                        host,
  2788	                        grant,
  2789	                        recv_sink,
  2790	                        progress.clone(),
  2791	                        instruments.trace_data_plane,
  2792	                    )
  2793	                    .await?;
  2794	                    // otp-5b-2: the pull data plane resizes too. Seed
  2795	                    // `resize_live` from the epoch-0 streams dialed and bound
  2796	                    // growth by the capacity THIS end advertised in its open
  2797	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2798	                    // responder's dial already clamps to, so both ends agree
  2799	                    // even when the caller advertised a max_streams below this
  2800	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2801	                    // Resize frame the initiator dials the epoch-N socket (vs
  2802	                    // the responder path's arm).
  2803	                    let ceiling = crate::dial::receiver_stream_ceiling(
  2804	                        negotiated.open.receiver_capacity.as_ref(),
  2805	                    );
  2806	                    (
  2807	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2808	                        initial,
  2809	                        ceiling,
  2810	                    )
  2811	                }
  2812	                // A grant with no host to dial is an inconsistent initiator
  2813	                // config: fail fast, mirroring the SOURCE initiator
  2814	                // (`source_send_half`). The SOURCE responder has already bound
  2815	                // and blocks accepting the socket this end would dial, so
  2816	                // silently taking the in-stream branch cannot fall back — it
  2817	                // would deadlock until the responder's accept times out. A
  2818	                // grant means the initiator MUST dial (contract §Transport).
  2819	                // (codex otp-5b-1 finding.)
  2820	                (Some(_), None) => {
  2821	                    return Err(eyre::Report::new(SessionFault::internal(
  2822	                        "responder granted a TCP data plane but this DESTINATION \
  2823	                     initiator has no host to dial",
  2824	                    )))
  2825	                }
  2826	                // No grant (the responder could not bind, or the initiator
  2827	                // asked for in-stream): the in-stream carrier.
  2828	                (None, _) => (None, 0usize, 0usize),
  2829	            },
  2830	        };
  2831	
  2832	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  2833	    // — it hashes first; the SOURCE reads the size from each
  2834	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  2835	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
  2836	    // here, after the carrier is settled.
  2837	    let resume_block_size = {
  2838	        let ceiling = if data_plane_recv.is_some() {
  2839	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2840	        } else {
  2841	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2842	        };
  2843	        match negotiated
  2844	            .open
  2845	            .resume
  2846	            .as_ref()
  2847	            .map(|r| r.block_size as usize)
  2848	            .unwrap_or(0)
  2849	        {
  2850	            0 => DEFAULT_BLOCK_SIZE,
  2851	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2852	        }
  2853	    };
  2854	
  2855	    let mut pending: Vec<FileHeader> = Vec::new();
  2856	    let mut needed_paths: Vec<String> = Vec::new();
  2857	    let mut manifest_complete = false;
  2858	    let mut files_written: u64 = 0;
  2859	    let mut bytes_written: u64 = 0;
  2860	
  2861	    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
  2862	    // loop so applies run concurrent with the diff, exactly as the
  2863	    // data-plane receive does.
  2864	    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));
  2865	
  2866	    loop {
  2867	        let received = match transport.recv().await? {
  2868	            Some(f) => f,
  2869	            None => {
  2870	                return Err(eyre::Report::new(SessionFault::internal(
  2871	                    "peer closed mid-session",
  2872	                )))
  2873	            }
  2874	        };
  2875	        match received.frame {
  2876	            Some(Frame::ManifestEntry(header)) => {
  2877	                if manifest_complete {
  2878	                    return Err(violation(format!(
  2879	                        "manifest entry '{}' after ManifestComplete",
  2880	                        header.relative_path
  2881	                    )));
  2882	                }
  2883	                // otp-6b: retain the full source path set for the mirror
  2884	                // diff (the need list keeps only files needing transfer).
  2885	                if mirror_enabled {
  2886	                    source_files.insert(header.relative_path.clone());
  2887	                }
  2888	                pending.push(header);
  2889	                if pending.len() >= DEST_DIFF_CHUNK {
  2890	                    let chunk = std::mem::take(&mut pending);
  2891	                    if let Some(la) = &local_apply {
  2892	                        diff_chunk_and_apply_local(
  2893	                            la,
  2894	                            &mut local_run,
  2895	                            chunk,
  2896	                            dst_root,
  2897	                            canonical_dst_root.as_deref(),
  2898	                            &compare_opts,
  2899	                            &mut granted,
  2900	                            &mut needed_paths,
  2901	                            progress.as_ref(),
  2902	                        )
  2903	                        .await?;
  2904	                    } else {
  2905	                        diff_chunk_and_send_needs(
  2906	                            transport,
  2907	                            chunk,
  2908	                            dst_root,
  2909	                            canonical_dst_root.as_deref(),
  2910	                            &compare_opts,
  2911	                            resume_enabled,
  2912	                            resume_block_size,
  2913	                            &resume_headers,
  2914	                            &mut granted,
  2915	                            &outstanding,
  2916	                            &mut needed_paths,
  2917	                            progress.as_ref(),
  2918	                        )
  2919	                        .await?;
  2920	                    }
  2921	                }
  2922	            }
  2923	            Some(Frame::ManifestComplete(complete)) => {
  2924	                if manifest_complete {
  2925	                    return Err(violation("duplicate ManifestComplete".into()));
  2926	                }
  2927	                // otp-6b: mirror deletions are data-loss-dangerous when the
  2928	                // source scan was incomplete — a source file missing from an
  2929	                // aborted scan would be misclassified extraneous and deleted
  2930	                // at the dest. Refuse here (before any transfer or deletion)
  2931	                // rather than partial-mirror. Matches the old paths'
  2932	                // require-complete-scan guard.
  2933	                if mirror_enabled && !complete.scan_complete {
  2934	                    return Err(eyre::Report::new(SessionFault::internal(
  2935	                        "mirror refused: the source scan did not complete \
  2936	                         (unreadable paths) — deleting now could remove files \
  2937	                         the source still has",
  2938	                    )));
  2939	                }
  2940	                // codex otp-9b F1 (R49-F2 on the session): an initiator
  2941	                // that declared "the source will be deleted after this
  2942	                // transfer" (`blit move`) must NOT get a success out of
  2943	                // an incomplete source scan — files the scan could not
  2944	                // read would be silently lost when the caller deletes
  2945	                // the source. Same abort point as the mirror guard.
  2946	                if negotiated.open.require_complete_scan && !complete.scan_complete {
  2947	                    return Err(eyre::Report::new(SessionFault::refusal(
  2948	                        session_error::Code::ScanIncomplete,
  2949	                        "transfer refused: the source scan did not complete \
  2950	                         (unreadable paths) and the operation requires a \
  2951	                         complete scan (move deletes the source afterwards)",
  2952	                    )));
  2953	                }
  2954	                let chunk = std::mem::take(&mut pending);
  2955	                if let Some(la) = &local_apply {
  2956	                    diff_chunk_and_apply_local(
  2957	                        la,
  2958	                        &mut local_run,
  2959	                        chunk,
  2960	                        dst_root,
  2961	                        canonical_dst_root.as_deref(),
  2962	                        &compare_opts,
  2963	                        &mut granted,
  2964	                        &mut needed_paths,
  2965	                        progress.as_ref(),
  2966	                    )
  2967	                    .await?;
  2968	                } else {
  2969	                    diff_chunk_and_send_needs(
  2970	                        transport,
  2971	                        chunk,
  2972	                        dst_root,
  2973	                        canonical_dst_root.as_deref(),
  2974	                        &compare_opts,
  2975	                        resume_enabled,
  2976	                        resume_block_size,
  2977	                        &resume_headers,
  2978	                        &mut granted,
  2979	                        &outstanding,
  2980	                        &mut needed_paths,
  2981	                        progress.as_ref(),
  2982	                    )
  2983	                    .await?;
  2984	                }
  2985	                // NeedComplete only after ManifestComplete received
  2986	                // AND every entry diffed — both true here.
  2987	                transport
  2988	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  2989	                    .await?;
  2990	                manifest_complete = true;
  2991	            }
  2992	            Some(Frame::FileBegin(header)) => {
  2993	                // Payload records ride the control lane only under the
  2994	                // in-stream carrier; with a TCP data plane active they
  2995	                // flow over the sockets, so one here is a violation.
  2996	                if data_plane_recv.is_some() {
  2997	                    return Err(violation(format!(
  2998	                        "file record '{}' on the control lane while a TCP data plane is active",
  2999	                        header.relative_path
  3000	                    )));
  3001	                }
  3002	                if !manifest_complete {
  3003	                    return Err(violation(format!(
  3004	                        "payload record for '{}' before ManifestComplete",
  3005	                        header.relative_path
  3006	                    )));
  3007	                }
  3008	                // A resume-flagged grant may be satisfied ONLY by its
  3009	                // block record — a whole-file record for it bypasses the
  3010	                // hash choreography this end committed to (codex F3).
  3011	                if resume_headers
  3012	                    .lock()
  3013	                    .expect("resume-headers lock poisoned")
  3014	                    .contains_key(&header.relative_path)
  3015	                {
  3016	                    return Err(violation(format!(
  3017	                        "file record for resume-flagged '{}' — the contract requires \
  3018	                         its block record",
  3019	                        header.relative_path
  3020	                    )));
  3021	                }
  3022	                if !outstanding
  3023	                    .lock()
  3024	                    .expect("outstanding-needs lock poisoned")
  3025	                    .remove(&header.relative_path)
  3026	                {
  3027	                    return Err(violation(format!(
  3028	                        "payload for '{}' which is not on the need list",
  3029	                        header.relative_path
  3030	                    )));
  3031	                }
  3032	                let outcome = receive_file_record(transport, sink.as_ref(), &header).await?;
  3033	                files_written += outcome.files_written as u64;
  3034	                bytes_written += outcome.bytes_written;
  3035	                // otp-10b-2: in-stream per-file progress, same convention

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1750,1905p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '2780,3025p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1750	                    resume.ready.push((header, list));
  1751	                    Ok(())
  1752	                }
  1753	                None => Err(eyre::Report::new(SessionFault::protocol_violation(
  1754	                    format!(
  1755	                        "BlockHashList for '{}' without a held resume need",
  1756	                        list.relative_path
  1757	                    ),
  1758	                ))),
  1759	            }
  1760	        }
  1761	        SourceEvent::NeedComplete => {
  1762	            if *need_complete {
  1763	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1764	                    "duplicate NeedComplete",
  1765	                )));
  1766	            }
  1767	            // Ordered lane: the destination sends every BlockHashList
  1768	            // before its NeedComplete, so a still-held resume need here
  1769	            // means the peer broke the choreography — fail fast rather
  1770	            // than hang waiting for a list that can no longer arrive.
  1771	            if !resume.held.is_empty() {
  1772	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1773	                    format!(
  1774	                        "NeedComplete with {} resume need(s) missing their BlockHashList",
  1775	                        resume.held.len()
  1776	                    ),
  1777	                )));
  1778	            }
  1779	            *need_complete = true;
  1780	            Ok(())
  1781	        }
  1782	        SourceEvent::ResizeAck(ack) => {
  1783	            let dp = data_plane.ok_or_else(|| {
  1784	                eyre::Report::new(SessionFault::protocol_violation(
  1785	                    "DataPlaneResizeAck on a session with no data plane",
  1786	                ))
  1787	            })?;
  1788	            // Match the ack to the in-flight proposal; stale/unsolicited
  1789	            // acks (wrong epoch, or none pending) are ignored, matching
  1790	            // old push. `take()` + restore keeps the borrow simple.
  1791	            let pending_r = match pending_resize.take() {
  1792	                Some(p) if p.epoch == ack.epoch => p,
  1793	                restored => {
  1794	                    *pending_resize = restored;
  1795	                    return Ok(());
  1796	                }
  1797	            };
  1798	            if ack.accepted {
  1799	                dp.add_stream(&pending_r.sub_token).await?;
  1800	                dp.dial()
  1801	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1802	                // Ramp one stream per accepted epoch: propose the next ADD.
  1803	                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1804	            } else {
  1805	                dp.dial()
  1806	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1807	                // A refusal is terminal for this shape ramp. Retrying the
  1808	                // same unattainable target under a fresh epoch would loop
  1809	                // forever; the settled live set still carries the transfer.
  1810	                Ok(())
  1811	            }
  1812	        }
  1813	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1814	            "TransferSummary before SourceDone",
  1815	        ))),
  1816	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1817	    }
  1818	}
  1819	
  1820	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1821	/// the stream count the accumulated need list implies, if none is in
  1822	/// flight. A no-op when the shape wants no more than the live count (the
  1823	/// dial returns `None`). Sends the frame and records the in-flight
  1824	/// proposal for the ack to match.
  1825	async fn maybe_propose_resize(
  1826	    dp: &data_plane::SourceDataPlane,
  1827	    tx: &mut Box<dyn FrameTx>,
  1828	    needed_bytes: u64,
  1829	    needed_count: usize,
  1830	    pending_resize: &mut Option<data_plane::PendingResize>,
  1831	) -> Result<()> {
  1832	    if pending_resize.is_some() {
  1833	        return Ok(());
  1834	    }
  1835	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1836	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1837	            op: DataPlaneResizeOp::Add as i32,
  1838	            epoch: proposal.epoch,
  1839	            target_stream_count: proposal.target_streams,
  1840	            sub_token: proposal.sub_token.clone(),
  1841	        })))
  1842	        .await?;
  1843	        *pending_resize = Some(proposal);
  1844	    }
  1845	    Ok(())
  1846	}
  1847	
  1848	/// Drive the one-stream-per-epoch shape ramp to its currently known target
  1849	/// before payload dispatch. Needs and resume hashes may continue arriving
  1850	/// while an ack is in flight, so process the shared SOURCE event lane rather
  1851	/// than waiting for only an ack. Each accepted ack proposes the next epoch
  1852	/// from the latest accumulated shape; the loop ends only when no proposal is
  1853	/// outstanding (target reached or the destination refused growth).
  1854	#[allow(clippy::too_many_arguments)]
  1855	async fn settle_shape_resizes(
  1856	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1857	    pending: &mut Vec<FileHeader>,
  1858	    resume: &mut ResumeSendState,
  1859	    need_complete: &mut bool,
  1860	    needed_bytes: &mut u64,
  1861	    needed_count: &mut usize,
  1862	    data_plane: &data_plane::SourceDataPlane,
  1863	    tx: &mut Box<dyn FrameTx>,
  1864	    pending_resize: &mut Option<data_plane::PendingResize>,
  1865	) -> Result<()> {
  1866	    while pending_resize.is_some() {
  1867	        let event = events.recv().await.ok_or_else(|| {
  1868	            eyre::Report::new(SessionFault::internal(
  1869	                "source receive half ended during data-plane shape resize",
  1870	            ))
  1871	        })?;
  1872	        process_source_event(
  1873	            event,
  1874	            pending,
  1875	            resume,
  1876	            need_complete,
  1877	            needed_bytes,
  1878	            needed_count,
  1879	            Some(data_plane),
  1880	            tx,
  1881	            pending_resize,
  1882	        )
  1883	        .await?;
  1884	    }
  1885	    Ok(())
  1886	}
  1887	
  1888	/// Block for the ack of the one in-flight resize and dial its socket (or
  1889	/// settle it refused). Does NOT propose further — it resolves exactly the
  1890	/// pending proposal so the destination's armed slot is consumed before we
  1891	/// finish the data plane.
  1892	async fn resolve_in_flight_resize(
  1893	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1894	    dp: &data_plane::SourceDataPlane,
  1895	    pending: data_plane::PendingResize,
  1896	) -> Result<()> {
  1897	    loop {
  1898	        match events.recv().await {
  1899	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1900	                if ack.accepted {
  1901	                    dp.add_stream(&pending.sub_token).await?;
  1902	                    dp.dial()
  1903	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1904	                } else {
  1905	                    dp.dial()
  2780	            }
  2781	            // DESTINATION initiator (pull, otp-5b): dial + receive when the
  2782	            // SOURCE responder granted a data plane and we have a host to dial.
  2783	            None => match (&negotiated.accept.data_plane, data_plane_host) {
  2784	                (Some(grant), Some(host)) => {
  2785	                    let initial = grant.initial_streams.max(1) as usize;
  2786	                    let run = data_plane::dial_destination_data_plane(
  2787	                        host,
  2788	                        grant,
  2789	                        recv_sink,
  2790	                        progress.clone(),
  2791	                        instruments.trace_data_plane,
  2792	                    )
  2793	                    .await?;
  2794	                    // otp-5b-2: the pull data plane resizes too. Seed
  2795	                    // `resize_live` from the epoch-0 streams dialed and bound
  2796	                    // growth by the capacity THIS end advertised in its open
  2797	                    // (it is the byte receiver) — the exact ceiling the SOURCE
  2798	                    // responder's dial already clamps to, so both ends agree
  2799	                    // even when the caller advertised a max_streams below this
  2800	                    // host's fresh local reading (codex otp-5b-2 F1). On a
  2801	                    // Resize frame the initiator dials the epoch-N socket (vs
  2802	                    // the responder path's arm).
  2803	                    let ceiling = crate::dial::receiver_stream_ceiling(
  2804	                        negotiated.open.receiver_capacity.as_ref(),
  2805	                    );
  2806	                    (
  2807	                        Some(data_plane::DestRecvPlane::Initiator(run)),
  2808	                        initial,
  2809	                        ceiling,
  2810	                    )
  2811	                }
  2812	                // A grant with no host to dial is an inconsistent initiator
  2813	                // config: fail fast, mirroring the SOURCE initiator
  2814	                // (`source_send_half`). The SOURCE responder has already bound
  2815	                // and blocks accepting the socket this end would dial, so
  2816	                // silently taking the in-stream branch cannot fall back — it
  2817	                // would deadlock until the responder's accept times out. A
  2818	                // grant means the initiator MUST dial (contract §Transport).
  2819	                // (codex otp-5b-1 finding.)
  2820	                (Some(_), None) => {
  2821	                    return Err(eyre::Report::new(SessionFault::internal(
  2822	                        "responder granted a TCP data plane but this DESTINATION \
  2823	                     initiator has no host to dial",
  2824	                    )))
  2825	                }
  2826	                // No grant (the responder could not bind, or the initiator
  2827	                // asked for in-stream): the in-stream carrier.
  2828	                (None, _) => (None, 0usize, 0usize),
  2829	            },
  2830	        };
  2831	
  2832	    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
  2833	    // — it hashes first; the SOURCE reads the size from each
  2834	    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
  2835	    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
  2836	    // here, after the carrier is settled.
  2837	    let resume_block_size = {
  2838	        let ceiling = if data_plane_recv.is_some() {
  2839	            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
  2840	        } else {
  2841	            MAX_IN_STREAM_RESUME_BLOCK_SIZE
  2842	        };
  2843	        match negotiated
  2844	            .open
  2845	            .resume
  2846	            .as_ref()
  2847	            .map(|r| r.block_size as usize)
  2848	            .unwrap_or(0)
  2849	        {
  2850	            0 => DEFAULT_BLOCK_SIZE,
  2851	            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
  2852	        }
  2853	    };
  2854	
  2855	    let mut pending: Vec<FileHeader> = Vec::new();
  2856	    let mut needed_paths: Vec<String> = Vec::new();
  2857	    let mut manifest_complete = false;
  2858	    let mut files_written: u64 = 0;
  2859	    let mut bytes_written: u64 = 0;
  2860	
  2861	    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
  2862	    // loop so applies run concurrent with the diff, exactly as the
  2863	    // data-plane receive does.
  2864	    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));
  2865	
  2866	    loop {
  2867	        let received = match transport.recv().await? {
  2868	            Some(f) => f,
  2869	            None => {
  2870	                return Err(eyre::Report::new(SessionFault::internal(
  2871	                    "peer closed mid-session",
  2872	                )))
  2873	            }
  2874	        };
  2875	        match received.frame {
  2876	            Some(Frame::ManifestEntry(header)) => {
  2877	                if manifest_complete {
  2878	                    return Err(violation(format!(
  2879	                        "manifest entry '{}' after ManifestComplete",
  2880	                        header.relative_path
  2881	                    )));
  2882	                }
  2883	                // otp-6b: retain the full source path set for the mirror
  2884	                // diff (the need list keeps only files needing transfer).
  2885	                if mirror_enabled {
  2886	                    source_files.insert(header.relative_path.clone());
  2887	                }
  2888	                pending.push(header);
  2889	                if pending.len() >= DEST_DIFF_CHUNK {
  2890	                    let chunk = std::mem::take(&mut pending);
  2891	                    if let Some(la) = &local_apply {
  2892	                        diff_chunk_and_apply_local(
  2893	                            la,
  2894	                            &mut local_run,
  2895	                            chunk,
  2896	                            dst_root,
  2897	                            canonical_dst_root.as_deref(),
  2898	                            &compare_opts,
  2899	                            &mut granted,
  2900	                            &mut needed_paths,
  2901	                            progress.as_ref(),
  2902	                        )
  2903	                        .await?;
  2904	                    } else {
  2905	                        diff_chunk_and_send_needs(
  2906	                            transport,
  2907	                            chunk,
  2908	                            dst_root,
  2909	                            canonical_dst_root.as_deref(),
  2910	                            &compare_opts,
  2911	                            resume_enabled,
  2912	                            resume_block_size,
  2913	                            &resume_headers,
  2914	                            &mut granted,
  2915	                            &outstanding,
  2916	                            &mut needed_paths,
  2917	                            progress.as_ref(),
  2918	                        )
  2919	                        .await?;
  2920	                    }
  2921	                }
  2922	            }
  2923	            Some(Frame::ManifestComplete(complete)) => {
  2924	                if manifest_complete {
  2925	                    return Err(violation("duplicate ManifestComplete".into()));
  2926	                }
  2927	                // otp-6b: mirror deletions are data-loss-dangerous when the
  2928	                // source scan was incomplete — a source file missing from an
  2929	                // aborted scan would be misclassified extraneous and deleted
  2930	                // at the dest. Refuse here (before any transfer or deletion)
  2931	                // rather than partial-mirror. Matches the old paths'
  2932	                // require-complete-scan guard.
  2933	                if mirror_enabled && !complete.scan_complete {
  2934	                    return Err(eyre::Report::new(SessionFault::internal(
  2935	                        "mirror refused: the source scan did not complete \
  2936	                         (unreadable paths) — deleting now could remove files \
  2937	                         the source still has",
  2938	                    )));
  2939	                }
  2940	                // codex otp-9b F1 (R49-F2 on the session): an initiator
  2941	                // that declared "the source will be deleted after this
  2942	                // transfer" (`blit move`) must NOT get a success out of
  2943	                // an incomplete source scan — files the scan could not
  2944	                // read would be silently lost when the caller deletes
  2945	                // the source. Same abort point as the mirror guard.
  2946	                if negotiated.open.require_complete_scan && !complete.scan_complete {
  2947	                    return Err(eyre::Report::new(SessionFault::refusal(
  2948	                        session_error::Code::ScanIncomplete,
  2949	                        "transfer refused: the source scan did not complete \
  2950	                         (unreadable paths) and the operation requires a \
  2951	                         complete scan (move deletes the source afterwards)",
  2952	                    )));
  2953	                }
  2954	                let chunk = std::mem::take(&mut pending);
  2955	                if let Some(la) = &local_apply {
  2956	                    diff_chunk_and_apply_local(
  2957	                        la,
  2958	                        &mut local_run,
  2959	                        chunk,
  2960	                        dst_root,
  2961	                        canonical_dst_root.as_deref(),
  2962	                        &compare_opts,
  2963	                        &mut granted,
  2964	                        &mut needed_paths,
  2965	                        progress.as_ref(),
  2966	                    )
  2967	                    .await?;
  2968	                } else {
  2969	                    diff_chunk_and_send_needs(
  2970	                        transport,
  2971	                        chunk,
  2972	                        dst_root,
  2973	                        canonical_dst_root.as_deref(),
  2974	                        &compare_opts,
  2975	                        resume_enabled,
  2976	                        resume_block_size,
  2977	                        &resume_headers,
  2978	                        &mut granted,
  2979	                        &outstanding,
  2980	                        &mut needed_paths,
  2981	                        progress.as_ref(),
  2982	                    )
  2983	                    .await?;
  2984	                }
  2985	                // NeedComplete only after ManifestComplete received
  2986	                // AND every entry diffed — both true here.
  2987	                transport
  2988	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  2989	                    .await?;
  2990	                manifest_complete = true;
  2991	            }
  2992	            Some(Frame::FileBegin(header)) => {
  2993	                // Payload records ride the control lane only under the
  2994	                // in-stream carrier; with a TCP data plane active they
  2995	                // flow over the sockets, so one here is a violation.
  2996	                if data_plane_recv.is_some() {
  2997	                    return Err(violation(format!(
  2998	                        "file record '{}' on the control lane while a TCP data plane is active",
  2999	                        header.relative_path
  3000	                    )));
  3001	                }
  3002	                if !manifest_complete {
  3003	                    return Err(violation(format!(
  3004	                        "payload record for '{}' before ManifestComplete",
  3005	                        header.relative_path
  3006	                    )));
  3007	                }
  3008	                // A resume-flagged grant may be satisfied ONLY by its
  3009	                // block record — a whole-file record for it bypasses the
  3010	                // hash choreography this end committed to (codex F3).
  3011	                if resume_headers
  3012	                    .lock()
  3013	                    .expect("resume-headers lock poisoned")
  3014	                    .contains_key(&header.relative_path)
  3015	                {
  3016	                    return Err(violation(format!(
  3017	                        "file record for resume-flagged '{}' — the contract requires \
  3018	                         its block record",
  3019	                        header.relative_path
  3020	                    )));
  3021	                }
  3022	                if !outstanding
  3023	                    .lock()
  3024	                    .expect("outstanding-needs lock poisoned")
  3025	                    .remove(&header.relative_path)

exec
/bin/zsh -lc "rg -n \"Some\\(Frame::Resize|Frame::ResizeAck\" crates/blit-core/src/transfer_session/mod.rs; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '3300,3475p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
463:        Some(Frame::Resize(_)) => "DataPlaneResize",
464:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
1195:            Some(Frame::ResizeAck(ack)) => {
3146:            Some(Frame::Resize(resize)) => {
3208:                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  3300	                    let canonical = canonical_dst_root.clone();
  3301	                    let files = std::mem::take(&mut source_files);
  3302	                    let filter = mirror_filter.clone_without_cache();
  3303	                    let tolerate_nonempty = mirror_kind == MirrorMode::FilteredSubset;
  3304	                    // otp-11: `--dry-run` (local carrier only) plans the
  3305	                    // pass without deleting; every wire session executes.
  3306	                    let execute = local_apply.as_ref().is_none_or(|la| !la.dry_run);
  3307	                    // codex otp-9b F2: if THIS future is dropped while the
  3308	                    // blocking pass runs (client disconnect, CancelJob),
  3309	                    // the guard's Drop flips the abort flag and the pass
  3310	                    // stops deleting instead of running to completion
  3311	                    // behind a cancelled job. (A completed await drops the
  3312	                    // guard too — harmless, the task is already done.)
  3313	                    let abort = Arc::new(AtomicBool::new(false));
  3314	                    let _abort_guard = AbortFlagOnDrop(Arc::clone(&abort));
  3315	                    let mut pass = tokio::task::spawn_blocking(move || {
  3316	                        mirror_delete_pass(
  3317	                            &dst,
  3318	                            &files,
  3319	                            &filter,
  3320	                            tolerate_nonempty,
  3321	                            canonical.as_deref(),
  3322	                            &abort,
  3323	                            execute,
  3324	                        )
  3325	                    });
  3326	                    // codex otp-10b-2 F1: a PEER fault mid-purge (a
  3327	                    // CancelJob on the serving source, a source-side
  3328	                    // abort) arrives as a control frame — a bare await
  3329	                    // here would leave it unread while deletions run to
  3330	                    // completion behind a cancelled session. Race ONE
  3331	                    // control-lane read against the pass (biased to the
  3332	                    // frame, so an already-queued cancel aborts the pass
  3333	                    // before its first delete); on any lane event, flip
  3334	                    // the abort flag, let the pass wind down at its next
  3335	                    // op, and surface the peer's fault instead of the
  3336	                    // aborted pass's error.
  3337	                    let mut peer_fault: Option<eyre::Report> = None;
  3338	                    let joined = tokio::select! {
  3339	                        biased;
  3340	                        received = transport.recv() => {
  3341	                            _abort_guard.0.store(true, Ordering::Release);
  3342	                            peer_fault = Some(match received {
  3343	                                Ok(Some(TransferFrame {
  3344	                                    frame: Some(Frame::Error(err)),
  3345	                                })) => eyre::Report::new(SessionFault::from_wire(err)),
  3346	                                Ok(Some(other)) => violation(format!(
  3347	                                    "unexpected {} during the mirror delete pass",
  3348	                                    frame_name(&other.frame)
  3349	                                )),
  3350	                                Ok(None) => eyre::Report::new(SessionFault::internal(
  3351	                                    "peer closed mid-session",
  3352	                                )),
  3353	                                Err(err) => err,
  3354	                            });
  3355	                            (&mut pass).await
  3356	                        }
  3357	                        joined = &mut pass => joined,
  3358	                    };
  3359	                    let pass_result = joined.map_err(|e| {
  3360	                        eyre::Report::new(SessionFault::internal(format!(
  3361	                            "mirror delete task panicked: {e}"
  3362	                        )))
  3363	                    })?;
  3364	                    if let Some(fault) = peer_fault {
  3365	                        // The peer's fault owns the outcome; the aborted
  3366	                        // pass's own "aborted" error is its consequence.
  3367	                        return Err(fault);
  3368	                    }
  3369	                    let (deleted_file_count, deleted_dir_count) = pass_result.map_err(|e| {
  3370	                        eyre::Report::new(SessionFault::internal(format!(
  3371	                            "mirror delete failed: {e:#}"
  3372	                        )))
  3373	                    })?;
  3374	                    // otp-11: the local summary reports the split; the
  3375	                    // wire summary keeps the one entries_deleted count.
  3376	                    if let Some(la) = &local_apply {
  3377	                        la.stats
  3378	                            .deleted_files
  3379	                            .store(deleted_file_count, Ordering::Relaxed);
  3380	                        la.stats
  3381	                            .deleted_dirs
  3382	                            .store(deleted_dir_count, Ordering::Relaxed);
  3383	                    }
  3384	                    deleted_file_count + deleted_dir_count
  3385	                } else {
  3386	                    0
  3387	                };
  3388	                let summary = TransferSummary {
  3389	                    files_transferred: files_written,
  3390	                    bytes_transferred: bytes_written,
  3391	                    entries_deleted,
  3392	                    in_stream_carrier_used,
  3393	                    files_resumed: files_resumed.load(Ordering::Relaxed),
  3394	                };
  3395	                transport.send(frame(Frame::Summary(summary))).await?;
  3396	                return Ok(DestinationOutcome {
  3397	                    summary,
  3398	                    needed_paths,
  3399	                    data_plane_streams,
  3400	                });
  3401	            }
  3402	            Some(Frame::Error(err)) => {
  3403	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  3404	            }
  3405	            other => {
  3406	                // Everything else is off-lane or off-phase here:
  3407	                // destination-lane frames echoed back (a ResizeAck or
  3408	                // BlockHashList the destination would never receive),
  3409	                // stray handshake frames, bare FileData/TarShardChunk
  3410	                // outside a record. Fail fast, no tolerant parsing.
  3411	                return Err(violation(format!(
  3412	                    "{} not valid on the destination's receive lane in this phase",
  3413	                    frame_name(&other)
  3414	                )));
  3415	            }
  3416	        }
  3417	    }
  3418	}
  3419	
  3420	/// The LOCAL carrier's twin of [`diff_chunk_and_send_needs`] (otp-11):
  3421	/// identical per-entry verdicts (the same [`destination_needs`] compare,
  3422	/// the same `granted` dedup, the same `needed_paths` record), but the
  3423	/// needed headers are planned into payloads and queued onto the
  3424	/// in-process apply pipeline instead of being granted to the source —
  3425	/// no frame is sent and nothing enters `outstanding`. Resume is
  3426	/// sink-level on the local carrier (`FsSinkConfig.resume`), so no need
  3427	/// is ever resume-flagged here.
  3428	#[allow(clippy::too_many_arguments)]
  3429	async fn diff_chunk_and_apply_local(
  3430	    local: &local::LocalApply,
  3431	    run: &mut Option<local::LocalApplyRun>,
  3432	    chunk: Vec<FileHeader>,
  3433	    dst_root: &Path,
  3434	    canonical_dst_root: Option<&Path>,
  3435	    compare_opts: &CompareOptions,
  3436	    granted: &mut HashSet<String>,
  3437	    needed_paths: &mut Vec<String>,
  3438	    progress: Option<&RemoteTransferProgress>,
  3439	) -> Result<()> {
  3440	    if chunk.is_empty() {
  3441	        return Ok(());
  3442	    }
  3443	    // Scanned workload (post-filter, pre-diff) — the summary's
  3444	    // scanned_files/scanned_bytes, folded where every manifest entry
  3445	    // passes through.
  3446	    local
  3447	        .stats
  3448	        .scanned_files
  3449	        .fetch_add(chunk.len() as u64, Ordering::Relaxed);
  3450	    local
  3451	        .stats
  3452	        .scanned_bytes
  3453	        .fetch_add(chunk.iter().map(|h| h.size).sum::<u64>(), Ordering::Relaxed);
  3454	
  3455	    // ONE diff core, both carriers (codex otp-11a F1): only the
  3456	    // dispatch differs — the wire twin grants these to the source,
  3457	    // this one plans and applies them in-process. The resume flag is
  3458	    // meaningless here (the local carrier's block phase is
  3459	    // sink-level).
  3460	    let needed = diff_chunk_verdicts(chunk, dst_root, canonical_dst_root, compare_opts).await?;
  3461	
  3462	    let fresh: Vec<FileHeader> = needed
  3463	        .into_iter()
  3464	        .map(|(header, _)| header)
  3465	        .filter(|header| granted.insert(header.relative_path.clone()))
  3466	        .collect();
  3467	    if fresh.is_empty() {
  3468	        return Ok(());
  3469	    }
  3470	    for header in &fresh {
  3471	        needed_paths.push(header.relative_path.clone());
  3472	    }
  3473	    if let Some(p) = progress {
  3474	        p.report_manifest_batch(fresh.len());
  3475	    }

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '3115,3230p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  3115	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  3116	                    for h in &shard.files {
  3117	                        if !out.remove(&h.relative_path) {
  3118	                            return Err(violation(format!(
  3119	                                "tar shard entry '{}' which is not on the need list",
  3120	                                h.relative_path
  3121	                            )));
  3122	                        }
  3123	                    }
  3124	                }
  3125	                // Capture member paths for the per-file progress lane
  3126	                // before the record consumes the shard (the data-plane
  3127	                // receive does the same); skip the allocation when no one
  3128	                // is listening.
  3129	                let member_paths: Option<Vec<String>> = progress.as_ref().map(|_| {
  3130	                    shard
  3131	                        .files
  3132	                        .iter()
  3133	                        .map(|h| h.relative_path.clone())
  3134	                        .collect()
  3135	                });
  3136	                let outcome = receive_tar_record(transport, sink.as_ref(), shard).await?;
  3137	                files_written += outcome.files_written as u64;
  3138	                bytes_written += outcome.bytes_written;
  3139	                if let Some(p) = &progress {
  3140	                    p.report_payload(0, outcome.bytes_written);
  3141	                    for path in member_paths.unwrap_or_default() {
  3142	                        p.report_file_complete(path);
  3143	                    }
  3144	                }
  3145	            }
  3146	            Some(Frame::Resize(resize)) => {
  3147	                // sf-2 shape correction (otp-4b-2 push, otp-5b-2 pull): the
  3148	                // SOURCE proposes one ADD; the DESTINATION grows its receive
  3149	                // set (bump `resize_live`) and acks so the SOURCE completes
  3150	                // the epoch-N socket. The control-lane frames are identical
  3151	                // in both directions — only the transport action flips: a
  3152	                // DESTINATION **responder** (push) ARMS a credential its
  3153	                // accept loop then accepts; a DESTINATION **initiator**
  3154	                // (pull) DIALS the epoch-N socket itself. Only ADD occurs
  3155	                // (REMOVE is a tuner concern, future work); anything else
  3156	                // fails fast.
  3157	                if data_plane_recv.is_none() {
  3158	                    return Err(violation(
  3159	                        "DataPlaneResize on a session with no data plane".into(),
  3160	                    ));
  3161	                }
  3162	                let op = DataPlaneResizeOp::try_from(resize.op)
  3163	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  3164	                if op != DataPlaneResizeOp::Add {
  3165	                    return Err(violation(format!(
  3166	                        "unsupported data-plane resize op {}",
  3167	                        op.as_str_name()
  3168	                    )));
  3169	                }
  3170	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  3171	                    return Err(violation(
  3172	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  3173	                    ));
  3174	                }
  3175	                // Cumulative ceiling bound (defense in depth — the source's
  3176	                // dial already clamps to the same profile). Under the
  3177	                // ceiling, grow per connection role: arm the credential
  3178	                // (responder) or dial the epoch-N socket (initiator). A
  3179	                // dial failure is fatal (`add_dialed_stream`); a gone accept
  3180	                // loop returns false (arm). The initiator dials BEFORE the
  3181	                // ack so the SOURCE responder — which accepts on the ack —
  3182	                // never commits to an accept the DESTINATION did not dial.
  3183	                let accepted = if resize_live < resize_ceiling {
  3184	                    match data_plane_recv
  3185	                        .as_mut()
  3186	                        .expect("data plane present (checked above)")
  3187	                    {
  3188	                        data_plane::DestRecvPlane::Responder(run) => {
  3189	                            run.arm(resize.sub_token.clone())
  3190	                        }
  3191	                        data_plane::DestRecvPlane::Initiator(run) => {
  3192	                            run.add_dialed_stream(&resize.sub_token).await?;
  3193	                            true
  3194	                        }
  3195	                    }
  3196	                } else {
  3197	                    false
  3198	                };
  3199	                if accepted {
  3200	                    resize_live += 1;
  3201	                }
  3202	                let effective = if accepted {
  3203	                    resize.target_stream_count
  3204	                } else {
  3205	                    resize_live as u32
  3206	                };
  3207	                transport
  3208	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  3209	                        epoch: resize.epoch,
  3210	                        effective_stream_count: effective,
  3211	                        accepted,
  3212	                    })))
  3213	                    .await?;
  3214	            }
  3215	            Some(Frame::SourceDone(_)) => {
  3216	                if !manifest_complete {
  3217	                    return Err(violation("SourceDone before ManifestComplete".into()));
  3218	                }
  3219	                // Completion, both carriers: the shared `outstanding`
  3220	                // set must be empty (every granted need claimed exactly
  3221	                // once). In-stream claims inline above; the data-plane
  3222	                // NeedListSink claims as payloads land, so joining the
  3223	                // receive task first drains the last of them (and
  3224	                // surfaces any receive error / stall). Set membership —
  3225	                // not a file count — is the contract (codex F1: a count
  3226	                // proxy let a peer substitute or duplicate paths).
  3227	                // `finish()` drops the arm sender (no more resizes), joins
  3228	                // the accept loop, and reports the settled stream count.
  3229	                //
  3230	                // otp-11: the LOCAL carrier joins its apply pipeline with

exec
/bin/zsh -lc "rg -n -C 8 \"DataPlaneResize|resize epoch|max_streams|unknown\" proto crates/blit-core/src/generated.rs docs/plan/ONE_TRANSFER_PATH.md docs/plan -g '*.proto' -g '*.rs' -g '*.md' | sed -n '1,700p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
rg: crates/blit-core/src/generated.rs: No such file or directory (os error 2)
docs/plan/ONE_TRANSFER_PATH.md-86-  dropped) as tests become role-parameterized; test count never
docs/plan/ONE_TRANSFER_PATH.md-87-  drops.
docs/plan/ONE_TRANSFER_PATH.md-88-- The sf-2 shape-correction behavior (stream count corrects as the
docs/plan/ONE_TRANSFER_PATH.md-89-  need list accumulates) becomes the one and only stream policy —
docs/plan/ONE_TRANSFER_PATH.md-90-  both directions inherit it by construction; its pins carry over.
docs/plan/ONE_TRANSFER_PATH.md-91-- **The bounded-unilateral dial contract carries unchanged**
docs/plan/ONE_TRANSFER_PATH.md-92-  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
docs/plan/ONE_TRANSFER_PATH.md-93-  dial, bounded by the byte RECEIVER's advertised capacity profile
docs/plan/ONE_TRANSFER_PATH.md:94:  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
docs/plan/ONE_TRANSFER_PATH.md-95-  unlimited). The session's role model must express this — profile
docs/plan/ONE_TRANSFER_PATH.md-96-  travels DESTINATION→SOURCE at setup regardless of who initiated —
docs/plan/ONE_TRANSFER_PATH.md-97-  and otp-1's contract names it explicitly.
docs/plan/ONE_TRANSFER_PATH.md-98-- Wire contract discipline (REV4 rule): the unified session's proto —
docs/plan/ONE_TRANSFER_PATH.md-99-  messages, field numbers, capability negotiation, transport
docs/plan/ONE_TRANSFER_PATH.md-100-  selection — is a reviewed doc+proto slice **before** any behavior
docs/plan/ONE_TRANSFER_PATH.md-101-  depends on it.
docs/plan/ONE_TRANSFER_PATH.md-102-- Every slice through the codex loop (D-2026-07-04-1); tree green
--
docs/plan/ONE_TRANSFER_PATH.md-258-   exchanged at session open; any mismatch is refused with a clear
docs/plan/ONE_TRANSFER_PATH.md-259-   error — D-2026-07-05-2; pinned by test when the session lands),
docs/plan/ONE_TRANSFER_PATH.md-260-   the receiver capacity profile + bounded-unilateral dial contract
docs/plan/ONE_TRANSFER_PATH.md-261-   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
docs/plan/ONE_TRANSFER_PATH.md-262-   that exists), transport selection, resume phase ordering (the
docs/plan/ONE_TRANSFER_PATH.md-263-   RELIABLE exception above), mirror phase, error/cancel semantics.
docs/plan/ONE_TRANSFER_PATH.md-264-   No feature-capability bits: same build implies same features.
docs/plan/ONE_TRANSFER_PATH.md-265-   The new proto text must carry NO version-tolerance semantics; the
docs/plan/ONE_TRANSFER_PATH.md:266:   capacity profile's absent/0 fields mean "unknown hardware value"
docs/plan/ONE_TRANSFER_PATH.md-267-   only, never "old peer" (today's proto comments frame some of that
docs/plan/ONE_TRANSFER_PATH.md-268-   contract as old-peer fallback — those comment blocks describe live
docs/plan/ONE_TRANSFER_PATH.md-269-   pre-cutover code and die with their messages at otp-10, per the
docs/plan/ONE_TRANSFER_PATH.md-270-   D-2026-07-05-2 review adjudication). Codex-reviewed before any
docs/plan/ONE_TRANSFER_PATH.md-271-   code consumes it.
docs/plan/ONE_TRANSFER_PATH.md-272-2. **otp-2 symmetric baseline (harness + rig, no production code)**:
docs/plan/ONE_TRANSFER_PATH.md-273-   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
docs/plan/ONE_TRANSFER_PATH.md-274-   cells, cold caches, tmpfs rows re-labeled wire-reference only —
--
docs/plan/ONE_TRANSFER_PATH.md-308-    this plan's acceptance evidence.
docs/plan/ONE_TRANSFER_PATH.md-309-13. **otp-13 verdict**: acceptance checklist walked with the owner;
docs/plan/ONE_TRANSFER_PATH.md-310-    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
docs/plan/ONE_TRANSFER_PATH.md-311-    against the unified baseline — owner call at that point.
docs/plan/ONE_TRANSFER_PATH.md-312-
docs/plan/ONE_TRANSFER_PATH.md-313-## Open questions
docs/plan/ONE_TRANSFER_PATH.md-314-
docs/plan/ONE_TRANSFER_PATH.md-315-- None requiring owner input now — scope, wire, and process were
docs/plan/ONE_TRANSFER_PATH.md:316:  delegated (Directive section). Slice-level unknowns (exact proto
docs/plan/ONE_TRANSFER_PATH.md-317-  shapes, resume edge semantics, TUI event wiring) are settled inside
docs/plan/ONE_TRANSFER_PATH.md-318-  their slices through the codex loop. — owner
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-93-  (`blit-core/build.rs:28-97`) — **all arms must be clean-tree builds; arms
docs/plan/OTP12_ACCEPTANCE_RUN.md-94-  swap BOTH ends together (matched pairs)**.
docs/plan/OTP12_ACCEPTANCE_RUN.md-95-- Old-arm binaries route the OLD drivers: `e757dcc` (zoey pair, staged in
docs/plan/OTP12_ACCEPTANCE_RUN.md-96-  `blit-temp/` — `.agents/machines.md`) and `0f922de` (Windows pair, checkout
docs/plan/OTP12_ACCEPTANCE_RUN.md-97-  detached there) both PREDATE the verb cutover (`0fbc966`), so their verbs
docs/plan/OTP12_ACCEPTANCE_RUN.md-98-  still call `Push`/`PullSync` — they are genuine old-path arms. Verified by
docs/plan/OTP12_ACCEPTANCE_RUN.md-99-  ancestry + `git ls-tree` (old drivers present at both shas).
docs/plan/OTP12_ACCEPTANCE_RUN.md-100-- July skippy binaries (`/mnt/generic-pool/video/blit-bin/`) are REV4-era:
docs/plan/OTP12_ACCEPTANCE_RUN.md:101:  unknown commit, no `Transfer` RPC, no handshake — **unusable for any
docs/plan/OTP12_ACCEPTANCE_RUN.md-102-  otp-12 arm**; skippy gets fresh staging (D6).
docs/plan/OTP12_ACCEPTANCE_RUN.md-103-- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md-104-  per-direction only — hardware-asymmetric endpoints, D-2026-07-05-1
docs/plan/OTP12_ACCEPTANCE_RUN.md-105-  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
docs/plan/OTP12_ACCEPTANCE_RUN.md-106-  owner-designated cross-direction rig).
docs/plan/OTP12_ACCEPTANCE_RUN.md-107-- Flags a harness touches that changed since the old scripts: none — `copy`,
docs/plan/OTP12_ACCEPTANCE_RUN.md-108-  `--yes`, `--force-grpc` are name-stable; `--diagnostics-counter-file` is a
docs/plan/OTP12_ACCEPTANCE_RUN.md-109-  global flag preceding the subcommand.
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-370-(otp-2 precedent) and each script supports `PREFLIGHT_ONLY=1` (run every
docs/plan/OTP12_ACCEPTANCE_RUN.md-371-preflight check and exit before fixtures).
docs/plan/OTP12_ACCEPTANCE_RUN.md-372-
docs/plan/OTP12_ACCEPTANCE_RUN.md-373-### D6 — staging per host
docs/plan/OTP12_ACCEPTANCE_RUN.md-374-
docs/plan/OTP12_ACCEPTANCE_RUN.md-375-| host | old arm | new arm |
docs/plan/OTP12_ACCEPTANCE_RUN.md-376-|------|---------|---------|
docs/plan/OTP12_ACCEPTANCE_RUN.md-377-| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
docs/plan/OTP12_ACCEPTANCE_RUN.md:378:| zoey | clean `e757dcc` zigbuild staged as `blit-daemon-e757dcc` — the 2026-07-10 staging at `blit-daemon` FAILED provenance (a dirty `731023b` build; correction note in the otp-2 README) and is left untouched as the otp-2 artifact | `cargo zigbuild --release --target aarch64-unknown-linux-musl` → staged beside as `blit-daemon-<sha>` (never overwrite); everything stays inside `blit-temp/` |
docs/plan/OTP12_ACCEPTANCE_RUN.md-379-| Windows | copy the detached-checkout exes ASIDE first (`D:\blit-test\bins\0f922de\`) before any checkout movement | fresh git bundle (pushes are owner-gated; origin lags at `6d37a22`) → checkout run commit → native `cargo build --release` (daemon AND `blit.exe` client) → `D:\blit-test\bins\<sha>\` |
docs/plan/OTP12_ACCEPTANCE_RUN.md:380:| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |
docs/plan/OTP12_ACCEPTANCE_RUN.md-381-
docs/plan/OTP12_ACCEPTANCE_RUN.md-382-Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
docs/plan/OTP12_ACCEPTANCE_RUN.md-383-path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
docs/plan/OTP12_ACCEPTANCE_RUN.md-384-one program-scoped firewall rule total (the rule is exe-path-scoped;
docs/plan/OTP12_ACCEPTANCE_RUN.md-385-sha-named dirs keep provenance, the copy log records each swap). Launch
docs/plan/OTP12_ACCEPTANCE_RUN.md-386-stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
docs/plan/OTP12_ACCEPTANCE_RUN.md-387-(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
docs/plan/OTP12_ACCEPTANCE_RUN.md-388-on every host) is recorded in each evidence README.
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-440-  directions still land on different OS write paths (APFS vs NTFS +
docs/plan/OTP12_ACCEPTANCE_RUN.md-441-  Defender at its normal state). D2's discriminator computation is the
docs/plan/OTP12_ACCEPTANCE_RUN.md-442-  pre-registered, evidence-backed handling; a platform-residue cell counts
docs/plan/OTP12_ACCEPTANCE_RUN.md-443-  as satisfied per D-2026-07-12-1.
docs/plan/OTP12_ACCEPTANCE_RUN.md-444-- **Old-arm provenance is a staging record, not a handshake** (old paths
docs/plan/OTP12_ACCEPTANCE_RUN.md-445-  predate it). Mitigated by machines.md provenance + the sha256 manifest;
docs/plan/OTP12_ACCEPTANCE_RUN.md-446-  accepted residual risk.
docs/plan/OTP12_ACCEPTANCE_RUN.md-447-- **First-of-kind surfaces**: a daemon on the Mac (application firewall
docs/plan/OTP12_ACCEPTANCE_RUN.md:448:  unknown until the smoke) and a client on skippy (musl-static, untested
docs/plan/OTP12_ACCEPTANCE_RUN.md-449-  there — the zoey zigbuild recipe retargeted). Both are preflight-gated;
docs/plan/OTP12_ACCEPTANCE_RUN.md-450-  failures block the affected block only.
docs/plan/OTP12_ACCEPTANCE_RUN.md-451-- **zoey availability**: under maintenance 2026-07-11; daemon runs there
docs/plan/OTP12_ACCEPTANCE_RUN.md-452-  need a fresh owner go regardless (STATE rule).
docs/plan/OTP12_ACCEPTANCE_RUN.md-453-- **Delegated arm includes trigger/relay overhead by design** — recorded,
docs/plan/OTP12_ACCEPTANCE_RUN.md-454-  expected sub-ms on this LAN; if it ever dominates a cell, that IS a
docs/plan/OTP12_ACCEPTANCE_RUN.md-455-  finding, not noise.
docs/plan/OTP12_ACCEPTANCE_RUN.md-456-- **Suite/test count**: untouched — no crates/proto changes anywhere in
--
proto/blit.proto-131-// is no probe phase to catch it before the first byte).
proto/blit.proto-132-//
proto/blit.proto-133-// Travel direction (receiver → sender): the session DESTINATION
proto/blit.proto-134-// advertises its profile in SessionOpen/SessionAccept.receiver_capacity
proto/blit.proto-135-// (whichever end plays DESTINATION). Nothing else carries one — the
proto/blit.proto-136-// delegated initiator is the dst daemon itself (otp-9b), so it
proto/blit.proto-137-// advertises its own capacity at session open.
proto/blit.proto-138-//
proto/blit.proto:139:// Every field uses 0 (or UNSPECIFIED) as "unknown"; the sender treats
proto/blit.proto:140:// unknown as "no information — stay conservative", never as "unlimited".
proto/blit.proto-141-message CapacityProfile {
proto/blit.proto-142-  // Logical CPU cores the receiver can devote to this transfer.
proto/blit.proto-143-  uint32 cpu_cores = 1;
proto/blit.proto-144-  // Storage class of the receive target, the coarse drain-speed signal.
proto/blit.proto-145-  DrainClass drain_class = 2;
proto/blit.proto-146-  // Receiver's current overall load estimate, percent (0-100+; may
proto/blit.proto:147:  // exceed 100 when oversubscribed, e.g. loadavg > cores). 0 = unknown
proto/blit.proto-148-  // or idle — senders must not distinguish the two.
proto/blit.proto-149-  uint32 load_percent = 3;
proto/blit.proto-150-  // Maximum parallel data-plane streams the receiver will accept for
proto/blit.proto-151-  // this transfer (the dial's hard ceiling; floor is always 1).
proto/blit.proto:152:  // 0 = unknown → sender stays at today's negotiated stream_count.
proto/blit.proto:153:  uint32 max_streams = 4;
proto/blit.proto-154-  // Estimated sustainable drain (write-to-storage) rate in bytes/sec.
proto/blit.proto-155-  uint64 drain_rate_bytes_per_sec = 5;
proto/blit.proto-156-  // Largest single chunk the receiver wants on the wire, bytes.
proto/blit.proto-157-  uint64 max_chunk_bytes = 6;
proto/blit.proto-158-  // Ceiling on prefetch / un-acked in-flight bytes the receiver can
proto/blit.proto-159-  // buffer safely.
proto/blit.proto-160-  uint64 max_inflight_bytes = 7;
proto/blit.proto-161-}
--
proto/blit.proto-172-}
proto/blit.proto-173-
proto/blit.proto-174-// ── ue-r2-1b: mid-transfer stream resize (consumed at ue-r2-2) ──────
proto/blit.proto-175-// Control-plane request to grow/shrink the live data-plane stream set.
proto/blit.proto-176-// Carried on the transfer control streams (never as a blind TCP
proto/blit.proto-177-// data-plane record), and only when the session negotiated resize
proto/blit.proto-178-// support at open.
proto/blit.proto-179-// Shape carried over from the adaptive-streams PR3 prior art (d9d4ec7).
proto/blit.proto:180:enum DataPlaneResizeOp {
proto/blit.proto-181-  DATA_PLANE_RESIZE_OP_UNSPECIFIED = 0;
proto/blit.proto-182-  DATA_PLANE_RESIZE_OP_ADD = 1;
proto/blit.proto-183-  DATA_PLANE_RESIZE_OP_REMOVE = 2;
proto/blit.proto-184-}
proto/blit.proto-185-
proto/blit.proto-186-// Controller → peer request to resize the live stream set. `epoch` is a
proto/blit.proto-187-// monotonic resize id (0 is reserved for the initial streams);
proto/blit.proto-188-// `target_stream_count` is the absolute desired live count (idempotent),
proto/blit.proto:189:// bounded by CapacityProfile.max_streams. For ADD, `sub_token` is the
proto/blit.proto-190-// 16-byte credential the newly dialed data socket must present after
proto/blit.proto-191-// the one_time_token; the accepting side registers it before the dialer
proto/blit.proto-192-// dials. For REMOVE, sub_token is empty.
proto/blit.proto:193:message DataPlaneResize {
proto/blit.proto:194:  DataPlaneResizeOp op = 1;
proto/blit.proto-195-  uint32 epoch = 2;
proto/blit.proto-196-  uint32 target_stream_count = 3;
proto/blit.proto-197-  bytes sub_token = 4;
proto/blit.proto-198-}
proto/blit.proto-199-
proto/blit.proto:200:// Ack of a DataPlaneResize. `accepted` is false if the peer could not
proto/blit.proto-201-// honor it (e.g. registration refused, dial failed);
proto/blit.proto-202-// `effective_stream_count` is the live count the acking side now
proto/blit.proto-203-// believes is in effect.
proto/blit.proto:204:message DataPlaneResizeAck {
proto/blit.proto-205-  uint32 epoch = 1;
proto/blit.proto-206-  uint32 effective_stream_count = 2;
proto/blit.proto-207-  bool accepted = 3;
proto/blit.proto-208-}
proto/blit.proto-209-
proto/blit.proto-210-// Shared transfer frames (session manifest / payload building blocks).
proto/blit.proto-211-message FileHeader {
proto/blit.proto-212-  string relative_path = 1;
--
proto/blit.proto-336-// Removed 2026-05-13: AuthRequest / AuthResponse — see BlitAuth note above.
proto/blit.proto-337-
proto/blit.proto-338-// ─────────────────────────────────────────────────────────────────────
proto/blit.proto-339-// TransferOperationSpec — the unified contract handed from initiator
proto/blit.proto-340-// to origin for every transfer (push, pull, mirror, remote→remote).
proto/blit.proto-341-// The wire shape — typed enums and orthogonal fields, not bool soup.
proto/blit.proto-342-// Receivers normalize this via `NormalizedTransferOperation::from_spec`
proto/blit.proto-343-// at the boundary; downstream code never sees raw proto-`Unspecified`
proto/blit.proto:344:// or unknown enum values.
proto/blit.proto-345-// ─────────────────────────────────────────────────────────────────────
proto/blit.proto-346-
proto/blit.proto-347-// What the initiator tells the origin about a transfer. Normalized
proto/blit.proto-348-// intent — not a flag bag. Identical regardless of whether the origin
proto/blit.proto-349-// is the local CLI (push), a remote daemon (pull), or daemon A talking
proto/blit.proto-350-// to daemon B (remote→remote, future).
proto/blit.proto-351-message TransferOperationSpec {
proto/blit.proto-352-  // Bumped when the wire shape of this message or any of its fields
--
proto/blit.proto-699-  TRANSFER_KIND_PULL = 2;
proto/blit.proto-700-  TRANSFER_KIND_PULL_SYNC = 3;
proto/blit.proto-701-  TRANSFER_KIND_DELEGATED_PULL = 4;
proto/blit.proto-702-}
proto/blit.proto-703-
proto/blit.proto-704-message ActiveTransfer {
proto/blit.proto-705-  string transfer_id = 1;
proto/blit.proto-706-  TransferKind kind = 2;
proto/blit.proto:707:  // `<ip>:<port>` of the connecting peer, or "unknown" when the
proto/blit.proto-708-  // transport didn't surface one (in-process tests).
proto/blit.proto-709-  string peer = 3;
proto/blit.proto-710-  string module = 4;
proto/blit.proto-711-  // Module-relative path the transfer targets. Empty until a served
proto/blit.proto-712-  // session resolves its open frame; populated immediately for
proto/blit.proto-713-  // delegated_pull (the only unary-request kind).
proto/blit.proto-714-  string path = 5;
proto/blit.proto-715-  uint64 start_unix_ms = 6;
--
proto/blit.proto-839-// `GetState.active[]`). A served session fires this with empty
proto/blit.proto-840-// `module`/`path` because those values arrive in the open frame, not
proto/blit.proto-841-// at dispatch time — subscribers can reconcile via a follow-up
proto/blit.proto-842-// GetState query if they need the populated endpoint before the
proto/blit.proto-843-// transfer completes.
proto/blit.proto-844-message TransferStarted {
proto/blit.proto-845-  string transfer_id = 1;
proto/blit.proto-846-  TransferKind kind = 2;
proto/blit.proto:847:  // `<ip>:<port>` of the connecting peer, or "unknown" when the
proto/blit.proto-848-  // transport didn't surface one (in-process tests).
proto/blit.proto-849-  string peer = 3;
proto/blit.proto-850-  // Module name on the daemon. Empty for streaming RPCs at
proto/blit.proto-851-  // registration time — populated by GetState.active[] once the
proto/blit.proto-852-  // first stream frame parses.
proto/blit.proto-853-  string module = 4;
proto/blit.proto-854-  // Module-relative path. Same "empty until first frame" caveat
proto/blit.proto-855-  // as `module`.
--
proto/blit.proto-968-  // Request the in-stream byte carrier (diagnostics / unreachable
proto/blit.proto-969-  // data-plane environments). The responder may also force it via a
proto/blit.proto-970-  // grant-less SessionAccept when it cannot bind a listener.
proto/blit.proto-971-  bool in_stream_bytes = 9;
proto/blit.proto-972-  bool ignore_existing = 10;
proto/blit.proto-973-  bool require_complete_scan = 11;
proto/blit.proto-974-  // Set iff the initiator is DESTINATION (dial contract: the byte
proto/blit.proto-975-  // receiver advertises capacity — D-2026-06-20-1/-2; absent/0 =
proto/blit.proto:976:  // unknown hardware value, conservative, never "old peer").
proto/blit.proto-977-  CapacityProfile receiver_capacity = 12;
proto/blit.proto-978-}
proto/blit.proto-979-
proto/blit.proto-980-// Responder's reply. Refusals are SessionError frames, never silent
proto/blit.proto-981-// closes.
proto/blit.proto-982-message SessionAccept {
proto/blit.proto-983-  // Set iff the responder is DESTINATION.
proto/blit.proto-984-  CapacityProfile receiver_capacity = 1;
--
proto/blit.proto-1088-    BlockHashList block_hashes = 8;
proto/blit.proto-1089-    FileHeader file_begin = 9;
proto/blit.proto-1090-    FileData file_data = 10;
proto/blit.proto-1091-    TarShardHeader tar_shard_header = 11;
proto/blit.proto-1092-    TarShardChunk tar_shard_chunk = 12;
proto/blit.proto-1093-    TarShardComplete tar_shard_complete = 13;
proto/blit.proto-1094-    BlockTransfer block = 14;
proto/blit.proto-1095-    BlockTransferComplete block_complete = 15;
proto/blit.proto:1096:    DataPlaneResize resize = 16;
proto/blit.proto:1097:    DataPlaneResizeAck resize_ack = 17;
proto/blit.proto-1098-    SourceDone source_done = 18;
proto/blit.proto-1099-    TransferSummary summary = 19;
proto/blit.proto-1100-    SessionError error = 20;
proto/blit.proto-1101-  }
proto/blit.proto-1102-}
--
docs/plan/ONE_TRANSFER_PATH.md-86-  dropped) as tests become role-parameterized; test count never
docs/plan/ONE_TRANSFER_PATH.md-87-  drops.
docs/plan/ONE_TRANSFER_PATH.md-88-- The sf-2 shape-correction behavior (stream count corrects as the
docs/plan/ONE_TRANSFER_PATH.md-89-  need list accumulates) becomes the one and only stream policy —
docs/plan/ONE_TRANSFER_PATH.md-90-  both directions inherit it by construction; its pins carry over.
docs/plan/ONE_TRANSFER_PATH.md-91-- **The bounded-unilateral dial contract carries unchanged**
docs/plan/ONE_TRANSFER_PATH.md-92-  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
docs/plan/ONE_TRANSFER_PATH.md-93-  dial, bounded by the byte RECEIVER's advertised capacity profile
docs/plan/ONE_TRANSFER_PATH.md:94:  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
docs/plan/ONE_TRANSFER_PATH.md-95-  unlimited). The session's role model must express this — profile
docs/plan/ONE_TRANSFER_PATH.md-96-  travels DESTINATION→SOURCE at setup regardless of who initiated —
docs/plan/ONE_TRANSFER_PATH.md-97-  and otp-1's contract names it explicitly.
docs/plan/ONE_TRANSFER_PATH.md-98-- Wire contract discipline (REV4 rule): the unified session's proto —
docs/plan/ONE_TRANSFER_PATH.md-99-  messages, field numbers, capability negotiation, transport
docs/plan/ONE_TRANSFER_PATH.md-100-  selection — is a reviewed doc+proto slice **before** any behavior
docs/plan/ONE_TRANSFER_PATH.md-101-  depends on it.
docs/plan/ONE_TRANSFER_PATH.md-102-- Every slice through the codex loop (D-2026-07-04-1); tree green
--
docs/plan/ONE_TRANSFER_PATH.md-258-   exchanged at session open; any mismatch is refused with a clear
docs/plan/ONE_TRANSFER_PATH.md-259-   error — D-2026-07-05-2; pinned by test when the session lands),
docs/plan/ONE_TRANSFER_PATH.md-260-   the receiver capacity profile + bounded-unilateral dial contract
docs/plan/ONE_TRANSFER_PATH.md-261-   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
docs/plan/ONE_TRANSFER_PATH.md-262-   that exists), transport selection, resume phase ordering (the
docs/plan/ONE_TRANSFER_PATH.md-263-   RELIABLE exception above), mirror phase, error/cancel semantics.
docs/plan/ONE_TRANSFER_PATH.md-264-   No feature-capability bits: same build implies same features.
docs/plan/ONE_TRANSFER_PATH.md-265-   The new proto text must carry NO version-tolerance semantics; the
docs/plan/ONE_TRANSFER_PATH.md:266:   capacity profile's absent/0 fields mean "unknown hardware value"
docs/plan/ONE_TRANSFER_PATH.md-267-   only, never "old peer" (today's proto comments frame some of that
docs/plan/ONE_TRANSFER_PATH.md-268-   contract as old-peer fallback — those comment blocks describe live
docs/plan/ONE_TRANSFER_PATH.md-269-   pre-cutover code and die with their messages at otp-10, per the
docs/plan/ONE_TRANSFER_PATH.md-270-   D-2026-07-05-2 review adjudication). Codex-reviewed before any
docs/plan/ONE_TRANSFER_PATH.md-271-   code consumes it.
docs/plan/ONE_TRANSFER_PATH.md-272-2. **otp-2 symmetric baseline (harness + rig, no production code)**:
docs/plan/ONE_TRANSFER_PATH.md-273-   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
docs/plan/ONE_TRANSFER_PATH.md-274-   cells, cold caches, tmpfs rows re-labeled wire-reference only —
--
docs/plan/ONE_TRANSFER_PATH.md-308-    this plan's acceptance evidence.
docs/plan/ONE_TRANSFER_PATH.md-309-13. **otp-13 verdict**: acceptance checklist walked with the owner;
docs/plan/ONE_TRANSFER_PATH.md-310-    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
docs/plan/ONE_TRANSFER_PATH.md-311-    against the unified baseline — owner call at that point.
docs/plan/ONE_TRANSFER_PATH.md-312-
docs/plan/ONE_TRANSFER_PATH.md-313-## Open questions
docs/plan/ONE_TRANSFER_PATH.md-314-
docs/plan/ONE_TRANSFER_PATH.md-315-- None requiring owner input now — scope, wire, and process were
docs/plan/ONE_TRANSFER_PATH.md:316:  delegated (Directive section). Slice-level unknowns (exact proto
docs/plan/ONE_TRANSFER_PATH.md-317-  shapes, resume edge semantics, TUI event wiring) are settled inside
docs/plan/ONE_TRANSFER_PATH.md-318-  their slices through the codex loop. — owner
--
docs/plan/POST_REVIEW_FIXES.md-247-
docs/plan/POST_REVIEW_FIXES.md-248-### 3.1 Adaptive tuning expansion
docs/plan/POST_REVIEW_FIXES.md-249-
docs/plan/POST_REVIEW_FIXES.md-250-- `auto_tune` covers `chunk_bytes`, `initial_streams`, `prefetch_count`,
docs/plan/POST_REVIEW_FIXES.md-251-  `tcp_buffer_size`. **Doesn't cover**: manifest batch size, channel
docs/plan/POST_REVIEW_FIXES.md-252-  capacities, planner thresholds (size buckets, tar shard targets),
docs/plan/POST_REVIEW_FIXES.md-253-  `RECEIVE_CHUNK_SIZE`.
docs/plan/POST_REVIEW_FIXES.md-254-- Bucketing is coarse — three bandwidth brackets, two chunk sizes,
docs/plan/POST_REVIEW_FIXES.md:255:  fixed `max_streams = 8`. No RTT, no filesystem-type, no mid-transfer
docs/plan/POST_REVIEW_FIXES.md-256-  feedback.
docs/plan/POST_REVIEW_FIXES.md-257-- All static thresholds in `transfer_plan.rs` and `remote/tuning.rs`
docs/plan/POST_REVIEW_FIXES.md-258-  should funnel through `TuningParams`.
docs/plan/POST_REVIEW_FIXES.md-259-
docs/plan/POST_REVIEW_FIXES.md-260-Target metric: **adaptive batched manifest** (the whitepaper's §8.1
docs/plan/POST_REVIEW_FIXES.md-261-fix) closes the small-file cold gap vs rsync. Ship that first as the
docs/plan/POST_REVIEW_FIXES.md-262-"first non-trivial use of adaptive tuning beyond chunk_bytes" and use
docs/plan/POST_REVIEW_FIXES.md-263-the implementation as the template for migrating the other hardcoded
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-211-REV2 makes wire shape an early deliverable. Proposed proto direction:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-212-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-213-- append `CapacityProfile receiver_capacity = 11` to
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-214-  `DataTransferNegotiation` rather than using reserved RDMA fields 5-10;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-215-- add a capacity profile to the request/setup side where the receiver is
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-216-  the client, especially PullSync and delegated pull;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-217-- add explicit peer capability bits/fields so resize messages are never
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-218-  sent to an old peer;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:219:- add `DataPlaneResize` and `DataPlaneResizeAck` as negotiated control
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-220-  messages in the relevant control streams, not as blind TCP data-plane
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-221-  records.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-222-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-223-Exact field names and numbers are part of the wire slice acceptance
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-224-criteria. Old peers must see current behavior: no capacity profile means
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-225-use today's static/conservative behavior; no resize support means no
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-226-mid-transfer add/drop.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-227-
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-274-7. **`ue-r2-1g-pull-multistream-converge`** - Route PullSync through the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-275-   engine and make pull multistream there. Preserve resume, checksum
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-276-   refusal, delete-list authority, cancellation, per-stream failure, and
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-277-   gRPC fallback behavior.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-278-8. **`ue-r2-1h-delete-deprecated-pull-rpc`** - Delete deprecated `Pull`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-279-   after PullSync has harvested the needed multistream/fallback pattern
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-280-   and tests cover the replacement.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-281-9. **`ue-r2-2-stream-resize`** - Finish negotiated
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md:282:   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-283-   transfer from live telemetry, using the elastic work queue from
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-284-   `ue-r2-1a`.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-285-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-286-## Review Findings Rolled In
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-287-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-288-- `ue-1c` was too large: split streaming-plan foundation, local adapter,
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-289-  push convergence, and pull convergence into separate slices.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md-290-- Local fast paths conflicted with "no separate small-transfer path":
--
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-297-}
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-298-```
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-299-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-300-`FilterSpec` is **already** defined at `proto/blit.proto:367-392` and the CLI
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-301-already produces one through `build_filter_spec` in
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-302-`crates/blit-cli/src/transfers/mod.rs`. We reuse both without modification.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-303-Version drift is handled through `TransferOperationSpec.spec_version` and
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-304-`PeerCapabilities`, which `NormalizedTransferOperation::from_spec` already
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:305:validates at the boundary. We do **not** rely on detecting unknown protobuf
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-306-fields (proto3 silently preserves them; that's not a compatibility strategy).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-307-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-308-### 4.2 New code paths
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-309-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-310-**CLI side (`crates/blit-cli/src/transfers/`):**
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-311-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-312-- New module `remote_remote_direct.rs`:
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-313-  ```rust
--
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-364-  connect before policy approves** (R23-F2):
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-365-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-366-  1. **Parse `RemoteSourceLocator`** through the existing
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-367-     `RemoteEndpoint` parser. Reject schemes other than the Blit gRPC
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-368-     control plane scheme. Reject malformed host/port.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-369-  2. **Spec validation**: validate `spec.spec_version`,
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-370-     `PeerCapabilities`, and convert via
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-371-     `NormalizedTransferOperation::from_spec` — exactly like the existing
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:372:     push and pull handlers. No parallel normalizer. Reject unknown
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-373-     versions explicitly.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-374-  3. **Daemon-wide gate** (§4.3): if `allow_delegated_pull == false`,
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-375-     return `DelegatedPullError{phase=DELEGATION_REJECTED}` immediately.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-376-     If `allowed_source_hosts` is non-empty, **resolve the source
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-377-     hostname to an IP set, validate every resolved address against the
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-378-     allowlist, and bind the connection to that resolved IP** (see §4.3
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-379-     for full semantics). On failure: same error phase.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-380-  4. **Module metadata lookup** for `dst_module` (no path resolution
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:381:     yet). Reject if module unknown or read-only (returning the existing
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-382-     read-only-module error code).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-383-  5. **Per-module override**: if the module's delegation-allowed flag is
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-384-     `false`, return `DELEGATION_REJECTED`. (The override only narrows
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-385-     the daemon-wide policy; it cannot widen it.)
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-386-  6. **F2 canonical-path containment** on `dst_destination_path` via
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-387-     `resolve_contained_path` in `crates/blit-daemon/src/service/util.rs`.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-388-     Reject contained-path violations at the boundary, before any
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-389-     outbound connect.
--
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-953-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-954-**Unit:**
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-955-- `TransferOperationSpec` round-trip across delegated handler boundary —
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-956-  ensure nothing is dropped/flattened (regression guard for R21-F1).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-957-- Delegation gate matrix (disabled / enabled / enabled+allowlist /
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-958-  allowlist mismatch / per-module override).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-959-- Containment check on `dst_module + dst_destination_path` rejects
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-960-  symlink escapes (extends F2 test pattern).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:961:- `spec_version` normalizer rejects unknown versions explicitly
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:962:  (regression guard for R21-F6 — we do not rely on unknown-field
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-963-  detection).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-964-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-965-**Integration (CLI tests calling real daemon binaries):**
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-966-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-967-*Byte-path isolation (load-bearing — addresses R21-F7 + R23-F4):*
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-968-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-969-The destination's view of "who connected to whom" is not authoritative.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-970-A destination daemon can only read its own local and remote socket
--
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1034-- Linux/Linux pair, 10 GbE LAN, CLI on 100 Mbit link: expect 50–100×
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1035-  improvement (CLI was the bottleneck).
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1036-- Linux/Linux pair, RDMA-capable: defer to Phase 3.5.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1037-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1038-## 7. Risks and open questions
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1039-
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1040-| Risk | Mitigation |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1041-|---|---|
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:1042:| `TransferOperationSpec` evolves and dst sees a newer version than it understands | Use `spec_version` + `PeerCapabilities`; `NormalizedTransferOperation::from_spec` rejects unknown versions explicitly with a clear error. We do not depend on protobuf unknown-field detection (proto3 silently preserves unknowns). |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1043-| Dst daemon becomes a network client to attacker-supplied source URIs (network-pivot/SSRF risk) | §4.3 delegation gate: default-disabled, host allowlist with strict matching semantics (§4.3.3), DNS-rebinding mitigation by binding the connection to the resolved IP. Per-module narrowing override. Documented in `DAEMON_CONFIG.md`. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1044-| Spec-construction drift between CLI `pull_sync` and daemon `delegated_pull` paths | §4.2 refactor extracts `pull_sync_with_spec`; both CLI and daemon use the same target-side pull body. Wire-equivalence unit test guards the seam (R23-F1). The seam includes the endpoint→spec mapping at `pull.rs:397-409`; `pull_sync_with_spec` is contractually forbidden to read `self.endpoint.path`, with an endpoint-isolation unit test guarding the boundary (R25-F1). |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1045-| Allowlist matching gotchas (DNS aliases, CIDR off-by-one, IPv6 forms, rebinding) | §4.3.3 specifies exact semantics; Phase 1 unit-test list covers each form including DNS-rebinding simulation (R23-F3). Loopback/link-local addresses additionally require IP- or CIDR-form authorization, never hostname-only (R25-F3) — closes the SSRF-via-DNS pivot. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1046-| CLI claims destination's capabilities incorrectly (e.g., asserts dst supports tar shards when it doesn't) | `client_capabilities` is the one spec field where CLI-supplied values are non-authoritative. Dst handler mandatorily replaces it with own `PeerCapabilities` before outbound connect (§4.2 step 8). Mandatory-override unit test guards the boundary (R25-F2). |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1047-| Progress event volume grows (every delegated pull pushes events) | Apply existing `RemoteTransferProgress` throttling; same as a normal pull. Bounded channel + stream backpressure handle overload. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1048-| Operator runs `blit copy` against three daemons (A→B and B→C in same script): does B as dst handle re-entry as src cleanly? | B is just a daemon, no special state. Two delegated pulls can land on it concurrently; metrics gauge counts both. Document as supported. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1049-| Dst aborts but src has already buffered chunks | Same failure mode as today's pull. Existing `pull_sync_with_spec` cleanup covers it. |
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md-1050-| **Open:** does `--checksum` work end-to-end? | The dst-as-initiator pull semantics are identical to a direct pull. F11/R15-F1 ack negotiation lives in `pull.rs` — should work unchanged. Verified in integration test. |
--
docs/plan/UNIFIED_TRANSFER_ENGINE.md-219-ceiling, and the tuner adjusts **live from the first byte** as PR1
docs/plan/UNIFIED_TRANSFER_ENGINE.md-220-telemetry streams in. The staging is by *what gets adjusted*, not by a
docs/plan/UNIFIED_TRANSFER_ENGINE.md-221-probe-to-continuous progression:
docs/plan/UNIFIED_TRANSFER_ENGINE.md-222-
docs/plan/UNIFIED_TRANSFER_ENGINE.md-223-- **`ue-1b` — cheap dials live:** chunk size, prefetch, TCP buffers move
docs/plan/UNIFIED_TRANSFER_ENGINE.md-224-  in response to in-flight telemetry, within the receiver's ceiling. This
docs/plan/UNIFIED_TRANSFER_ENGINE.md-225-  already delivers "tuned live" — no static table remains.
docs/plan/UNIFIED_TRANSFER_ENGINE.md-226-- **`ue-2` — stream count live (in scope at Active):** mid-transfer
docs/plan/UNIFIED_TRANSFER_ENGINE.md:227:  add/drop of streams via PR3's `DataPlaneResize`/`Ack`, riding the
docs/plan/UNIFIED_TRANSFER_ENGINE.md-228-  elastic work-stealing stream-set from PR2. This is the genuinely hard
docs/plan/UNIFIED_TRANSFER_ENGINE.md-229-  piece; it is sequenced after the foundation slices because it needs the
docs/plan/UNIFIED_TRANSFER_ENGINE.md-230-  converged engine + the finished resize protocol, not because it is
docs/plan/UNIFIED_TRANSFER_ENGINE.md-231-  optional.
docs/plan/UNIFIED_TRANSFER_ENGINE.md-232-
docs/plan/UNIFIED_TRANSFER_ENGINE.md-233-The two expensive-to-retrofit pieces — a mutable dial read by both ends,
docs/plan/UNIFIED_TRANSFER_ENGINE.md-234-and an elastic stream-set — exist from `ue-1b`. That is the answer to
docs/plan/UNIFIED_TRANSFER_ENGINE.md-235-"does starting simple paint us into a corner for continuous?": **no**,
--
docs/plan/UNIFIED_TRANSFER_ENGINE.md-286-   engine-internal behavior, not the H10b concept (D-2026-06-20-3). Verify
docs/plan/UNIFIED_TRANSFER_ENGINE.md-287-   the one-entry property + loopback parity band.
docs/plan/UNIFIED_TRANSFER_ENGINE.md-288-4. **`ue-1d-pull-multistream`** — pull gains multi-stream through the
docs/plan/UNIFIED_TRANSFER_ENGINE.md-289-   unified sequencer (the w2-3 goal, now via the engine not a path-specific
docs/plan/UNIFIED_TRANSFER_ENGINE.md-290-   hack). Absorbs `MULTISTREAM_PULL.md` acceptance criteria: negotiation,
docs/plan/UNIFIED_TRANSFER_ENGINE.md-291-   per-stream failure, cancellation mid-transfer, old↔new compat.
docs/plan/UNIFIED_TRANSFER_ENGINE.md-292-5. **`ue-1e-delete-pull-rpc`** — w2-4: delete the deprecated Pull RPC now
docs/plan/UNIFIED_TRANSFER_ENGINE.md-293-   that its multi-stream pattern is harvested into the engine.
docs/plan/UNIFIED_TRANSFER_ENGINE.md:294:6. **`ue-2-stream-resize`** — wire PR3's `DataPlaneResize`/`Ack`; add/drop
docs/plan/UNIFIED_TRANSFER_ENGINE.md-295-   streams mid-transfer from live telemetry, riding the elastic
docs/plan/UNIFIED_TRANSFER_ENGINE.md-296-   work-stealing stream-set. **In scope at Active** (owner: 11 months of
docs/plan/UNIFIED_TRANSFER_ENGINE.md-297-   benchmarking is the justification); 10 GbE is the sign-off measure, not
docs/plan/UNIFIED_TRANSFER_ENGINE.md-298-   a gate. Sequenced after `ue-1c` because it needs the one engine.
docs/plan/UNIFIED_TRANSFER_ENGINE.md-299-
docs/plan/UNIFIED_TRANSFER_ENGINE.md-300-## Open questions
docs/plan/UNIFIED_TRANSFER_ENGINE.md-301-
docs/plan/UNIFIED_TRANSFER_ENGINE.md-302-- **(RESOLVED — q1)** Small-transfer threshold — **obviated.** No probe
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-75-      delegated daemon-to-daemon transfers.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-76-- [ ] Existing local fast paths are either engine-owned strategies
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-77-      (`journal_skip`, `single_file`, `tiny_manifest`, `single_huge_file`)
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-78-      or explicitly deleted by owner decision. No local path bypasses the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-79-      transfer behavior owner by accident.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-80-- [ ] The static stream/dial sources are replaced by one dial source.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-81-      (Corrected against code: today there are **two static tables, not
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-82-      three** — the client-side `remote/tuning.rs::determine_remote_tuning`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:83:      ladder, whose `initial_streams`/`max_streams`/`prefetch_count`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-84-      drive both local and push; and the daemon-side
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-85-      `DataTransferNegotiation.stream_count` (proto field 4). Pull is
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-86-      single-stream today via the `force_grpc` single-file path, not a
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-87-      third ladder. The earlier "push/control.rs::desired_streams" and
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-88-      "pull.rs::pull_stream_count" references were stale; both paths
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-89-      consume `determine_remote_tuning`.)
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-90-- [ ] The engine starts transfer work within about 1 second without a
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-91-      probe-then-go phase. This holds for **both** novel workloads (no
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-261-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-262-- append `CapacityProfile receiver_capacity = 11` to
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-263-  `DataTransferNegotiation` rather than using reserved RDMA fields 5–10
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-264-  (field 11 is the first free number after the reservation);
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-265-- add a capacity profile to the request/setup side where the receiver is
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-266-  the client, especially PullSync and delegated pull;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-267-- add explicit peer capability bits/fields so resize messages are never
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-268-  sent to an old peer;
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:269:- add `DataPlaneResize` and `DataPlaneResizeAck` as negotiated control
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-270-  messages in the relevant control streams, not as blind TCP data-plane
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-271-  records.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-272-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-273-Exact field names and numbers are part of the wire slice acceptance
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-274-criteria. Old peers must see current behavior: no capacity profile means
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-275-use today's static/conservative behavior; no resize support means no
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-276-mid-transfer add/drop.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-277-
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-332-1. **`ue-r2-1a-salvage-substrate`** — Cherry-pick adaptive PR1+PR2 up to
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-333-   `eafb187`, excluding `d9d4ec7`. Resolve `data_plane.rs`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-334-   StallGuard-vs-`Probe`. Treat work-stealing as behavior, not inert
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-335-   substrate: add/keep slow-sink, failing-sink, cancellation,
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-336-   byte-accounting, StallGuard, and byte-identical tests. The elastic
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-337-   work-stealing stream-set exists from this slice onward (C-ready seam).
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-338-2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-339-   capability, and resize proto shape (`receiver_capacity = 11`,
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:340:   `DataPlaneResize`/`Ack`). Add compatibility tests for old client/new
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-341-   daemon and new client/old daemon. No behavior depends on these fields
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-342-   until this slice is green.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-343-3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-344-   convert `TransferOrchestrator` into a local adapter. Move local fast
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-345-   paths under engine-owned strategies, preserving behavior and accounting.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-346-4. **`ue-r2-1d-streaming-plan-foundation`** — Introduce partial-scan
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-347-   initial plans and plan updates. Prove first-byte / first-useful-work
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-348-   timing for local and push shapes, and document any RELIABLE exception
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-358-7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-359-   engine and make pull multistream there. Preserve resume, checksum
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-360-   refusal, delete-list authority, cancellation, per-stream failure, and
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-361-   gRPC fallback behavior.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-362-8. **`ue-r2-1h-delete-deprecated-pull-rpc`** — Delete deprecated `Pull`
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-363-   after PullSync has harvested the needed multistream/fallback pattern
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-364-   and tests cover the replacement, including old/new peer pairs.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-365-9. **`ue-r2-2-stream-resize`** — Finish negotiated
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:366:   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-367-   transfer from live telemetry, using the elastic work queue from
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-368-   `ue-r2-1a`. Wires onto the already-mutable dial and elastic
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-369-   stream-set — a wire-up, not a restructuring.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-370-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-371-### Slice dependencies
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-372-
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-373-Explicit blocking, since REV3 has nine slices and ordering matters:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md-374-
--
docs/plan/OTP11_LOCAL_SESSION.md-13-## Why this doc
docs/plan/OTP11_LOCAL_SESSION.md-14-
docs/plan/OTP11_LOCAL_SESSION.md-15-otp-11 deletes the largest surviving old-path block (~4.8k LOC: `orchestrator/`,
docs/plan/OTP11_LOCAL_SESSION.md-16-`engine/` minus dial, `local_worker.rs`, `auto_tune/`, `change_journal/`) and
docs/plan/OTP11_LOCAL_SESSION.md-17-re-routes three frontends (CLI, TUI, blit-app) at once, near the otp-13 test
docs/plan/OTP11_LOCAL_SESSION.md-18-floor (suite 1488 vs ≥1483). Three design decisions were not settled by the
docs/plan/OTP11_LOCAL_SESSION.md-19-parent plan and are fixed here so implementation is transcription: the local
docs/plan/OTP11_LOCAL_SESSION.md-20-byte-carrier, the app-facing option/summary surface, and the fate of each
docs/plan/OTP11_LOCAL_SESSION.md:21:engine-only feature. Slice-level unknowns are delegated to the agent + codex
docs/plan/OTP11_LOCAL_SESSION.md-22-loop (parent plan §Open questions).
docs/plan/OTP11_LOCAL_SESSION.md-23-
docs/plan/OTP11_LOCAL_SESSION.md-24-## Current state (verified 2026-07-11, HEAD `d2bd843`)
docs/plan/OTP11_LOCAL_SESSION.md-25-
docs/plan/OTP11_LOCAL_SESSION.md-26-One chokepoint already exists: CLI (`crates/blit-cli/src/transfers/local.rs:125`)
docs/plan/OTP11_LOCAL_SESSION.md-27-and TUI (`crates/blit-tui/src/main.rs:4089,4161`) both call
docs/plan/OTP11_LOCAL_SESSION.md-28-`blit_app::transfers::local::run` (`crates/blit-app/src/transfers/local.rs:36`),
docs/plan/OTP11_LOCAL_SESSION.md-29-which `spawn_blocking`s into `TransferOrchestrator::execute_local_mirror`
--
docs/plan/WORKFLOW_PHASE_4.md-80-> capability probes) are deferred to 0.2.0 per
docs/plan/WORKFLOW_PHASE_4.md-81-> `RELEASE_PLAN_v2_2026-05-04.md` §3.3 (D6 owner sign-off).
docs/plan/WORKFLOW_PHASE_4.md-82-> `blit diagnostics dump` does include client-side probes today —
docs/plan/WORKFLOW_PHASE_4.md-83-> the deferred work is making the daemon do it too.
docs/plan/WORKFLOW_PHASE_4.md-84-
docs/plan/WORKFLOW_PHASE_4.md-85-| Task | Description | Deliverable |
docs/plan/WORKFLOW_PHASE_4.md-86-|------|-------------|-------------|
docs/plan/WORKFLOW_PHASE_4.md-87-| 4.8.1 | Build per-mount capability detector (reflink, sparse files, xattrs, checksum offload) with a curated FS-type table and on-demand probes. | `fs_capability` cache + planner-facing API. |
docs/plan/WORKFLOW_PHASE_4.md:88:| 4.8.2 | *(0.2.0)* Have `blit-daemon` probe during startup/idle windows and persist results per export; surface warnings for unknown filesystems with guidance to run profile. | Daemon capability cache + logs. |
docs/plan/WORKFLOW_PHASE_4.md-89-| 4.8.3 | *(0.2.0)* Extend `blit diagnostics profile` to run local probes and attach results to performance history/telemetry. | CLI profile output updated + docs. |
docs/plan/WORKFLOW_PHASE_4.md-90-
docs/plan/WORKFLOW_PHASE_4.md-91-### 4.9 Telemetry Intelligence Exploration
docs/plan/WORKFLOW_PHASE_4.md-92-
docs/plan/WORKFLOW_PHASE_4.md-93-> **Scope note (2026-05-13):** **Removed from project scope.**
docs/plan/WORKFLOW_PHASE_4.md-94-> Per `RELEASE_PLAN_v2_2026-05-04.md` §5.4 (owner decision
docs/plan/WORKFLOW_PHASE_4.md-95-> 2026-05-13), AI telemetry analysis is not on the roadmap.
docs/plan/WORKFLOW_PHASE_4.md-96-> Performance history will continue to be collected for the
--
docs/plan/OTP12_PERF_FINDINGS.md-188-  the epoch-0 topology **simultaneously**, so a positive result implicates
docs/plan/OTP12_PERF_FINDINGS.md-189-  *the topology pair*, not H1 specifically. It cannot distinguish
docs/plan/OTP12_PERF_FINDINGS.md-190-  source-accept serialization from synchronous destination dialing
docs/plan/OTP12_PERF_FINDINGS.md-191-  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
docs/plan/OTP12_PERF_FINDINGS.md-192-  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
docs/plan/OTP12_PERF_FINDINGS.md-193-  1. **dial/accept inversion** — same direction, same hosts, same fixture;
docs/plan/OTP12_PERF_FINDINGS.md-194-     only who dials changes. Implicates the topology pair (or exonerates it).
docs/plan/OTP12_PERF_FINDINGS.md-195-  2. **no-resize / pre-opened streams** — force the final stream count at
docs/plan/OTP12_PERF_FINDINGS.md:196:     epoch 0 so no resize epoch ever fires. If the gap survives with zero
docs/plan/OTP12_PERF_FINDINGS.md-197-     resizes, H1's resize-specific mechanism is **KILLED** regardless of
docs/plan/OTP12_PERF_FINDINGS.md-198-     what (1) shows (and note `dial.rs:474`: all three fixtures already
docs/plan/OTP12_PERF_FINDINGS.md-199-     target 8 streams, so resize *count* was never the discriminator).
docs/plan/OTP12_PERF_FINDINGS.md-200-  3. **per-side ordering** — hold the topology fixed and vary only whether
docs/plan/OTP12_PERF_FINDINGS.md-201-     the destination's dial-before-ACK is synchronous. Separates the two
docs/plan/OTP12_PERF_FINDINGS.md-202-     halves the inversion conflates.
docs/plan/OTP12_PERF_FINDINGS.md-203-  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
docs/plan/OTP12_PERF_FINDINGS.md-204-  across (1) AND survives (2); it is KILLED if the gap persists with no
--
docs/plan/OTP12_PERF_FINDINGS.md-349-and **a reference must share the MTU of the sessions graded against it.**
docs/plan/OTP12_PERF_FINDINGS.md-350-
docs/plan/OTP12_PERF_FINDINGS.md-351-**Do not over-read pf-0 here** (codex, 2026-07-14): the "3–4% faster at jumbo"
docs/plan/OTP12_PERF_FINDINGS.md-352-figure is **one cell (`wm_tcp_large`), one rig (W), both arms of the NEW build**.
docs/plan/OTP12_PERF_FINDINGS.md-353-pf-0 measured **no** small cells, **no** rig-Z cells, and **no** OLD-build MTU
docs/plan/OTP12_PERF_FINDINGS.md-354-response — so it does **not** quantify the leniency across the acceptance
docs/plan/OTP12_PERF_FINDINGS.md-355-matrices. What it establishes is that the mismatch is **real** and that MTU moves
docs/plan/OTP12_PERF_FINDINGS.md-356-wall time on at least one cell; a mismatched ceiling is therefore unsound in an
docs/plan/OTP12_PERF_FINDINGS.md:357:**unknown** direction, and lenient in the one direction actually measured.
docs/plan/OTP12_PERF_FINDINGS.md-358-
docs/plan/OTP12_PERF_FINDINGS.md-359-The resolution — re-record each rig's baseline at MTU 9000 and re-freeze —
docs/plan/OTP12_PERF_FINDINGS.md-360-carries a **non-loosening guard**, because a re-record also re-rolls hardware and
docs/plan/OTP12_PERF_FINDINGS.md-361-day state (rig W's Mac end is now `q`, not nagatha) and D2/F2 already forbids a
docs/plan/OTP12_PERF_FINDINGS.md-362-slower old rerun from loosening the bar: **the reference is the per-cell MINIMUM
docs/plan/OTP12_PERF_FINDINGS.md-363-of {2026-07-10 median, re-recorded 9000 median}; it can only tighten, and a cell
docs/plan/OTP12_PERF_FINDINGS.md-364-whose re-record is slower is flagged, never silently adopted.**
docs/plan/OTP12_PERF_FINDINGS.md-365-
--
docs/plan/OTP12_PERF_FINDINGS.md-375-Same-session references (`old_session`) are MTU-matched by construction and were
docs/plan/OTP12_PERF_FINDINGS.md-376-never at risk.
docs/plan/OTP12_PERF_FINDINGS.md-377-
docs/plan/OTP12_PERF_FINDINGS.md-378-## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
docs/plan/OTP12_PERF_FINDINGS.md-379-
docs/plan/OTP12_PERF_FINDINGS.md-380-- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
docs/plan/OTP12_PERF_FINDINGS.md-381-  connection-initiating end DIALS; byte direction is role-set
docs/plan/OTP12_PERF_FINDINGS.md-382-  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
docs/plan/OTP12_PERF_FINDINGS.md:383:  session the SOURCE is the responder: each sf-2 resize epoch is
docs/plan/OTP12_PERF_FINDINGS.md-384-  ACCEPTED off the source's listener while the DESTINATION dials
docs/plan/OTP12_PERF_FINDINGS.md-385-  (otp-5b-2: `SourceSockets` Dial/Accept branches;
docs/plan/OTP12_PERF_FINDINGS.md-386-  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
docs/plan/OTP12_PERF_FINDINGS.md-387-  accept/dial round-trips or serialization in the accept branch that the
docs/plan/OTP12_PERF_FINDINGS.md-388-  dial branch does not pay.
docs/plan/OTP12_PERF_FINDINGS.md-389-  **⚠ H1 ACCUSES CODE, NOT A PLATFORM (canonical; added 2026-07-14 after the
docs/plan/OTP12_PERF_FINDINGS.md-390-  shorthand misled two sessions).** The word "Windows" appears nowhere above.
docs/plan/OTP12_PERF_FINDINGS.md-391-  Windows is merely *who happens to be the accepting source* in P1's slow arm on
--
docs/plan/OTP12_PERF_FINDINGS.md-507-  counterfactual (a task-local/batch-inline path behind a debug flag).
docs/plan/OTP12_PERF_FINDINGS.md-508-  H7 and H6 are independent and may BOTH contribute.
docs/plan/OTP12_PERF_FINDINGS.md-509-
docs/plan/OTP12_PERF_FINDINGS.md-510-## Method (the investigation slice — no behavior changes)
docs/plan/OTP12_PERF_FINDINGS.md-511-
docs/plan/OTP12_PERF_FINDINGS.md-512-1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
docs/plan/OTP12_PERF_FINDINGS.md-513-   in-process/two-process rigs on the Mac with the otp-2 fixture
docs/plan/OTP12_PERF_FINDINGS.md-514-   shapes; `--trace-data-plane` + targeted `tracing` spans (added
docs/plan/OTP12_PERF_FINDINGS.md:515:   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
docs/plan/OTP12_PERF_FINDINGS.md-516-   ack), need-batch emission times, per-file sink open/write/close in
docs/plan/OTP12_PERF_FINDINGS.md-517-   the receive path, shard planner in/out timestamps.
docs/plan/OTP12_PERF_FINDINGS.md-518-2. **A/B the role layouts in one process**: the role suite already
docs/plan/OTP12_PERF_FINDINGS.md-519-   runs both initiator layouts over identical fixtures (otp-3) — but
docs/plan/OTP12_PERF_FINDINGS.md-520-   it forces the in-stream carrier (`transfer_session_roles.rs`), so
docs/plan/OTP12_PERF_FINDINGS.md-521-   the timing-harness variant MUST add a TCP-carrier mode; it reports
docs/plan/OTP12_PERF_FINDINGS.md-522-   phase timings per layout for mixed and small fixtures. A positive
docs/plan/OTP12_PERF_FINDINGS.md-523-   layout-dependent delta in a named phase confirms; local ABSENCE
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-106-      `single_huge_file`, and the single-file copy shortcut at
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-107-      `orchestrator.rs:178`) or explicitly deleted by owner decision. No
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-108-      local path bypasses the transfer behavior owner by accident.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-109-- [ ] **The three static code-level stream/dial ladders plus the
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-110-      negotiated proto field are replaced by one dial source** (corrected
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-111-      against code — see Current Code Reality). Concretely, the dial
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-112-      subsumes:
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-113-      1. `remote/tuning.rs::determine_remote_tuning` (size-keyed
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:114:         `initial_streams`/`max_streams`/`chunk_bytes`/`tcp_buffer_size`/
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-115-         `prefetch_count`; the *client's* ladder, consumed by push and by
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-116-         the daemon pull paths);
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-117-      2. `blit-daemon .../push/control.rs::desired_streams` (the daemon
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-118-         push-negotiation ladder, already keyed on **file count** as well
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-119-         as bytes — the daemon's ladder "wins" per `tuning.rs`'s own doc
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-120-         comment);
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-121-      3. `blit-daemon .../pull.rs::pull_stream_count` (the deprecated
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:122:         Pull RPC ladder, byte-keyed, capped by `tuning.max_streams`);
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-123-      and the negotiated `DataTransferNegotiation.stream_count` (proto
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-124-      field 4) those ladders feed onto the wire. After convergence no
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-125-      static size→streams table remains in any path.
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-126-- [ ] The engine starts transfer work within about 1 second without a
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-127-      probe-then-go phase. This holds for **both** novel workloads (no
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-128-      telemetry extant — start copying immediately at conservative
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-129-      defaults and tune live from the first byte) **and** known workloads
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-130-      (telemetry extant — replay the last run if it was optimal, else
--
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-219-     daemon `pull.rs:141/261`. Its own doc comment (`tuning.rs:7-10`):
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-220-     *"the daemon's push negotiation currently runs its own ladder and
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-221-     wins (single-owner consolidation is w2-2)."*
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-222-  2. `blit-daemon .../push/control.rs::desired_streams(files)` (:476,
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-223-     called :198/:267) — daemon push negotiation, keyed on **both**
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-224-     `total_bytes` and `file_count` (e.g. `file_count >= 200_000 → 16`).
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-225-  3. `blit-daemon .../pull.rs::pull_stream_count(total_bytes,
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md-226-     tuning_max)` (:904, called :167/:277) — deprecated Pull RPC,

exec
/bin/zsh -lc 'rg -n -C 10 "otp-12|worker|stream target|FAST|immediate|payload" docs/plan/ONE_TRANSFER_PATH.md docs/STATE.md REVIEW.md .review/findings/otp-12-worker-parity.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
.review/findings/otp-12-worker-parity.md:1:# otp-12-worker-parity — initiator-independent stream target
.review/findings/otp-12-worker-parity.md-2-
.review/findings/otp-12-worker-parity.md:3:**Slice**: ONE_TRANSFER_PATH otp-12 acceptance repair. The active plan
.review/findings/otp-12-worker-parity.md-4-requires one sender-owned, receiver-bounded stream policy for both role
.review/findings/otp-12-worker-parity.md:5:assignments; initiator/verb may not change the realized worker count.
.review/findings/otp-12-worker-parity.md-6-
.review/findings/otp-12-worker-parity.md-7-## What
.review/findings/otp-12-worker-parity.md-8-
.review/findings/otp-12-worker-parity.md-9-The unified session computed the same shape target in both orientations but
.review/findings/otp-12-worker-parity.md-10-did not guarantee reaching it. Resize advances one stream per epoch. Once
.review/findings/otp-12-worker-parity.md-11-`NeedComplete` arrived, the SOURCE resolved only the one epoch already in
.review/findings/otp-12-worker-parity.md-12-flight and stopped proposing. On the same 10,000-file fixture (shape target
.review/findings/otp-12-worker-parity.md-13-8), the source-initiator test settled at 3 streams and the
.review/findings/otp-12-worker-parity.md-14-destination-initiator test at 2.
.review/findings/otp-12-worker-parity.md-15-
.review/findings/otp-12-worker-parity.md-16-The destination-initiator admission side also interpreted an advertised
.review/findings/otp-12-worker-parity.md-17-`max_streams = 0` as a one-stream ceiling, while the SOURCE dial correctly
.review/findings/otp-12-worker-parity.md-18-interpreted the wire value as unknown/default. That was a role-specific cap.
.review/findings/otp-12-worker-parity.md-19-
.review/findings/otp-12-worker-parity.md-20-## Approach
.review/findings/otp-12-worker-parity.md-21-
.review/findings/otp-12-worker-parity.md:22:- Before each payload batch enters the shared elastic send pipeline, drive
.review/findings/otp-12-worker-parity.md-23-  the existing one-stream-per-epoch resize protocol until the currently known
.review/findings/otp-12-worker-parity.md-24-  shape target is settled. Needs and resume hashes continue to be processed
.review/findings/otp-12-worker-parity.md-25-  while acknowledgements are in flight, so the target incorporates all work
.review/findings/otp-12-worker-parity.md-26-  learned during the ramp.
.review/findings/otp-12-worker-parity.md-27-- Stop a refused ramp instead of retrying the same unattainable target under
.review/findings/otp-12-worker-parity.md-28-  fresh epochs forever.
.review/findings/otp-12-worker-parity.md-29-- Centralize receiver stream-ceiling resolution in `dial.rs` and use it on
.review/findings/otp-12-worker-parity.md-30-  both the SOURCE dial and destination-initiator admission path. Wire value
.review/findings/otp-12-worker-parity.md-31-  zero remains unknown/default, never one.
.review/findings/otp-12-worker-parity.md-32-- Strengthen both role-orientation integration pins from merely `> 1` to the
--
.review/findings/otp-12-worker-parity.md-49-  `transfer_session_roles` integration target passes 39/39.
.review/findings/otp-12-worker-parity.md-50-- Full workspace gate passes: `cargo fmt --all -- --check`,
.review/findings/otp-12-worker-parity.md-51-  `cargo clippy --workspace --all-targets -- -D warnings`, and
.review/findings/otp-12-worker-parity.md-52-  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
.review/findings/otp-12-worker-parity.md-53-
.review/findings/otp-12-worker-parity.md-54-## Known gaps
.review/findings/otp-12-worker-parity.md-55-
.review/findings/otp-12-worker-parity.md-56-- Socket acquisition remains connection-role-specific by design: the network
.review/findings/otp-12-worker-parity.md-57-  initiator dials the responder so a pull caller does not need an inbound
.review/findings/otp-12-worker-parity.md-58-  listener through NAT/firewalls. Byte work is still one SOURCE send pipeline
.review/findings/otp-12-worker-parity.md:59:  and one DESTINATION receive pipeline. This slice removes worker-count drift;
.review/findings/otp-12-worker-parity.md-60-  it does not invert that network topology.
.review/findings/otp-12-worker-parity.md:61:- No hardware benchmark is part of this code slice. The existing otp-12
.review/findings/otp-12-worker-parity.md-62-  acceptance rigs remain the performance proof after review.
.review/findings/otp-12-worker-parity.md-63-
.review/findings/otp-12-worker-parity.md-64-## Reviewer comments
.review/findings/otp-12-worker-parity.md-65-
.review/findings/otp-12-worker-parity.md-66-(appended after the codex round)
--
docs/plan/ONE_TRANSFER_PATH.md-67-  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
docs/plan/ONE_TRANSFER_PATH.md-68-  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
docs/plan/ONE_TRANSFER_PATH.md-69-  after cutover, not one of this plan's slices — see the Design note
docs/plan/ONE_TRANSFER_PATH.md-70-  on the write-strategy seam. One narrow owner-granted exception
docs/plan/ONE_TRANSFER_PATH.md-71-  (D-2026-07-09-1, otp-7b): the CLI end-of-operation fault summary —
docs/plan/ONE_TRANSFER_PATH.md-72-  name the file(s) a session fault affected and suggest a re-run —
docs/plan/ONE_TRANSFER_PATH.md-73-  lands inside otp-7. Nothing else new rides this plan.
docs/plan/ONE_TRANSFER_PATH.md-74-
docs/plan/ONE_TRANSFER_PATH.md-75-## Constraints
docs/plan/ONE_TRANSFER_PATH.md-76-
docs/plan/ONE_TRANSFER_PATH.md:77:- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
docs/plan/ONE_TRANSFER_PATH.md-78-  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
docs/plan/ONE_TRANSFER_PATH.md-79-  violated at the choreography layer.
docs/plan/ONE_TRANSFER_PATH.md-80-- **Converge up, not down**: per benchmark cell, the unified session
docs/plan/ONE_TRANSFER_PATH.md-81-  must match the better of today's two directions (within ±10% run
docs/plan/ONE_TRANSFER_PATH.md-82-  noise), not their average. Unification that slows the fast
docs/plan/ONE_TRANSFER_PATH.md-83-  direction fails review.
docs/plan/ONE_TRANSFER_PATH.md-84-- REV4 invariants carry: byte-identical results, StallGuard,
docs/plan/ONE_TRANSFER_PATH.md-85-  cancellation, byte-accounting. Existing pins are ported (not
docs/plan/ONE_TRANSFER_PATH.md-86-  dropped) as tests become role-parameterized; test count never
docs/plan/ONE_TRANSFER_PATH.md-87-  drops.
--
docs/plan/ONE_TRANSFER_PATH.md-130-      misses this bar only by a discriminator-attributed destination
docs/plan/ONE_TRANSFER_PATH.md-131-      write-path residue counts as satisfied — D-2026-07-12-1;
docs/plan/ONE_TRANSFER_PATH.md-132-      `docs/plan/OTP12_ACCEPTANCE_RUN.md` D2.)
docs/plan/ONE_TRANSFER_PATH.md-133-- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
docs/plan/ONE_TRANSFER_PATH.md-134-      (driver), daemon `push/control.rs` choreography, daemon
docs/plan/ONE_TRANSFER_PATH.md-135-      `pull_sync.rs` choreography, the delegated-pull driver, the
docs/plan/ONE_TRANSFER_PATH.md-136-      separate local orchestration path, and the `Push`/`PullSync`
docs/plan/ONE_TRANSFER_PATH.md-137-      RPCs no longer exist in the tree; one `TransferSession` and one
docs/plan/ONE_TRANSFER_PATH.md-138-      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
docs/plan/ONE_TRANSFER_PATH.md-139-      as trigger + progress relay — the proof must show it carries no
docs/plan/ONE_TRANSFER_PATH.md:140:      payload bytes (codex F3). Recorded file-by-file in the final
docs/plan/ONE_TRANSFER_PATH.md-141-      slice's finding doc.
docs/plan/ONE_TRANSFER_PATH.md-142-- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
docs/plan/ONE_TRANSFER_PATH.md-143-      guard), filters, block-resume, gRPC fallback carrier, delegated
docs/plan/ONE_TRANSFER_PATH.md-144-      transfer, progress events, jobs/cancel, read-only enforcement —
docs/plan/ONE_TRANSFER_PATH.md-145-      each demonstrated by ported tests on the session.
docs/plan/ONE_TRANSFER_PATH.md-146-- [ ] Suite green throughout; final test count ≥ pre-plan baseline
docs/plan/ONE_TRANSFER_PATH.md-147-      (1483); all REV4 invariant pins and the sf-2 pin pass
docs/plan/ONE_TRANSFER_PATH.md-148-      role-parameterized.
docs/plan/ONE_TRANSFER_PATH.md-149-- [ ] Benchmark methodology corrected and recorded: symmetric-fs
docs/plan/ONE_TRANSFER_PATH.md-150-      cells are the verdict cells; tmpfs cells remain only as
--
docs/plan/ONE_TRANSFER_PATH.md-159-diff planner, tar-shard, stall guard, progress, `operation_spec` (the
docs/plan/ONE_TRANSFER_PATH.md-160-REV4 unified contract), and the engine dial (stream policy incl. sf-2
docs/plan/ONE_TRANSFER_PATH.md-161-shape correction). The defect layer is above it: four driver loops
docs/plan/ONE_TRANSFER_PATH.md-162-choreograph these pieces differently per direction.
docs/plan/ONE_TRANSFER_PATH.md-163-
docs/plan/ONE_TRANSFER_PATH.md-164-**The one choreography** (roles, not directions):
docs/plan/ONE_TRANSFER_PATH.md-165-
docs/plan/ONE_TRANSFER_PATH.md-166-1. Initiator opens the single bidi `Transfer` RPC and sends the
docs/plan/ONE_TRANSFER_PATH.md-167-   operation spec: which end is SOURCE, which is DESTINATION, path/
docs/plan/ONE_TRANSFER_PATH.md-168-   module, filters, mirror/resume flags, capabilities.
docs/plan/ONE_TRANSFER_PATH.md:169:2. SOURCE enumerates and **streams** its manifest immediately (no
docs/plan/ONE_TRANSFER_PATH.md-170-   buffered-enumeration phase — this generalizes push's fast start;
docs/plan/ONE_TRANSFER_PATH.md-171-   pull's full-enumeration-then-negotiate slow start is deleted, which
docs/plan/ONE_TRANSFER_PATH.md-172-   absorbs the "pull 1s-start" residue item).
docs/plan/ONE_TRANSFER_PATH.md-173-3. DESTINATION diffs incrementally against its own filesystem and
docs/plan/ONE_TRANSFER_PATH.md-174-   returns need-list batches (one diff owner, always the end that
docs/plan/ONE_TRANSFER_PATH.md-175-   owns the target fs — push's proven model; pull_sync's
docs/plan/ONE_TRANSFER_PATH.md-176-   source-side diff is deleted).
docs/plan/ONE_TRANSFER_PATH.md:177:4. The data plane opens at the dial floor immediately; stream count
docs/plan/ONE_TRANSFER_PATH.md-178-   shape-corrects as the need list accumulates (sf-2 mechanism, now
docs/plan/ONE_TRANSFER_PATH.md-179-   the only policy, both roles).
docs/plan/ONE_TRANSFER_PATH.md:180:5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
docs/plan/ONE_TRANSFER_PATH.md-181-   the one pipeline into the data plane; DESTINATION writes through
docs/plan/ONE_TRANSFER_PATH.md-182-   the one receive path. The receive sink is built with a
docs/plan/ONE_TRANSFER_PATH.md-183-   **runtime-selected write-strategy seam**: buffered relay is the
docs/plan/ONE_TRANSFER_PATH.md-184-   universal strategy; capability-gated alternatives slot in behind
docs/plan/ONE_TRANSFER_PATH.md-185-   it without new paths — the first is zero-copy/splice
docs/plan/ONE_TRANSFER_PATH.md-186-   (D-2026-07-05-3, unparked for CPU-bound receivers like the
docs/plan/ONE_TRANSFER_PATH.md-187-   owner's UNAS 8 Pro; design input:
docs/plan/ONE_TRANSFER_PATH.md:188:   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
docs/plan/ONE_TRANSFER_PATH.md-189-   follow-on slice set after cutover. Strategy selection reads
docs/plan/ONE_TRANSFER_PATH.md:190:   capability and payload type, never role or initiator.
docs/plan/ONE_TRANSFER_PATH.md-191-6. Mirror: DESTINATION computes deletions from the completed source
docs/plan/ONE_TRANSFER_PATH.md-192-   manifest it received (filter-scoped, scan-complete-guarded) and
docs/plan/ONE_TRANSFER_PATH.md-193-   executes them locally. One rule, no per-direction delete
docs/plan/ONE_TRANSFER_PATH.md-194-   choreography.
docs/plan/ONE_TRANSFER_PATH.md-195-7. Resume: optional block-hash phase inside the same session, same
docs/plan/ONE_TRANSFER_PATH.md-196-   messages regardless of roles.
docs/plan/ONE_TRANSFER_PATH.md-197-8. Summary/byte-accounting: one record shape.
docs/plan/ONE_TRANSFER_PATH.md-198-
docs/plan/ONE_TRANSFER_PATH.md-199-**Transport facts vs choreography**: the connection-initiating end
docs/plan/ONE_TRANSFER_PATH.md-200-dials TCP data-plane sockets (NAT reality) — byte direction within a
docs/plan/ONE_TRANSFER_PATH.md-201-socket is set by role, not by who dialed. The gRPC-fallback lane
docs/plan/ONE_TRANSFER_PATH.md-202-becomes a *byte-carrier option* inside the same session (control-
docs/plan/ONE_TRANSFER_PATH.md-203-stream frames instead of TCP sockets), selected at negotiation — not
docs/plan/ONE_TRANSFER_PATH.md-204-a separate transfer path. Resize keeps its controller-at-sender rule.
docs/plan/ONE_TRANSFER_PATH.md-205-
docs/plan/ONE_TRANSFER_PATH.md-206-**Delegated transfer**: a daemon receiving a delegated request simply
docs/plan/ONE_TRANSFER_PATH.md-207-becomes an initiator of the same session against the other daemon
docs/plan/ONE_TRANSFER_PATH.md-208-(destination role on its module fs). The bespoke delegated-pull
docs/plan/ONE_TRANSFER_PATH.md-209-driver is deleted; the delegation *gate* (authorization) stays. The
docs/plan/ONE_TRANSFER_PATH.md-210-`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
docs/plan/ONE_TRANSFER_PATH.md:211:(`DelegatedPullProgress` stream) — it never carries payload bytes;
docs/plan/ONE_TRANSFER_PATH.md-212-its handler shrinks to "authorize, spawn the session, relay the
docs/plan/ONE_TRANSFER_PATH.md-213-session's progress events." It stays wire-compatible or is folded at
docs/plan/ONE_TRANSFER_PATH.md-214-cutover — either way the deletion proof asserts no bytes flow
docs/plan/ONE_TRANSFER_PATH.md-215-through it (codex F3).
docs/plan/ONE_TRANSFER_PATH.md-216-
docs/plan/ONE_TRANSFER_PATH.md-217-**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
docs/plan/ONE_TRANSFER_PATH.md-218-a strictly-ordered block-hash exchange — the DESTINATION's block map
docs/plan/ONE_TRANSFER_PATH.md-219-for a file must complete before the SOURCE sends any block of that
docs/plan/ONE_TRANSFER_PATH.md-220-file, and stale/mismatched partials fall back to full-file transfer.
docs/plan/ONE_TRANSFER_PATH.md:221:This is an explicit exception to the immediate-start rule, exactly as
docs/plan/ONE_TRANSFER_PATH.md-222-today's resume path is an explicit single-stream RELIABLE exception
docs/plan/ONE_TRANSFER_PATH.md-223-(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
docs/plan/ONE_TRANSFER_PATH.md-224-contract; otp-7 pins the stale-partial and mid-resume-failure cases
docs/plan/ONE_TRANSFER_PATH.md-225-in tests.
docs/plan/ONE_TRANSFER_PATH.md-226-
docs/plan/ONE_TRANSFER_PATH.md-227-**Local transfers**: the same session driver over an in-process
docs/plan/ONE_TRANSFER_PATH.md-228-transport (both roles in one process, no wire). The engine underneath
docs/plan/ONE_TRANSFER_PATH.md-229-is already shared; the separate local orchestration path is deleted
docs/plan/ONE_TRANSFER_PATH.md-230-in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
docs/plan/ONE_TRANSFER_PATH.md-231-guard the migration.
--
docs/plan/ONE_TRANSFER_PATH.md-292-8. **otp-8 fallback byte-carrier** (control-stream frames) as the
docs/plan/ONE_TRANSFER_PATH.md-293-   session's alternate transport.
docs/plan/ONE_TRANSFER_PATH.md-294-9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
docs/plan/ONE_TRANSFER_PATH.md-295-   delegated-pull driver retired behind the existing gate;
docs/plan/ONE_TRANSFER_PATH.md-296-   `DelegatedPull` RPC reduced to trigger + progress relay.
docs/plan/ONE_TRANSFER_PATH.md-297-10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
docs/plan/ONE_TRANSFER_PATH.md-298-    operation through the session; `Push`/`PullSync` and all four
docs/plan/ONE_TRANSFER_PATH.md-299-    drivers deleted from the tree and the proto, no bridge
docs/plan/ONE_TRANSFER_PATH.md-300-    (D-2026-07-05-2); ported-test accounting proves count never
docs/plan/ONE_TRANSFER_PATH.md-301-    dropped. Deletion proof recorded, incl. the DelegatedPull
docs/plan/ONE_TRANSFER_PATH.md:302:    no-payload-bytes assertion.
docs/plan/ONE_TRANSFER_PATH.md-303-11. **otp-11 local transfers** ride the in-process transport; the
docs/plan/ONE_TRANSFER_PATH.md-304-    separate local orchestration is deleted; local perf pins hold.
docs/plan/ONE_TRANSFER_PATH.md:305:12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
docs/plan/ONE_TRANSFER_PATH.md-306-    on the unified path — initiator/verb invariance A/B within noise
docs/plan/ONE_TRANSFER_PATH.md-307-    AND every cell ≤ the better old direction + noise; committed as
docs/plan/ONE_TRANSFER_PATH.md-308-    this plan's acceptance evidence.
docs/plan/ONE_TRANSFER_PATH.md-309-13. **otp-13 verdict**: acceptance checklist walked with the owner;
docs/plan/ONE_TRANSFER_PATH.md-310-    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
docs/plan/ONE_TRANSFER_PATH.md-311-    against the unified baseline — owner call at that point.
docs/plan/ONE_TRANSFER_PATH.md-312-
docs/plan/ONE_TRANSFER_PATH.md-313-## Open questions
docs/plan/ONE_TRANSFER_PATH.md-314-
docs/plan/ONE_TRANSFER_PATH.md-315-- None requiring owner input now — scope, wire, and process were
--
docs/STATE.md-1-# STATE — single entry point for "what is true right now"
docs/STATE.md-2-
docs/STATE.md-3-Last updated: 2026-07-15 (52nd handoff — round 11 fixed + round-12 consensus: P1 IS REAL, the Mac↔Mac run is parked; owner to pick direction)
docs/STATE.md-4-
docs/STATE.md-5-- **NEXT ACTION — OWNER DECISION, then execute it. NO DATA HAS EVER BEEN TAKEN and none is queued.** Round 11 is fully fixed (instrument at `bfae311`, prereg **rev 11**), and the round-12 review — reframed per **D-2026-07-14-5** to ask "is this the best experiment", not "is the code correct per my plan" — reached a **two-reviewer consensus that changes the plan**: read `.review/results/macmac-r12.{codex-design,codex-harness,grok-design}.md` and `.review/results/p1-adjudication-r1.{codex,grok}.md`.
docs/STATE.md:6:  - **P1 IS REAL — settled by independent adjudication of the RECORDED data (codex + grok, high confidence).** A prior review claimed P1 might be a free-writeback timing artifact of the old harness (`bench_otp12_win.sh` flushes with no settle) and should be re-measured first. **The data refute that:** on `wm_tcp_mixed` the flush is **symmetric** (72 vs 73 ms) against a **~300 ms** effect, the effect is entirely in **transfer time** (remove flush and the ratio *rises*, 1.385→1.417, with zero arm overlap), the **same-fixture gRPC control passes at 1.020** (a writeback artifact would hit it identically), and Linux's identical immediate-flush method shows **no P1**. The precedent both cite: a *real* accounting artifact was caught here once (`2c0af86`) because it polluted the gRPC control — P1 is carrier-specific, so it passes that test. **The release blocker is genuine, not measurement error.**
docs/STATE.md-7-  - **BOTH REVIEWERS: the Mac↔Mac run is NOT the next move — no outcome of it changes the release-critical action.** It answers only "can P1 occur without a Windows peer?", and every outcome still routes to fixing P1 on the pair where it lives (macOS↔Windows). Grok's power analysis: with four independent full-range controls that must ALL be clean and rig W's fast arm known-bimodal, the *most likely* successful outcome is `CONTROLS-NOT-CLEAN` — a re-run, not an answer.
docs/STATE.md-8-  - **THE DECISION FOR THE OWNER**: (1) **instrument the TCP dial/accept transfer path on rig W** — both reviewers' recommendation; P1 is now pinned to TCP + destination-initiated + mixed, so add timing spans to `SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`; the fastest route to a fix. Or (2) **run Mac↔Mac anyway** for the 2×2 map — instrument is READY (BLOCKER closed and proved: pointing it at the 1GbE NIC trips three independent gates), but see the consensus above. **No agent may pick this; owner call.**
docs/STATE.md-9-  - **The Mac↔Mac instrument is DONE and REVIEWED** — engine 40 cases / 19 mutations, harness self-test 0-blind on both Macs, fabric gates proved by mutation. If run: nagatha↔`q`, 10GbE MTU 9000, build `f35702a` (nagatha's worktree + build were MISSING and were rebuilt this session), both Macs codex-quiet and Time Machine off. Host facts: `.agents/machines.md`.
docs/STATE.md-10-  - **⚠ ROUND-12 STILL-OPEN correctness findings (real, not yet fixed — apply before any Mac↔Mac run):** the threshold `min(src/10, 230)` can report `REPRODUCES` on a cell whose ratio (1.092) *passes* the 1.10 bar (codex BLOCKER — the `min` gives EITHER standard, the prose says BOTH); the end-fabric gate re-checks MSS/IP but **not link speed** (a 10GbE→1GbE renegotiation keeping MTU 9000 grades — my own duplicate-site bug); the `B ≥ T/2` refusal guards only the positive margin, not the smaller `src/11` negative one; two mutations "kill" for the wrong reason. Detail in `macmac-r12.codex-design.md`.
docs/STATE.md-11-- **THE INSTRUMENT IS THE RISK — ~110 findings across TEN reviews of this ONE harness, all accepted, none rejected, and it has still never run.** Three project claims were already retracted to harness bugs. **TWO DEFECT CLASSES recur in EVERY round; the next review must assume both are present.** (1) **"Fixed the branch I was shown, not the class"** — the same materiality bug escaped **four** rounds; a fail-open `pgrep` was fixed in one gate and left in its duplicate; the drain was fixed by VALUE and left failing by STATUS; Spotlight coerced a non-number to 0 exactly as the drain once accepted `"."`. **And a deletion regressed the build pin**: cutting the escalation block out took the adjacent `EXPECT_SHA` check with it, so any sha — including `.dirty` — was accepted. (2) **"A protection that never executes, or cannot fail"** — `SETTLE_MS` **had never run in any revision** (a quoting bug killed the `sleep` and its status was discarded), while the prereg asserted it for three revisions; the ssh-dispatch **bound** was measured once at preflight and never enforced on a run. Earned rules: **verify the instrument before believing the measurement**; **`bash -n` is not an execution**; **a protection that cannot be observed is not a protection**; **a mutation that cannot be killed is not a proof.**
docs/STATE.md-12-- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** "Reproduces ⇒ H1 dies" was **WRONG**: H1 accuses **blit's own code paths**, not Windows, and that code runs on macOS too — so a reproduction is *consistent with* H1. It answers one thing, scoped to this pair: **can P1 occur WITHOUT a Windows peer?** A reproduction ⇒ P1 is not waivable as "Windows residue" (it does **not** prove a platform-*general* cost, and leaves macOS/APFS and host×role open). A null ⇒ it did not reproduce *on this pair* — consistent with "Windows required", **not proof** of it, and reportable only if the run could have SEEN the effect. Detail: the pre-registration.
docs/STATE.md-13-- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
docs/STATE.md-14-- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
docs/STATE.md:15:- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
docs/STATE.md-16-- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
docs/STATE.md-17-- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
docs/STATE.md:18:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
docs/STATE.md-19-- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
docs/STATE.md-20-- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**
docs/STATE.md-21-
docs/STATE.md-22-Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
docs/STATE.md-23-≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
docs/STATE.md-24-procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
docs/STATE.md-25-
docs/STATE.md-26-## Now (active work)
docs/STATE.md-27-
docs/STATE.md-28-- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
--
docs/STATE.md-32-  because the per-direction drivers and `Push`/`PullSync` are deleted
docs/STATE.md-33-  at cutover. Slices otp-1..13; converge-up per cell (±10%);
docs/STATE.md-34-  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
docs/STATE.md-35-  same-build peers only, refusal at session open.**
docs/STATE.md-36-  - **Slices otp-1 … otp-11 are all `[x]` CLOSED** — the session
docs/STATE.md-37-    machine, the baselines, the cutover deletion (−13.8k lines) and
docs/STATE.md-38-    otp-11b's deletion of the old orchestration (−6.2k). The
docs/STATE.md-39-    deletion-proof acceptance line COMPLETES. The closed-slice record
docs/STATE.md-40-    was rotated verbatim to `docs/history/state-archive.md`
docs/STATE.md-41-    (2026-07-14 drift); per-slice detail lives in DEVLOG + `.review/`.
docs/STATE.md:42:  - **Open: otp-12d and otp-13** — both DEFERRED behind pf-final, see
docs/STATE.md-43-    Queue 1.
docs/STATE.md-44-- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
docs/STATE.md-45-  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
docs/STATE.md-46-  resume/re-derive on the unified baseline. Principle: ceiling-driven,
docs/STATE.md-47-  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
docs/STATE.md-48-- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
docs/STATE.md-49-  in Blocked); the codex loop governs all changes (D-2026-07-04-1).
docs/STATE.md-50-
docs/STATE.md-51-## Queue (ordered)
docs/STATE.md-52-
docs/STATE.md-53-1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
docs/STATE.md-54-   the only work item until it ships**: slices otp-1..13 through the
docs/STATE.md-55-   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
docs/STATE.md-56-   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
docs/STATE.md-57-   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
docs/STATE.md:58:   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
docs/STATE.md-59-   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
docs/STATE.md-60-   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
docs/STATE.md-61-   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
docs/STATE.md-62-   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
docs/STATE.md-63-   gap shapes recorded for the walk
docs/STATE.md:64:   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
docs/STATE.md-65-   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
docs/STATE.md-66-   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
docs/STATE.md-67-   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
docs/STATE.md-68-   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
docs/STATE.md:69:   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
docs/STATE.md-70-   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
docs/STATE.md-71-   pre-fix new arms for acceptance. Assembling the acceptance matrix now
docs/STATE.md-72-   would build otp-13's artifact from void rows.
docs/STATE.md-73-1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
docs/STATE.md-74-   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
docs/STATE.md-75-   reviewloop slice by slice"; implementation proceeds, each slice
docs/STATE.md-76-   through the codex loop).
docs/STATE.md-77-   Two experiments come BEFORE any code; both docs own their detail.
docs/STATE.md-78-   **(i) The A-B-B-A MTU run on `q` — `[x]` DONE 2026-07-14: MTU KILLED**
docs/STATE.md-79-   (`r = −3.1%`; `docs/bench/otp12-jumbo-win-2026-07-13/`). See the pf-0
--
docs/STATE.md-83-   (1.237/1.300/1.385/1.362); macOS↔macOS = **?** Design, decision rule and
docs/STATE.md-84-   the retraction of the "H1 dies" framing: **see NEXT ACTION at the top**
docs/STATE.md-85-   and the rev-2 pre-registration. **Both Macs are bench ENDS: the codex
docs/STATE.md-86-   loop CANNOT run during the session** (the gate enforces it).
docs/STATE.md-87-   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
docs/STATE.md-88-   *cross-direction* miss for a cell that ALREADY passes invariance — P1
docs/STATE.md-89-   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
docs/STATE.md-90-   acceptance criterion 1.** Not assumed either way. P2
docs/STATE.md-91-   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
docs/STATE.md-92-   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
docs/STATE.md:93:   → pf-final (ALL rigs) → otp-12d → otp-13.**
docs/STATE.md:94:1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
docs/STATE.md-95-   path, opposite directions: a fidelity fix ADDS per-file work to a path
docs/STATE.md-96-   already losing to robocopy, so planning them apart optimises one against
docs/STATE.md-97-   the other). Both docs own their detail; do not restate it here.
docs/STATE.md-98-   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
docs/STATE.md-99-     — Windows attributes + ADS silently dropped, exit 0, **both routes
docs/STATE.md-100-     (measured)**; loss is **conditional on file count**
docs/STATE.md-101-     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
docs/STATE.md-102-     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
docs/STATE.md-103-   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
docs/STATE.md:104:     apply **does not scale** (8 workers buy 1.05×; robocopy gets ~2.2× from 8
docs/STATE.md:105:     threads) and ships **one** worker. At EQUAL concurrency blit BEATS
docs/STATE.md-106-     robocopy; at 8-vs-8 it loses 1.9×. `docs/bench/win-local-ab-2026-07-13/`.
docs/STATE.md-107-2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
docs/STATE.md-108-   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
docs/STATE.md:109:   absorbed by otp-2/otp-12's rig matrices.
docs/STATE.md-110-3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
docs/STATE.md-111-   resumes/re-derives after ONE_TRANSFER_PATH ships.
docs/STATE.md-112-4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
docs/STATE.md-113-   likely landed inside otp-6's one-delete-rule slice; re-check first).
docs/STATE.md-114-5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
docs/STATE.md-115-   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
docs/STATE.md-116-   cutover as a runtime-selected write strategy in the unified receive
docs/STATE.md:117:   sink (design: eval doc §If-FAST-evidence; dead module deletes in
docs/STATE.md-118-   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
docs/STATE.md-119-   **Standing owner safety rule**: ALL activity on rig `zoey` stays
docs/STATE.md-120-   inside its `…/blit-temp/` folder — nothing written outside it, ever;
docs/STATE.md-121-   no daemon runs on zoey without a fresh go.
docs/STATE.md-122-6. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.
docs/STATE.md-123-
docs/STATE.md-124-## Authoritative docs right now
docs/STATE.md-125-
docs/STATE.md-126-- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
docs/STATE.md-127-  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
--
docs/STATE.md-143-  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
docs/STATE.md-144-
docs/STATE.md-145-## Blocked / waiting (all owner declarations; checkpoints are owner-only)
docs/STATE.md-146-
docs/STATE.md-147-- **The Mac↔Mac run is BLOCKED and NOT clearable by an agent** — round 11's
docs/STATE.md-148-  findings are unfixed (engine 2 HIGH, harness 1 BLOCKER + 4 HIGH) and both
docs/STATE.md-149-  Macs must be codex-quiet. Basis and detail: NEXT ACTION at the top of this
docs/STATE.md-150-  file; never restated here (re-verified 2026-07-14 against
docs/STATE.md-151-  `.review/results/macmac-harness-r11.*` and `git log -- scripts/bench_otp12pf_mac.sh`,
docs/STATE.md-152-  whose newest commit is still round 10's `8997f92`).
docs/STATE.md:153:- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
docs/STATE.md-154-  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
docs/STATE.md-155-  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
docs/STATE.md-156-  `.agents/machines.md`.
docs/STATE.md:157:- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
docs/STATE.md-158-  evidence, NOT acceptance evidence; Queue 1a):
docs/STATE.md-159-  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
docs/STATE.md-160-  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
docs/STATE.md-161-  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
docs/STATE.md-162-- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
docs/STATE.md-163-  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)
docs/STATE.md-164-
docs/STATE.md-165-## Open questions
docs/STATE.md-166-
docs/STATE.md-167-- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
--
REVIEW.md-63-| otp-4a | Daemon serves `Transfer` (runs `run_destination` as Responder; client `run_source`s as SOURCE initiator over a gRPC `FrameTransport`, in-stream carrier). Responder-resolution API (`DestinationTarget` + async `OpenResolver` through `establish`); read-only/unknown-module refusals as `SessionError` frames; A/B byte-identical parity vs old push; unified SizeMtime = safe-skip (⚠ narrow owner-ack, STATE). Codex FAIL (1/1 accepted: cancel must emit a framed `SessionError{CANCELLED}`). | `[x]` | `4b07bbb` + review fix `25f538b` |
REVIEW.md-64-| otp-4b | TCP data plane + resize + sf-2 pin ported to the session; deterministic mid-transfer cancel e2e. 4b-1 single-stream data plane (codex 3 passes), 4b-2 resize/multi-stream/sf-2 (codex PASS), 4b-3 mid-transfer cancel — source surfaces `SessionFault{CANCELLED}` over the data plane, no hang (codex 3 passes) | `[x]` | `881d412`+`e1aafcc`+`777dfc5` / `dce56de` / `3ae0a5f`+`a530005`+`46cc4bb` |
REVIEW.md-65-| otp-5a | Daemon serves BOTH roles via new `run_responder` (dispatches on declared `initiator_role`): a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent, streams its module tree, in-stream); a SOURCE initiator keeps otp-4 push. `establish`→`exchange_hello`+`responder_finish`; `run_source`/`run_destination` bodies→`drive_source`/`drive_destination`; new `SourceResponderTarget`; client `run_pull_session`. A/B byte-identical vs old `pull_sync`. Codex PASS (no findings). Data plane for the SOURCE responder is otp-5b. | `[x]` | `84be1cc` |
REVIEW.md-66-| otp-5b-1 | Single-stream SOURCE-responder TCP data plane: decouples data-plane connection role (RESPONDER binds+accepts, INITIATOR dials) from byte role (SOURCE sends, DESTINATION receives). New `accept_source_data_plane` (SOURCE responder accepts+sends) + `dial_destination_data_plane` (DESTINATION initiator dials+receives), `DestRecvPlane` enum; `responder_finish` binds for either role; `run_pull_session` defaults to TCP. Single-stream (`resizable=false`); resize is otp-5b-2. Codex FAIL → 1 Med accepted+fixed (grant-without-host fail-fast). | `[x]` | `e6a0b3b`+`13485ee` |
REVIEW.md-67-| otp-5b-2 | Pull data-plane resize: lifts otp-5b-1's single-stream cap so the pull data plane grows mid-transfer via sf-2 shape correction, exactly as push. Same `DataPlaneResize{ADD}`/`Ack` frames; only socket acquisition flips — SOURCE responder ACCEPTS each epoch-N socket off its listener, DESTINATION initiator DIALS it. `SourceSockets` enum (Dial/Accept); `add_stream` branches; `InitiatorReceivePlaneRun.add_dialed_stream`; `destination_session` initiator branch seeds `resize_live`+ceiling; `Frame::Resize` branches arm (responder) vs dial (initiator). Codex NEEDS FIXES → 1 Low accepted+fixed (ceiling uses advertised capacity, not a fresh local read). | `[x]` | `d579365`+`773a877` |
REVIEW.md-68-| otp-6a | Filters on the session: `SessionOpen.filter` honored via the universal `FilteredSource` chokepoint (not the per-impl `scan(filter)` arg); globs validated at OPEN, peer-notified refusal. Codex F1 accepted (chokepoint, not scan arg). *(Row backfilled 2026-07-10 — the 07-06 session logged the close only in DEVLOG.)* | `[x]` | `c026692`+`0bb27f5` |
REVIEW.md-69-| otp-6b | Mirror on the session — the ONE delete rule: DESTINATION diffs the complete source manifest at SourceDone, scan-complete-guarded + filter-scoped; `plan_session_deletions` + containment-checked `mirror_delete_pass`. Codex NEEDS FIXES → 2 accepted+fixed (High: keep-set now folds case on macOS too — case-insensitive-FS data-loss; Med: Windows read-only clear before delete). *(Row backfilled 2026-07-10.)* | `[x]` | `01d9c41`+`3c99557` |
REVIEW.md-70-| otp-7a | Resume block phase over the in-stream carrier (`docs/plan/OTP7_RESUME.md` Active, D-2026-07-09-1): DEST flags eligible needs (D2), sends per-grant `BlockHashList`, applies block records in place; SOURCE holds a resume need until its hash list arrives, sends only stale blocks (D1 graceful stale fallback); `files_resumed` real; resume sessions in-stream-only until 7b. All four plan guard-proof pins run live under both initiator roles; 5 guard proofs by temporary revert. Codex FAIL → 6 findings: 4 accepted + fixed (wire bounds D-2026-07-10-1 — block size clamped [64 KiB, 2 MiB] + 65_536-hash cap; choreography-bypass records rejected; arrival-time validation; mid-fault pin observes the partial patch), 1 partial (aggregate hash-list buffering documented), 1 deferred to 7b (cancel-during-resume e2e). | `[x]` | `4e5ff58` + review fix `1919410` |
REVIEW.md-71-| otp-10a | Push-shaped verb rides the session: `blit_app run_remote_push` (CLI copy/mirror/move-push + relay + TUI F1) reroutes onto `run_push_session`; deferred verb wiring lands — `PushSessionOptions` mirror/filter, `--force-grpc`→in-stream, w6-1 progress via new `SourceInstruments` (need-batch denominator; both carriers per-file lane), `--trace-data-plane`, resume flags, verb-level `end_of_operation_summary` print, old-push unreadable-scan error (move's source-delete gate); `PushExecutionOutcome` retyped to session `TransferSummary` so 10c is pure deletion. Codex NEEDS FIXES → 8 findings, 7 accepted+fixed, F1 in part (High: move now pushes `IgnoreTimes` — compare-skip + source-delete data loss, mutation-proven; copy half = standing owner Q. High: wire paths POSIX-normalized. High: daemon `--force-grpc-data` honored by sessions. Med: relay+resume refused; `SessionFault.io_kind` keeps `--retry` alive; resume w6-1 progress both carriers; fault-summary unit pins. Low: `build_spec` validates globs pre-connect). Suite 1555 → **1576**; 10 guard proofs by temporary mutation across both rounds. | `[x]` | `0fbc966` + review fixes `6b292ed` |
REVIEW.md-72-| otp-10b-1 | Checksum compare on the session (contract v3): `COMPARISON_MODE_CHECKSUM` = real content compare both roles — SOURCE fills manifest Blake3 via new `ChecksummingSource` (through the inner source's `open_file`, outside the filter), DEST hashes same-size diff candidates in the blocking chunk; daemon `--no-server-checksums` refuses at OPEN with new `CHECKSUM_DISABLED` (ResponderPolicy absorbs otp-10a's force_in_stream). Role-suite pins both layouts with SizeMtime controls; e2e served-skip + both-role refusal. Codex NEEDS FIXES → 5/5 accepted+fixed (High: unhashable files now EMIT with empty checksum — the drop let pulls succeed with a file silently absent; hashing stop probes bound teardown to one 64 KiB chunk both ends; `AbortFlagOnDrop` hoisted; delegated phase map + STATE drift). Suite 1576 → **1581**; 3 mutation guard proofs. | `[x]` | `e82859e` + review fixes `7d3a1f2` |
REVIEW.md:73:| otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
REVIEW.md-74-| otp-10c-1 | `--relay-via-cli` removed (owner decision D-2026-07-11-1) — remote→remote is delegated-only, the CLI never in the byte path: flag + `RemoteToRemoteRelay` route + all four relay-combination gates deleted; `RemoteTransferSource` + bounded-read helpers + constructed-counter die; `PushExecution.source` narrows `Endpoint`→`PathBuf` (remote push source unrepresentable); delegated hints reworded (CONNECT_SOURCE → manual two-hop). Codex FAIL → 3/3 accepted+fixed (Med: counter's positive control restored — new push e2e, mutation-proven against a no-op'd recorder; Med: live guidance purge incl. ARCHITECTURE/WHITEPAPER beyond codex's list; Low: comment retype + relay-1 row closed moot). Suite 1605 → 1585 (20 relay-only tests retired, accounted) → **1586** | `[x]` | `f53f5a4` + review fixes `27bef56` |
REVIEW.md:75:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
REVIEW.md:76:| otp-11b | THE LOCAL ORCHESTRATION DELETION — the last old path out of the tree (−6.2k lines): `orchestrator/`, `engine/` (dial RELOCATED VERBATIM → `src/dial.rs`, blob-identical, 17 tests), `local_worker`, `auto_tune/`, `change_journal/` (the UNSOUND journal skip — the 11a-addendum data-loss repro), `copy/parallel+stats`, `CopyConfig`; the otp-10c-2 F2 `compare_manifests` sweep (live compare owner `header_transfer_status` + `compare_file`/`CompareMode`/`CompareOptions`/`FileStatus` survive); stranded `plan_local_mirror`/`LocalDiffInputs`/`filter_unchanged`; types re-homed → `transfer_session/local.rs` (dead axes dropped, `JournalSkip`/`PredictorEstimate` retired); `TRANSFER_SESSION.md` local-carrier contract note. Codex CHANGES REQUESTED → core CONFIRMED ("deletion, re-homes, converted coverage, remote-session behavior, one-transfer-path structure, and the 1484-pass suite check out") + 6 docs/record findings, 6/6 fixed (live-doc sweep completed incl. WHITEPAPER/ARCHITECTURE/repo-guidance; predictor promises retyped; effective worker count printed; accounting equation corrected). Suite 1513 → **1484** (died-in-modules 41 + deleted files 10 + retired 5, conversions 25 in place, new +27; the otp-13 ≥1483 floor MET at the deletion slice, margin +1); SizeOnly mutation guard proof. The plan's deletion-proof acceptance line for "the separate local orchestration path" COMPLETES here. | `[x]` | slice `805e48c` + docs `b1650c4` + review fixes `9e810ee` |
REVIEW.md:77:| otp-12a | Zoey converge-up A/B recorded (design `docs/plan/OTP12_ACCEPTANCE_RUN.md` Active — owner flip; D-2026-07-12-1 residue rule). Three codex rounds: design CHANGES REQUIRED 7 findings (6 accepted + 1 overtaken-by-owner-decision); harness REQUEST CHANGES 9/9 accepted (zero false positives); run round FAIL 6/6 accepted (provenance `+sha` form, D2 supersession amendment, drift/gap wording per CSVs). En route: otp-2 daemon provenance corrected (staged pair was dirty `731023b`, not `e757dcc`); zoey I/O-storm diagnosed → per-run dest sweep. Evidence `docs/bench/otp12-zoey-2026-07-12/` (3 sessions incl. aborted storm): **10 PASS; pull_tcp_large FAIL-REFERENCE-DRIFT (rig-side by strongest evidence); push_tcp_small FAIL-SAME-SESSION 1.105** — both carried to the otp-13 walk. | `[x]` | design `045da4a`+`92e1d51`; harness `8f4fbf9`+`50dc135`; run `b2b6901`+`b3729da`+`042c06f`+`6bc9cb6`+`b0ebf73`+fixes `fa18787` |
REVIEW.md:78:| otp-12b | Mac↔Windows acceptance session recorded — THE INVARIANCE CRITERION MEASURED: 11/12 cells PASS at 1.003–1.057 (the owner's sentence holds); wm_tcp_mixed FAIL 1.237 (TCP×mixed×destination-initiator — real, block-1-corroborated, code-shaped). Converge 10/12 (push_tcp_small 1.149 FAIL-BOTH — matches zoey's 1.105, second rig; pull_tcp_mixed 1.313 same root). Cross: Win→Mac 6/6 beat the better old direction; Mac→Win gap rows recorded per D-2026-07-12-1 shapes (large unchanged / mixed+grpc_small narrowed / tcp_small widened), adjudication reserved to otp-13. Three codex rounds: harness FAIL 12/12 accepted; run-round FAIL 3/3 accepted (self-adjudication scrubbed); + two found-live fixes (pwsh `$rc:R` scope-parse sentinel; CR-split verdicts). 192 runs, zero voided. Evidence `docs/bench/otp12-win-2026-07-12/`. | `[x]` | harness `d30b1e3`+`772cfe6`+`d3eae58`; run `e21cf84`+`856af64`+`44c2046`+fixes `49dee5c` |
REVIEW.md:79:| otp-12c | Rig-D delegated-parity session recorded (netwatch-01↔skippy) + a rig-W re-baseline at the CUTOVER sha `f35702a` (12b measured `e21cf84`, so no committed rig-W evidence existed at the sha the shipped binaries embed). New harness `scripts/bench_otp12_delegated.sh` (plan D4: delegated = Mac CLI triggers `DelegatedPull`, no payload through the Mac; direct = the destination host's own CLI pulls; same session code, roles, data plane, destination disk and flush — only the initiator differs). **Rig D: 7/7 PASS** — RUNS=4 gave 5 PASS / 2 FAIL (`sw_tcp_mixed` 1.119, `ws_tcp_large` 1.129); both FAIL cells met D2's pre-registered escalation trigger (straddle + >25% arm spread) and re-ran at RUNS=8, whose medians govern per the D2 supersession amendment → both PASS (1.035, 1.068), with the wide spread appearing on the *direct* arm too at higher n. 88 timed runs across two sessions, **zero voided pairs**. Rig-W re-baseline: 198 runs, 93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED — `wm_tcp_mixed` invariance **1.300** (12b: 1.237), i.e. the TCP×mixed×dest-initiator cell did NOT wash out at the cutover sha. Three harness bugs found live, each caught by the script's own gates (apostrophes in `:?` messages swallowing assignments — the otp-12b `772cfe6` bug re-made; macOS `$TMPDIR` blowing ssh's 104-byte ControlPath limit; skippy's `drop_caches` needing the exact NOPASSWD grant, whose generic form silently no-op'd → runs would have read WARM). Codex FAIL → **7/7 accepted, 0 rejected**: F1 cold-cache fail-open (HIGH — grant now a hard gate; a failed purge voids the pair); **F2 D2 misread (HIGH — the first draft scoped the escalation amendment to converge-up rows only and so ducked the verdict; the rule says "a comparison", delegated parity included → rig D 7/7 PASS)**; F3 provenance (`proto/` added to the dirty-tree gate; `+sha` no longer substring-matches `+sha.dirty.<hash>` — the otp-12a zoey trap); F4 machine-readable build fields recorded harness HEAD, not the gated binary identity; F5 silent `sync`/drain failures (failed sync → NA → void; a disk regex matching no device is DRAIN-NODEV, not drained); F6 teardown logged "stopped" without verifying (a survivor now exits nonzero); F7 a PASS listed among the FAILs. Codex independently confirmed the otp-12b F5 arm asymmetry does NOT recur and that every committed CSV recomputes exactly. Evidence `docs/bench/otp12c-{win,delegated}-2026-07-13/`. Acceptance reserved to the otp-13 owner walk. Suite untouched at **1484** (zero `crates/`/`proto/` changes). | `[x]` | harness `c26bc2d`+`b49413d`+`a2dea3f`; evidence `d12534d`+`68bb490`; record `9350b24` + review fixes `0fb4a64`+`4cc9b6e` |
REVIEW.md-80-| otp-10b-2 | Pull-shaped verb rides the session — verb cutover COMPLETE: `blit_app run_remote_pull` (CLI copy/mirror/move-pull + TUI F3) reroutes onto `run_pull_session`; ONE args→compare mapping for BOTH verbs (`transfers/compare.rs`; push `--checksum` gate lifted, every compare flag + `--ignore-existing` honored both directions); dest-side w6-1 progress + pull `--trace-data-plane` via new `DestinationInstruments`; printers retype to the session summary; mirror = the in-session one delete rule (`apply_pull_mirror_purge` off the verb path); move maps IgnoreTimes/Checksum-only + new `--size-only` move gate; A/B parity vs old pull on twin daemons; multistream e2e ported to trace-based fan-out. Codex NEEDS FIXES → 6 findings, 5 fixed (1 in part), 1 deferred (High: cancel frame mid-purge now aborts deletions; delegated move rode SizeMtime-then-delete — wire-pinned fix; gate texts + explicit local move mapping — the claimed live local loss did NOT reproduce, probe recorded, pins are otp-11 regression pins. Med: served sessions record real kind/endpoint/metrics — pull = PullSync rows; monitor-through-purge display deferred to the M-C reshape. Low: TUI builders through the one mapping). Suite 1581 → **1605**; 12 mutation guard proofs across both rounds. | `[x]` | `2014782` + review fixes `3534ffa` |
REVIEW.md-81-
REVIEW.md-82-## Design-review queue (ratified D-2026-06-11-2, in execution order)
REVIEW.md-83-
REVIEW.md-84-Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
REVIEW.md-85-`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (per-finding evidence).
REVIEW.md-86-Coder loop: pick the topmost `[ ]` row. W2.3 requires a `docs/plan/` doc with
REVIEW.md-87-**Status: Active** before code.
REVIEW.md-88-
REVIEW.md-89-| ID | Severity | Title | Status | Branch | Commit |
REVIEW.md-90-|----|----------|-------|--------|--------|--------|
REVIEW.md-91-| w5-1-log-backend | Medium | Install stderr log backend (warn) in all 4 binaries + one prefix convention; today every log::warn/error is discarded | `[x]` | master | `56bda09`+`7145202` |
REVIEW.md-92-| w4-2-delete-push-upload-channel | Medium | Delete the 262,144-slot push upload channel (drain-and-discard; wedges gRPC-fallback pushes >262k files) | `[x]` | master | `03bcb1d` |
REVIEW.md-93-| w5-2-retry-classifier-consolidation | Medium | Delete dead contradictory blit-core/errors.rs; move is_retryable into blit-core with contract test | `[x]` | master | `9c960dc` |
REVIEW.md:94:| w4-1-abortondrop-family | High | Hoist AbortOnDrop; fix the remaining detach-on-drop sites (2 of 5 deleted with the Pull RPC at ue-r2-1h; design-2 now scopes to push/control.rs only; JoinSet for per-stream workers). Codex NEEDS FIXES (1 Low: relocated drop-test was vacuous) → fixed `bedfa52` | `[x]` | master | `65ecb93`+`bedfa52` |
REVIEW.md-95-| w9-5-jobs-lifecycle-e2e | Medium | jobs/detach lifecycle e2e tests (Subscribe, watch fallback, cancel exit codes) — net before W4.3 | `[x]` | master | `ad773d8` |
REVIEW.md-96-| w4-3-daemon-disconnect-racing | Medium | Daemon handlers race tx.closed()+cancel token (delegated_pull's select generalized to resolve_transfer_outcome + resolve_streaming_outcome; 2 live sites — pull spawn closure died with the Pull RPC at ue-r2-1h); false supports_cancellation comment fixed, dispatch policy itself unchanged (flip = open owner question, since decided D-2026-07-04-3 → w4-5). Codex PASS (0 findings) | `[x]` | master | `37d7f91` |
REVIEW.md-97-| w4-5-supports-cancellation-flip | Medium | Flip supports_cancellation for Push/PullSync (owner-authorized D-2026-07-04-3): CancelJob + TUI F2 work on attached transfers; policy-only after w4-3's race wiring (one-predicate flip — Pull history-only stays gated; TUI/CLI needed zero logic changes); contract change exit 2→0 pinned at table + RPC-handler level, authz now covers flipped kinds; every old-policy comment surface updated incl. proto wire-contract doc. Codex NEEDS FIXES (1 Low: module-doc scope log still claimed Pull wired) → fixed `1708075` | `[x]` | master | `05a8b39`+`1708075` |
REVIEW.md-98-| w1-2-data-socket-policy-helper | Medium | Shared configure_data_socket (NODELAY/keepalive/tuned buffers) hoisted to blit-core; pull client connect + daemon push/pull_sync accepts all route through it; pull_sync passes the dial's tcp_buffer_bytes (resize accept reads it live — the computed-and-discarded gap closed); daemon's silently-swallowing twin + socket2 dep deleted. design-3 (connect timeouts) untouched. Codex PASS (0 findings) | `[x]` | master | `16237e2` |
REVIEW.md-99-| w1-3-tcp-keepalive-honesty | Medium | Real TcpKeepalive timing (idle 60s / interval 10s / retries 5) at the single site left after w1-2 (the shared helper; daemon copy already deleted, logs-failure clause satisfied structurally) — dead idle peer detected in ~2 min, not ~2 h; comments now true; socket2 features=["all"] for retries + test getters. Codex PASS (0 findings) | `[x]` | master | `865fc1e` |
REVIEW.md-100-| w1-4-accept-token-constants | Low | Shared DATA_PLANE_ACCEPT_TIMEOUT(30s)/DATA_PLANE_TOKEN_TIMEOUT(15s) in remote::transfer::socket replacing the 3 declarations left at HEAD (4th died with the Pull RPC); values byte-identical. Codex NEEDS FIXES (1 Low: stall_guard comments named the deleted pair) → fixed `d17b089` | `[x]` | master | `6a19e1d`+`d17b089` |
REVIEW.md-101-| w2-1-delete-warmup-machinery | Medium | Delete dead auto_tune warmup branches + analyze_warmup_result (honest static table) | `[x]` | master | `2a8a490` |
REVIEW.md-102-| w2-2-stream-ladder-owner | Medium | Single stream-count/chunk owner: the 3 stream ladders died with REV4 (ue-r2-1e dial / -1f initial_stream_proposal takes file_count / -1h Pull RPC; absorption recorded D-2026-06-20-1); this slice closed the remaining leg — deleted the dead transfer_plan chunk lane (16/32 MiB ladder, Plan/PlannedPayloads wrappers, chunk_bytes_override + refresh sites, never-called plan_to_daemon_format, orphaned TuningParams); dial is the single chunk owner; W3.1's "settled tuning owner" = engine::TransferDial. Codex NEEDS FIXES (1 Low: new ensure_dial comment said "fallback batch" in the data-plane branch) → fixed `27f53a0` | `[x]` | master | `01209bc`+`27f53a0` |
REVIEW.md-103-| w2-3-multistream-pull-plan | High | Multi-stream pull-sync: write plan doc (authorized D-2026-06-11-2), harvest deprecated Pull's pattern, implement — absorbed into REV4 (D-2026-06-20-1); delivered as `ue-r2-1g` | `[x]` | master | `48e583e` |
REVIEW.md-104-| w2-4-delete-pull-rpc | High | Delete deprecated Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); port scan_remote_files — absorbed into REV4; delivered as `ue-r2-1h` | `[x]` | master | `2a13f53` |
REVIEW.md-105-| w3-1-memory-aware-buffer-pool | High | BufferPool::for_data_plane(chunk_bytes, streams) owns the formula (streams*2+4, shared 64 KiB DATA_PLANE_BUFFER_FLOOR) + available/4 memory cap with a 2-buffers-per-stream liveness floor (buffer shrinks, never concurrency — the double-buffered sender holds 2); replaces the 3 pasted sites; elastic paths authorize dial.ceiling_max_streams() up front (closes both "growing the pool live is a W3.1 concern" deferrals); fixes the sysinfo units bug (0.38 returns bytes; old *1024 over-reported memory 1024x, making every cap vacuous); RECEIVE_CHUNK_SIZE comment truth. 8 params-layer pins, mutation-verified. Codex PASS (0 findings) | `[x]` | master | `f49f8f6` |
REVIEW.md-106-| w6-1-progress-event-contract | Medium | ProgressEvent contract owned by blit-core: bytes ride Payload only (FileComplete's bytes field DELETED — design-1's class unrepresentable); files count once via byteless FileComplete{wire-relative path} or Payload.files (aggregate lane: delegated bridge, tar-shard appliers); ManifestBatch = direction-flavored denominator, documented. All producers normalized (TCP receive double-emit fixed; tar-shard members + resume lanes gain missing events; send side moves planned bytes to Payload; gRPC pull absolute-path leak fixed; 2 dead emitters conformed pending w8). Consumers collapsed onto shared ProgressTotals (CLI monitor — closes design-1 — + all 3 TUI forwarders; TUI's 3 accumulate_* rules deleted). +12 blit-core tests incl. 4 producer emission pins, 2 mutation-verified; 1460→1472/0/2. Codex PASS (0 findings) | `[x]` | master | `8fd8978` |
REVIEW.md-107-| w6-2-progress-residue-verify | Medium | Verify-then-fix map §1.6 residue: all three claims CONFIRMED at HEAD `8fd8978` (delegated live progress wire-dead — zero BytesProgress producers; daemon row counters fed only by delegated dispatch core.rs:667 — push receive + pull_sync serve stay 0; TransferProgress/GetState/TransferComplete hardcode bytes_total/files_* to 0). Per the ratified spec each confirmed item became its own follow-on: w6-2a/-2b/-2c filed in the pending-review section (independent slices — 2a needs only the already-fed delegated counter; suggested order 2b→2a→2c on smallest-first grounds, coder's pick). Verification + filing only, no code. Codex NEEDS FIXES (2 Low, doc-coherence: "no code anywhere constructs" overstated vs consumer tests; 2b-as-substrate-for-2a wording) → both fixed | `[x]` | master | `0aba593` + fix |
REVIEW.md:108:| w4-4-blocking-work-off-runtime | Medium | Blocking work off the runtime: push manifest requires-upload checks (canonical containment walk + stat, ~3M+ syscalls per 1M-file push, previously inline on a tokio worker) now buffer and run in chunked spawn_blocking batches (MANIFEST_CHECK_CHUNK=128 = need-list early-flush threshold; lexical-containment alternative rejected — weakens F2); need-list order kept, mid-manifest TCP spin-up moved to post-chunk-drain, ManifestComplete drains the remainder, design-4 untouched. collect_pull_entries_with_checksums runs ENTIRELY on one spawn_blocking thread (single-file branch's inline full-file Blake3 + metadata probes were pinning a worker). +4 tests incl. containment-escape via the batched path, mutation-verified; 1472→1476/0/2. Codex NEEDS FIXES (1 Medium: chunk-only draining muted the batcher's 64KiB/5ms early-flush for trickling manifests) → manifest_drain_due chunk-or-delay trigger, fixed `768e7e3` | `[x]` | master | `0feca34`+`768e7e3` |
REVIEW.md-109-| w9-1-ungate-windows-tests | High | Remove blanket #[cfg(unix)] from remote transfer tests with nothing unix-specific | `[x]` | master | `9324559` |
REVIEW.md-110-| w9-2-revive-root-tests | Medium | Relocate dead workspace-root tests/ into blit-core/tests (MirrorPlanner coverage); delete connection.rs; fix AGENTS.md §4 | `[x]` | master | `461525d` |
REVIEW.md-111-| w9-3-test-harness-builder | Medium | One daemon-spawn harness: TestContext::builder() (read_only/delegation/extra_daemon_args) + spawn_daemon/spawn_second_daemon absorb the SEVEN clones at HEAD (audit counted 5; w9-4/w9-5 had each added another — the finding's prediction twice proven) plus 5 cli_bin/7 run_with_timeout/4 ChildGuard copies; daemon build OnceLock'd per test binary (R16-F1 independence kept; was ~75 nested cargo invocations serializing on the build-dir flock — the daemon-spawn load-flakiness home); new blit_core::remote::grpc_server owns the audit-1 HTTP/2 keepalive (30s/20s) as production_server_builder() — daemon main.rs + all FIVE fake tonic servers (not 3: remote_remote ×2, jobs_lifecycle, pull_sync_with_spec_wire ×2) route through it, zero bare Server::builder() left; port-collision race surfaced by the build de-serialization fixed two-layer (process-global claimed-port set + child-death readiness check). Net −1,251 test-tree lines; 1478→1479 same-method A/B (+1 keepalive pin, mutation-verified). Codex NEEDS FIXES (1 Medium: fake-server :0 bind bypassed the claimed set — wrong-listener race for mixed fake/daemon binaries) → claim_port() shared, fixed | `[x]` | master | `f6e592e`+`8641bc6` |
REVIEW.md-112-| w9-4-readonly-enforcement-tests | Medium | Tests for all 3 read-only-module gates (push, purge, delegated pull) — zero coverage today | `[x]` | master | `4d67210` |
REVIEW.md-113-| w7-1-mirror-executor-consolidation | Medium | One mirror/purge deletion executor + parallel enumerate_local_manifest in blit-core (R58-F3 class closure) | `[ ]` | — | — |
REVIEW.md-114-| w7-2-filter-spec-chokepoint | Medium | filter_from_spec pub; push handler uses validated chokepoint (mirror-purge filter currently unvalidated) | `[ ]` | — | — |
REVIEW.md-115-| w7-3-wire-metadata-helpers | Medium | Wire metadata + path helpers into blit-core; one mtime error convention; delete per-crate twins | `[ ]` | — | — |
REVIEW.md-116-| w7-4-hash-reader-helper | Medium | checksum::hash_reader owning the 256 KiB loop; daemon build_file_header calls it | `[x]` | master | `6b2f433` |
REVIEW.md-117-| w7-5-presenter-formatting | Medium | format_bps in blit_app::display (binary units); switch jobs.rs + 5 TUI copies | `[ ]` | — | — |
REVIEW.md-118-| w7-6-default-port-pub | Low | RemoteEndpoint::DEFAULT_PORT pub; delete 9031 literals | `[x]` | master | `de04054` |
REVIEW.md-119-| w8-1-foundation-deadcode-sweep | Medium | Delete tar_stream, delete.rs, copy/parallel+stats, chunked_copy_file, fs_enum leftovers (~800 lines). zero_copy EXCLUDED → w8-1b | `[ ]` | — | — |
REVIEW.md:120:| w8-1b-zero-copy-fast-eval | Medium | Evaluate wiring splice/zero_copy into the receive pipeline (owner: FAST potential); outcome = plan doc or deletion | `[x]` | master | `6189d82` |
REVIEW.md:121:| w8-2-delete-control-plane-payload | Medium | Delete transfer_payloads_via_control_plane (zero-caller duplicate); sequence with W1.1 chunk_bytes deletion | `[ ]` | — | — |
REVIEW.md-122-| w8-3-deadcode-hygiene-sweep | Low | --interval-ms flag, blit-cli unused deps, blit-app stubs, stale #[allow(dead_code)] sweep | `[ ]` | — | — |
REVIEW.md-123-| w5-3-daemon-status-helpers | Medium | internal_err({:#}) + io_to_status helpers; sweep ~69 chain-amputating + 116 Status::internal sites | `[ ]` | — | — |
REVIEW.md-124-| w5-4-mpsc-sendfail-vocabulary | Medium | One honest mpsc send-failure vocabulary; prefer joining the exited task's real error | `[ ]` | — | — |
REVIEW.md-125-| w5-5-logger-trait-cleanup | Low | Logger trait permanently-noop error channel cleanup | `[ ]` | — | — |
REVIEW.md-126-| w9-6-test-misc | Low | Harness stderr capture; tuning-tier unit tests | `[ ]` | — | — |
REVIEW.md-127-| w10-docs-batch | Medium | Docs batch: AGENTS.md ghost names, WORKFLOW_PHASE_2 re-status, --resume/--retry help scoping (help+manpage+README), comment-truth sweep | `[ ]` | — | — |
REVIEW.md-128-
REVIEW.md-129-## Currently pending review
REVIEW.md-130-
REVIEW.md-131-| ID                | Severity | Title                                       | Status | Branch      | Commit    |
REVIEW.md-132-|-------------------|----------|---------------------------------------------|--------|-------------|-----------|
REVIEW.md-133-| relay-1-subpath-double-join | Low | `--relay-via-cli` with a subpath source scans `sub/sub` (endpoint rel_path joined twice). Pre-existing (deleted Pull-RPC code had the identical join); surfaced by the ue-r2-1h self-review panel; port kept parity, fix deferred. **CLOSED AS MOOT at otp-10c-1 (D-2026-07-11-1): the relay path and its scan were deleted with `--relay-via-cli`; nothing joins the rel_path twice because nothing joins it at all** | `[x]` | master | `f53f5a4` |
REVIEW.md-134-| win-1-push-needlist-separators | High | Windows daemon push need-list echoed native separators — every nested push to a Windows daemon stalled 30s. One-line `relative_path_to_posix` fix; reviewed within the ue-r2-1h codex+panel batch | `[x]` | master | `48c5a11` |
REVIEW.md-135-| design-1-cli-pull-byte-double-count | Medium | CLI pull progress double-counts bytes on the TCP data plane (producer reports both Payload and FileComplete with full bytes; CLI fold adds both). From design map §1.6, hand-verified. Fixed structurally by w6-1 (producer double-emit removed AND FileComplete's bytes field deleted — the class is unrepresentable); graded within the w6-1 codex round | `[x]` | master | `8fd8978` |
REVIEW.md-136-| design-2-orphaned-daemon-data-planes | High | Daemon data-plane tasks detach (not abort) on control-stream death at 3 spawn sites; orphan unreachable by CancelJob. AbortOnDrop fix exists but never propagated. From design map §1.9, hand-verified. Fixed by w4-1 (2 of 3 sites deleted with the Pull RPC at ue-r2-1h; remaining push/control.rs site now wrapped); graded within the w4-1 codex round | `[x]` | master | `65ecb93` |
REVIEW.md-137-| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
REVIEW.md-138-| w6-2a-delegated-bytesprogress-producer | Medium | Delegated live progress is wire-dead: proto BytesProgress has zero producers — the dst daemon sends Started, silence, then one post-hoc ManifestBatch (delegated_pull.rs:363-369 deliberate 0.1.0 gap, :433). The row atomic is ALREADY fed (core.rs:667); bridge it onto the DelegatedPullProgress stream on the progress tick so CLI footer + TUI delegated pane go live. Client side needs nothing (w6-1 aggregate lane + report_bytes_progress ready). Filed by w6-2 verification | `[ ]` | — | — |
REVIEW.md-139-| w6-2b-daemon-counters-push-pullsync | Medium | Daemon row byte counters stay 0 for served sessions. **Re-scoped at otp-10c-2** (the original prescription targeted the deleted push/pull_sync handlers): the served `Transfer` dispatcher builds its session config without `with_byte_progress`/`ByteProgressSink` (service/transfer.rs module-doc note), so GetState/TransferProgress/TransferComplete report 0 bytes for served-session rows of either role. Wire `job.bytes_counter()` through the served responder config, as `delegated_pull` already does (core.rs) and otp-9a did for the initiated side. Filed by w6-2 verification | `[ ]` | — | — |
REVIEW.md-140-| w6-2c-daemon-progress-denominators | Medium | Daemon event stream has no denominators or file counts: TransferProgress hardcodes bytes_total/files_completed/files_total 0 (core.rs:240-242), TransferComplete.files 0 (:322-325, + tcp_fallback_used false :329), GetState bytes_total 0 (:994-996) — "N of M"/percent impossible for every consumer. Thread manifest totals + a files counter onto ActiveJobs rows. Filed by w6-2 verification | `[ ]` | — | — |
REVIEW.md-141-| design-4-fallback-midmanifest-negotiation | High | Forced-gRPC pushes fail at ≥128 files (FILE_LIST_EARLY_FLUSH_ENTRIES; ~100 flaky). Mechanism VERIFIED two-sided: daemon announced fallback negotiation mid-manifest AND a force_grpc client streamed FileData with no negotiation at all — both racing the daemon manifest loop's FileData rejection. Fixed: daemon early-flush branch TCP-only; client gates fallback sends on fallback_negotiated. Owner-ratified 2026-06-12. NOTE: grade before design-5 (sequential overlapping commits on push/client/mod.rs) | `[x]` | master | `ddfeb58` |
REVIEW.md:142:| design-5-send-failure-masks-rejection | Medium | Push rejection reason (e.g. read-only) masked by 'failed to send push request payload' when the client loses the send-vs-status race — first CI failure surfaced by the w9-1/w9-4 ungating (macOS+Windows). Fixed: prefer_server_error harvests the daemon's terminal status on send failure at the 3 manifest-phase sites; 500-file deterministic regression. Owner-ratified 2026-06-12 ("strong bias for proper fixes"). Grade after design-4 | `[x]` | master | `08d71a2` |
REVIEW.md-143-| audit-h1-mirror-relay-incomplete-scan | Data-loss | Reject `mirror --relay-via-cli` for remote→remote (round 2: gate moved before mirror confirm prompt + yes=false regression test) | `[x]` | `master` | `4467faf` |
REVIEW.md-144-| audit-h3a-push-receive-stall | Robustness | StallGuard on the daemon push-receive socket (`TRANSFER_STALL_TIMEOUT` hoist) — closes one of three remaining stall-guard gaps from R3 H3; symmetric with audit-1c CLI pull-receive | `[x]` | `master` | `dd51a1c` |
REVIEW.md-145-| audit-m28-tui-sot-sweep | Docs | TUI source-of-truth sweep (round 2: audit INDEX + R3 updated to record 2026-06-04 owner ratification of H10b + resolution of L39/M27/M28) | `[x]` | `master` | `15fabbf` |
REVIEW.md-146-| audit-l39-m27-env-var-purge | Convention | Owner-directed env-var purge (round 2: bench-script prose + Clap `hide_short_help` doc corrections in 3 sites) | `[x]` | `master` | `ec06a95` |
REVIEW.md-147-| audit-h11-f1-confirm-detail-err | Data-loss UI | F1 confirm-detail explicit Local/Remote/Err arms + `debug_assert!` (round 2: re-armed at build-fix HEAD; h11 logic itself was correct in dirty tree, blocked by uncommitted Phase 6 dual-pane modules) | `[x]` | `master` | `1b3cb39` |
REVIEW.md-148-| audit-h3b-pull-data-plane-write-stall | Robustness | New `StallGuardWriter<W>` wired inside `DataPlaneSession` (round 2: same build-fix re-arm + tightened `Ok(0)` semantics so a zero-byte poll_write doesn't reset the deadline) | `[x]` | `master` | `1b3cb39` |
REVIEW.md-149-| audit-h3c-slice1-grpc-fallback-frame-contract | Robustness | Slice 1 of 2: gRPC fallback chunk cap at 1 MiB (`GRPC_FALLBACK_CHUNK_BYTES`) decoupled from TCP tuning; 3 CLI pull receive sites routed through `recv_fallback_message` (the chokepoint slice 2 will wrap with the dynamic progress watchdog). Round-2 adversarial concerns all addressed. Verified 2026-06-11 (owner accept; review assessment found the cap also fixes the tonic 4 MiB decode-limit failure — see DEVLOG, feeds slice-2 re-scope) | `[x]` | `master` | `bf4cc82` |
REVIEW.md-150-| d-62-f1-trigger-error | Feature | Inline validation feedback in the F1 trigger modal (round 2) | `[x]` | `phase5/a1` | `0b47a72` |
REVIEW.md-151-| d-63-f1-push-progress | Feature | Live byte/file footer for the F1 push (round 2) | `[x]` | `phase5/a1` | `aba54f8` |
REVIEW.md-152-| d-64-f1-push-ttl | Feature | Auto-hide the F1 push outcome footer (round 2) | `[x]` | `phase5/a1` | `2f67e96` |

exec
/bin/zsh -lc "rg -n -C 18 \"refus|accepted: false|accepted.*false|ResizeAck|shape.*resize|propose_shape_resize|resize_settled\" crates/blit-core/src/dial.rs crates/blit-core/src/transfer_session/mod.rs crates/blit-core/tests/transfer_session_roles.rs -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/dial.rs-91-        .and_then(|capacity| (capacity.max_streams > 0).then_some(capacity.max_streams as usize))
crates/blit-core/src/dial.rs-92-        .unwrap_or(DIAL_CEILING_MAX_STREAMS)
crates/blit-core/src/dial.rs-93-        .clamp(1, DIAL_CEILING_MAX_STREAMS)
crates/blit-core/src/dial.rs-94-}
crates/blit-core/src/dial.rs-95-
crates/blit-core/src/dial.rs-96-/// The one mutable tuning object for a transfer.
crates/blit-core/src/dial.rs-97-#[derive(Debug)]
crates/blit-core/src/dial.rs-98-pub struct TransferDial {
crates/blit-core/src/dial.rs-99-    chunk_bytes: AtomicUsize,
crates/blit-core/src/dial.rs-100-    prefetch_count: AtomicUsize,
crates/blit-core/src/dial.rs-101-    /// 0 = unset (kernel default), matching the old `Option<usize>`.
crates/blit-core/src/dial.rs-102-    tcp_buffer_bytes: AtomicUsize,
crates/blit-core/src/dial.rs-103-    initial_streams: AtomicUsize,
crates/blit-core/src/dial.rs-104-    max_streams: AtomicUsize,
crates/blit-core/src/dial.rs-105-    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
crates/blit-core/src/dial.rs-106-    // resize ids; 0 is reserved for the initial stream set) ──────────
crates/blit-core/src/dial.rs-107-    /// Settled live stream count. Epoch-0 write is
crates/blit-core/src/dial.rs-108-    /// `set_negotiated_streams`; later writes come from
crates/blit-core/src/dial.rs:109:    /// `resize_settled` on an accepted epoch.
crates/blit-core/src/dial.rs-110-    live_streams: AtomicUsize,
crates/blit-core/src/dial.rs-111-    /// Last settled epoch (0 until the first accepted resize).
crates/blit-core/src/dial.rs-112-    resize_epoch: AtomicU32,
crates/blit-core/src/dial.rs-113-    /// In-flight proposal's epoch; 0 = none. While non-zero no new
crates/blit-core/src/dial.rs-114-    /// proposal is produced (the wire is idempotent but overlapping
crates/blit-core/src/dial.rs-115-    /// epochs would complicate sub-token registration).
crates/blit-core/src/dial.rs-116-    pending_epoch: AtomicU32,
crates/blit-core/src/dial.rs-117-    /// Resize-eligible ticks since the last settle (cooldown clock).
crates/blit-core/src/dial.rs-118-    ticks_since_settle: AtomicU32,
crates/blit-core/src/dial.rs-119-    /// Consecutive same-direction tick counter: positive = "pipe clean
crates/blit-core/src/dial.rs-120-    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
crates/blit-core/src/dial.rs-121-    /// dials floored" streak. Any other tick resets it.
crates/blit-core/src/dial.rs-122-    resize_sustain: AtomicI32,
crates/blit-core/src/dial.rs-123-    // Profile-clamped bounds, fixed at construction.
crates/blit-core/src/dial.rs-124-    ceiling_chunk_bytes: usize,
crates/blit-core/src/dial.rs-125-    ceiling_prefetch: usize,
crates/blit-core/src/dial.rs-126-    ceiling_max_streams: usize,
crates/blit-core/src/dial.rs-127-    ceiling_tcp_buffer_bytes: usize,
crates/blit-core/src/dial.rs-128-}
crates/blit-core/src/dial.rs-129-
crates/blit-core/src/dial.rs-130-/// One engine resize decision (`ue-r2-2`). The adapter that owns the
crates/blit-core/src/dial.rs-131-/// control stream turns this into a wire `DataPlaneResize` (the engine
crates/blit-core/src/dial.rs-132-/// stays wire-type-free here on purpose) and MUST eventually call
crates/blit-core/src/dial.rs:133:/// [`TransferDial::resize_settled`] for the epoch — with what actually
crates/blit-core/src/dial.rs-134-/// happened — or no further proposals are produced.
crates/blit-core/src/dial.rs-135-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
crates/blit-core/src/dial.rs-136-pub struct ResizeProposal {
crates/blit-core/src/dial.rs-137-    /// The wire epoch for this change (`resize_epoch() + 1`).
crates/blit-core/src/dial.rs-138-    pub epoch: u32,
crates/blit-core/src/dial.rs-139-    /// Absolute desired live count (idempotent, per the proto).
crates/blit-core/src/dial.rs-140-    pub target_streams: usize,
crates/blit-core/src/dial.rs-141-    /// Convenience: `target_streams > live` at proposal time.
crates/blit-core/src/dial.rs-142-    pub add: bool,
crates/blit-core/src/dial.rs-143-}
crates/blit-core/src/dial.rs-144-
crates/blit-core/src/dial.rs-145-impl TransferDial {
crates/blit-core/src/dial.rs-146-    /// Conservative start with default ceilings (no receiver profile).
crates/blit-core/src/dial.rs-147-    pub fn conservative() -> Self {
crates/blit-core/src/dial.rs-148-        Self::conservative_within(None)
crates/blit-core/src/dial.rs-149-    }
crates/blit-core/src/dial.rs-150-
crates/blit-core/src/dial.rs-151-    /// Conservative start bounded by the receiver's advertised
--
crates/blit-core/src/dial.rs-233-        self.initial_streams.store(clamped, Ordering::Relaxed);
crates/blit-core/src/dial.rs-234-        self.live_streams.store(clamped, Ordering::Relaxed);
crates/blit-core/src/dial.rs-235-        clamped
crates/blit-core/src/dial.rs-236-    }
crates/blit-core/src/dial.rs-237-
crates/blit-core/src/dial.rs-238-    // ── ue-r2-2 resize policy ────────────────────────────────────────
crates/blit-core/src/dial.rs-239-
crates/blit-core/src/dial.rs-240-    /// The settled live stream count (epoch-0 negotiation, then each
crates/blit-core/src/dial.rs-241-    /// accepted resize).
crates/blit-core/src/dial.rs-242-    pub fn live_streams(&self) -> usize {
crates/blit-core/src/dial.rs-243-        self.live_streams.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs-244-    }
crates/blit-core/src/dial.rs-245-
crates/blit-core/src/dial.rs-246-    /// Last settled resize epoch (0 = only the initial stream set).
crates/blit-core/src/dial.rs-247-    pub fn resize_epoch(&self) -> u32 {
crates/blit-core/src/dial.rs-248-        self.resize_epoch.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs-249-    }
crates/blit-core/src/dial.rs-250-
crates/blit-core/src/dial.rs:251:    /// True while a proposal is awaiting `resize_settled`.
crates/blit-core/src/dial.rs-252-    pub fn resize_pending(&self) -> bool {
crates/blit-core/src/dial.rs-253-        self.pending_epoch.load(Ordering::Relaxed) != 0
crates/blit-core/src/dial.rs-254-    }
crates/blit-core/src/dial.rs-255-
crates/blit-core/src/dial.rs-256-    fn cheap_dials_maxed(&self) -> bool {
crates/blit-core/src/dial.rs-257-        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
crates/blit-core/src/dial.rs-258-            && self.prefetch_count.load(Ordering::Relaxed) >= self.ceiling_prefetch
crates/blit-core/src/dial.rs-259-    }
crates/blit-core/src/dial.rs-260-
crates/blit-core/src/dial.rs-261-    fn cheap_dials_floored(&self) -> bool {
crates/blit-core/src/dial.rs-262-        self.chunk_bytes.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs-263-            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
crates/blit-core/src/dial.rs-264-            && self.prefetch_count.load(Ordering::Relaxed)
crates/blit-core/src/dial.rs-265-                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
crates/blit-core/src/dial.rs-266-    }
crates/blit-core/src/dial.rs-267-
crates/blit-core/src/dial.rs-268-    /// One resize-eligible tuner tick. Streams move only as the LAST
crates/blit-core/src/dial.rs-269-    /// escalation step in either direction: the cheap dials must
crates/blit-core/src/dial.rs-270-    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
crates/blit-core/src/dial.rs-271-    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
crates/blit-core/src/dial.rs-272-    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
crates/blit-core/src/dial.rs-273-    /// since the last settle, and no proposal may be in flight. Idle
crates/blit-core/src/dial.rs-274-    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
crates/blit-core/src/dial.rs-275-    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
crates/blit-core/src/dial.rs-276-    /// folded in at construction — `CapacityProfile.max_streams` is
crates/blit-core/src/dial.rs-277-    /// authoritative per the proto). One stream per epoch.
crates/blit-core/src/dial.rs-278-    ///
crates/blit-core/src/dial.rs-279-    /// The caller must forward the returned proposal to the peer and
crates/blit-core/src/dial.rs:280:    /// call [`Self::resize_settled`] with the outcome; until then
crates/blit-core/src/dial.rs-281-    /// every subsequent tick returns `None`.
crates/blit-core/src/dial.rs-282-    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
crates/blit-core/src/dial.rs-283-        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
crates/blit-core/src/dial.rs-284-            return None;
crates/blit-core/src/dial.rs-285-        }
crates/blit-core/src/dial.rs-286-        let ticks = self
crates/blit-core/src/dial.rs-287-            .ticks_since_settle
crates/blit-core/src/dial.rs-288-            .fetch_add(1, Ordering::Relaxed)
crates/blit-core/src/dial.rs-289-            .saturating_add(1);
crates/blit-core/src/dial.rs-290-        if delta_bytes == 0 {
crates/blit-core/src/dial.rs-291-            self.resize_sustain.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-292-            return None;
crates/blit-core/src/dial.rs-293-        }
crates/blit-core/src/dial.rs-294-        let live = self.live_streams.load(Ordering::Relaxed).max(1);
crates/blit-core/src/dial.rs-295-        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
crates/blit-core/src/dial.rs-296-            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
crates/blit-core/src/dial.rs-297-            let next = prev.saturating_add(1);
crates/blit-core/src/dial.rs-298-            self.resize_sustain.store(next, Ordering::Relaxed);
--
crates/blit-core/src/dial.rs-307-            0
crates/blit-core/src/dial.rs-308-        };
crates/blit-core/src/dial.rs-309-        if ticks < RESIZE_COOLDOWN_TICKS {
crates/blit-core/src/dial.rs-310-            return None;
crates/blit-core/src/dial.rs-311-        }
crates/blit-core/src/dial.rs-312-        let target = if sustain >= RESIZE_SUSTAIN_TICKS {
crates/blit-core/src/dial.rs-313-            (live + 1).min(self.ceiling_max_streams.max(1))
crates/blit-core/src/dial.rs-314-        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
crates/blit-core/src/dial.rs-315-            live.saturating_sub(1).max(1)
crates/blit-core/src/dial.rs-316-        } else {
crates/blit-core/src/dial.rs-317-            return None;
crates/blit-core/src/dial.rs-318-        };
crates/blit-core/src/dial.rs-319-        if target == live {
crates/blit-core/src/dial.rs-320-            // Already at the bound in the wanted direction.
crates/blit-core/src/dial.rs-321-            self.resize_sustain.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-322-            return None;
crates/blit-core/src/dial.rs-323-        }
crates/blit-core/src/dial.rs-324-        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
crates/blit-core/src/dial.rs:325:        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
crates/blit-core/src/dial.rs-326-        // another task, and a plain store here could stack two live
crates/blit-core/src/dial.rs-327-        // proposals onto one epoch number.
crates/blit-core/src/dial.rs-328-        if self
crates/blit-core/src/dial.rs-329-            .pending_epoch
crates/blit-core/src/dial.rs-330-            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
crates/blit-core/src/dial.rs-331-            .is_err()
crates/blit-core/src/dial.rs-332-        {
crates/blit-core/src/dial.rs-333-            return None;
crates/blit-core/src/dial.rs-334-        }
crates/blit-core/src/dial.rs-335-        self.resize_sustain.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-336-        Some(ResizeProposal {
crates/blit-core/src/dial.rs-337-            epoch,
crates/blit-core/src/dial.rs-338-            target_streams: target,
crates/blit-core/src/dial.rs-339-            add: target > live,
crates/blit-core/src/dial.rs-340-        })
crates/blit-core/src/dial.rs-341-    }
crates/blit-core/src/dial.rs-342-
crates/blit-core/src/dial.rs-343-    /// sf-2: shape-correction proposal. On push the daemon proposes the
crates/blit-core/src/dial.rs-344-    /// epoch-0 stream count from whatever manifest prefix it has seen at
crates/blit-core/src/dial.rs-345-    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
crates/blit-core/src/dial.rs-346-    /// many-tiny-file push can negotiate far fewer streams than
crates/blit-core/src/dial.rs-347-    /// [`initial_stream_proposal`] assigns the full workload. As the
crates/blit-core/src/dial.rs-348-    /// need list accumulates client-side, the client re-runs the shape
crates/blit-core/src/dial.rs-349-    /// table and corrects upward through the normal resize wire.
crates/blit-core/src/dial.rs-350-    ///
crates/blit-core/src/dial.rs-351-    /// Unlike [`Self::resize_tick`] this is a definite signal — the
crates/blit-core/src/dial.rs-352-    /// shape is known, not inferred from throughput — so there is no
crates/blit-core/src/dial.rs-353-    /// sustain/cooldown discipline. It still honors one-in-flight and
crates/blit-core/src/dial.rs-354-    /// the receiver-profile ceiling, still moves ONE stream per epoch
crates/blit-core/src/dial.rs-355-    /// (the wire carries one `sub_token` per ADD), and never proposes
crates/blit-core/src/dial.rs-356-    /// REMOVE: shrinking below a live count is throughput evidence and
crates/blit-core/src/dial.rs-357-    /// stays the tuner's call.
crates/blit-core/src/dial.rs:358:    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
crates/blit-core/src/dial.rs-359-        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/dial.rs-360-        let live = self.live_streams.load(Ordering::Relaxed).max(1);
crates/blit-core/src/dial.rs-361-        if desired <= live {
crates/blit-core/src/dial.rs-362-            return None;
crates/blit-core/src/dial.rs-363-        }
crates/blit-core/src/dial.rs-364-        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
crates/blit-core/src/dial.rs-365-        if self
crates/blit-core/src/dial.rs-366-            .pending_epoch
crates/blit-core/src/dial.rs-367-            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
crates/blit-core/src/dial.rs-368-            .is_err()
crates/blit-core/src/dial.rs-369-        {
crates/blit-core/src/dial.rs-370-            return None;
crates/blit-core/src/dial.rs-371-        }
crates/blit-core/src/dial.rs-372-        Some(ResizeProposal {
crates/blit-core/src/dial.rs-373-            epoch,
crates/blit-core/src/dial.rs-374-            target_streams: live + 1,
crates/blit-core/src/dial.rs-375-            add: true,
crates/blit-core/src/dial.rs-376-        })
crates/blit-core/src/dial.rs-377-    }
crates/blit-core/src/dial.rs-378-
crates/blit-core/src/dial.rs-379-    /// Settle the in-flight proposal with what ACTUALLY happened:
crates/blit-core/src/dial.rs-380-    /// `effective_streams` is the live count now in effect (from the
crates/blit-core/src/dial.rs-381-    /// peer's ack, or the local count if a post-ack dial failed and
crates/blit-core/src/dial.rs:382:    /// nothing changed). `accepted = false` leaves the live count
crates/blit-core/src/dial.rs-383-    /// untouched. Stale epochs (not the pending one) are ignored.
crates/blit-core/src/dial.rs-384-    /// Either way the cooldown clock restarts.
crates/blit-core/src/dial.rs:385:    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
crates/blit-core/src/dial.rs-386-        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
crates/blit-core/src/dial.rs-387-            return;
crates/blit-core/src/dial.rs-388-        }
crates/blit-core/src/dial.rs-389-        self.pending_epoch.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-390-        self.ticks_since_settle.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-391-        self.resize_sustain.store(0, Ordering::Relaxed);
crates/blit-core/src/dial.rs-392-        if accepted {
crates/blit-core/src/dial.rs-393-            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/dial.rs-394-            self.live_streams.store(clamped, Ordering::Relaxed);
crates/blit-core/src/dial.rs-395-            self.resize_epoch.store(epoch, Ordering::Relaxed);
crates/blit-core/src/dial.rs-396-        }
crates/blit-core/src/dial.rs-397-    }
crates/blit-core/src/dial.rs-398-
crates/blit-core/src/dial.rs-399-    /// Raise max_streams toward the ceiling (used when a peer's
crates/blit-core/src/dial.rs-400-    /// negotiation allows more than the floor; still profile-bounded).
crates/blit-core/src/dial.rs-401-    pub fn allow_streams_up_to(&self, streams: usize) {
crates/blit-core/src/dial.rs-402-        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
crates/blit-core/src/dial.rs-403-        self.max_streams.store(clamped, Ordering::Relaxed);
--
crates/blit-core/src/dial.rs-590-            // the conservative-start contract. ue-r2-2 review (panel
crates/blit-core/src/dial.rs-591-            // F3): the idle tick must still reach `resize_tick` so a
crates/blit-core/src/dial.rs-592-            // sustain streak cannot survive a stall — "consecutive
crates/blit-core/src/dial.rs-593-            // busy ticks" means consecutive.
crates/blit-core/src/dial.rs-594-            if delta_bytes == 0 {
crates/blit-core/src/dial.rs-595-                if resize_tx.is_some() {
crates/blit-core/src/dial.rs-596-                    dial.resize_tick(0, 0.0);
crates/blit-core/src/dial.rs-597-                }
crates/blit-core/src/dial.rs-598-                continue;
crates/blit-core/src/dial.rs-599-            }
crates/blit-core/src/dial.rs-600-            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
crates/blit-core/src/dial.rs-601-            dial.apply_tick(ratio);
crates/blit-core/src/dial.rs-602-            if let Some(tx) = &resize_tx {
crates/blit-core/src/dial.rs-603-                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
crates/blit-core/src/dial.rs-604-                    if tx.send(proposal).is_err() {
crates/blit-core/src/dial.rs-605-                        // Controller gone (transfer tearing down):
crates/blit-core/src/dial.rs-606-                        // release the pending slot so the dial state
crates/blit-core/src/dial.rs-607-                        // stays honest for late readers.
crates/blit-core/src/dial.rs:608:                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs-609-                    }
crates/blit-core/src/dial.rs-610-                }
crates/blit-core/src/dial.rs-611-            }
crates/blit-core/src/dial.rs-612-        }
crates/blit-core/src/dial.rs-613-    })
crates/blit-core/src/dial.rs-614-}
crates/blit-core/src/dial.rs-615-
crates/blit-core/src/dial.rs-616-#[cfg(test)]
crates/blit-core/src/dial.rs-617-mod tests {
crates/blit-core/src/dial.rs-618-    use super::*;
crates/blit-core/src/dial.rs-619-
crates/blit-core/src/dial.rs-620-    fn profile(max_streams: u32, max_chunk: u64, max_inflight: u64) -> CapacityProfile {
crates/blit-core/src/dial.rs-621-        CapacityProfile {
crates/blit-core/src/dial.rs-622-            cpu_cores: 0,
crates/blit-core/src/dial.rs-623-            drain_class: 0,
crates/blit-core/src/dial.rs-624-            load_percent: 0,
crates/blit-core/src/dial.rs-625-            max_streams,
crates/blit-core/src/dial.rs-626-            drain_rate_bytes_per_sec: 0,
--
crates/blit-core/src/dial.rs-820-            .expect("sustained clean signal at maxed dials proposes");
crates/blit-core/src/dial.rs-821-        assert_eq!(
crates/blit-core/src/dial.rs-822-            proposal,
crates/blit-core/src/dial.rs-823-            ResizeProposal {
crates/blit-core/src/dial.rs-824-                epoch: 1,
crates/blit-core/src/dial.rs-825-                target_streams: 5,
crates/blit-core/src/dial.rs-826-                add: true
crates/blit-core/src/dial.rs-827-            }
crates/blit-core/src/dial.rs-828-        );
crates/blit-core/src/dial.rs-829-        assert!(dial.resize_pending());
crates/blit-core/src/dial.rs-830-
crates/blit-core/src/dial.rs-831-        // In flight: no further proposals regardless of signal.
crates/blit-core/src/dial.rs-832-        for _ in 0..8 {
crates/blit-core/src/dial.rs-833-            assert_eq!(dial.resize_tick(1024, 0.0), None, "pending blocks");
crates/blit-core/src/dial.rs-834-        }
crates/blit-core/src/dial.rs-835-
crates/blit-core/src/dial.rs-836-        // Accepted settle: live moves, epoch advances, cooldown blocks
crates/blit-core/src/dial.rs-837-        // the immediate next proposal even under a perfect signal.
crates/blit-core/src/dial.rs:838:        dial.resize_settled(1, 5, true);
crates/blit-core/src/dial.rs-839-        assert_eq!(dial.live_streams(), 5);
crates/blit-core/src/dial.rs-840-        assert_eq!(dial.resize_epoch(), 1);
crates/blit-core/src/dial.rs-841-        assert!(!dial.resize_pending());
crates/blit-core/src/dial.rs-842-        for _ in 0..(RESIZE_COOLDOWN_TICKS - 1) {
crates/blit-core/src/dial.rs-843-            assert_eq!(dial.resize_tick(1024, 0.0), None, "cooldown holds");
crates/blit-core/src/dial.rs-844-        }
crates/blit-core/src/dial.rs-845-        // Cooldown expired and the clean streak has been building the
crates/blit-core/src/dial.rs-846-        // whole time — the next clean tick proposes epoch 2.
crates/blit-core/src/dial.rs-847-        let next = dial.resize_tick(1024, 0.0).expect("epoch 2 proposes");
crates/blit-core/src/dial.rs-848-        assert_eq!(next.epoch, 2);
crates/blit-core/src/dial.rs-849-        assert_eq!(next.target_streams, 6);
crates/blit-core/src/dial.rs-850-    }
crates/blit-core/src/dial.rs-851-
crates/blit-core/src/dial.rs-852-    #[test]
crates/blit-core/src/dial.rs-853-    fn resize_remove_requires_floored_cheap_dials_and_floors_at_one() {
crates/blit-core/src/dial.rs-854-        let dial = TransferDial::conservative();
crates/blit-core/src/dial.rs-855-        dial.set_negotiated_streams(2);
crates/blit-core/src/dial.rs-856-        burn_cooldown(&dial);
crates/blit-core/src/dial.rs-857-
crates/blit-core/src/dial.rs-858-        // Blocked pipe with cheap dials at the floor (conservative
crates/blit-core/src/dial.rs-859-        // start IS the floor): two sustained ticks propose a drop.
crates/blit-core/src/dial.rs-860-        assert_eq!(dial.resize_tick(1024, 0.9), None, "sustain tick 1");
crates/blit-core/src/dial.rs-861-        let proposal = dial.resize_tick(1024, 0.9).expect("sustained block drops");
crates/blit-core/src/dial.rs-862-        assert_eq!(
crates/blit-core/src/dial.rs-863-            proposal,
crates/blit-core/src/dial.rs-864-            ResizeProposal {
crates/blit-core/src/dial.rs-865-                epoch: 1,
crates/blit-core/src/dial.rs-866-                target_streams: 1,
crates/blit-core/src/dial.rs-867-                add: false
crates/blit-core/src/dial.rs-868-            }
crates/blit-core/src/dial.rs-869-        );
crates/blit-core/src/dial.rs:870:        dial.resize_settled(1, 1, true);
crates/blit-core/src/dial.rs-871-        assert_eq!(dial.live_streams(), 1);
crates/blit-core/src/dial.rs-872-
crates/blit-core/src/dial.rs-873-        // At one stream, a blocked pipe can never drop to zero.
crates/blit-core/src/dial.rs-874-        burn_cooldown(&dial);
crates/blit-core/src/dial.rs-875-        for _ in 0..8 {
crates/blit-core/src/dial.rs-876-            assert_eq!(dial.resize_tick(1024, 0.9), None, "floor at 1");
crates/blit-core/src/dial.rs-877-        }
crates/blit-core/src/dial.rs-878-    }
crates/blit-core/src/dial.rs-879-
crates/blit-core/src/dial.rs-880-    #[test]
crates/blit-core/src/dial.rs-881-    fn resize_signal_interruptions_and_idle_reset_sustain() {
crates/blit-core/src/dial.rs-882-        let dial = TransferDial::conservative();
crates/blit-core/src/dial.rs-883-        dial.set_negotiated_streams(4);
crates/blit-core/src/dial.rs-884-        while dial.step_up_cheap_dials() {}
crates/blit-core/src/dial.rs-885-        burn_cooldown(&dial);
crates/blit-core/src/dial.rs-886-
crates/blit-core/src/dial.rs-887-        // clean → idle → clean: the idle tick resets the streak, so
crates/blit-core/src/dial.rs-888-        // the second clean tick is streak 1, not 2.
crates/blit-core/src/dial.rs-889-        assert_eq!(dial.resize_tick(1024, 0.0), None);
crates/blit-core/src/dial.rs-890-        assert_eq!(dial.resize_tick(0, 0.0), None, "idle resets");
crates/blit-core/src/dial.rs-891-        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
crates/blit-core/src/dial.rs-892-        // clean → in-band → clean: same reset.
crates/blit-core/src/dial.rs-893-        assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band resets");
crates/blit-core/src/dial.rs-894-        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
crates/blit-core/src/dial.rs-895-        assert!(dial.resize_tick(1024, 0.0).is_some(), "streak completes");
crates/blit-core/src/dial.rs-896-    }
crates/blit-core/src/dial.rs-897-
crates/blit-core/src/dial.rs-898-    #[test]
crates/blit-core/src/dial.rs:899:    fn resize_refusal_keeps_live_count_and_stale_settles_are_ignored() {
crates/blit-core/src/dial.rs-900-        let dial = TransferDial::conservative();
crates/blit-core/src/dial.rs-901-        dial.set_negotiated_streams(4);
crates/blit-core/src/dial.rs-902-        while dial.step_up_cheap_dials() {}
crates/blit-core/src/dial.rs-903-        burn_cooldown(&dial);
crates/blit-core/src/dial.rs-904-        assert_eq!(dial.resize_tick(1024, 0.0), None);
crates/blit-core/src/dial.rs-905-        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");
crates/blit-core/src/dial.rs-906-
crates/blit-core/src/dial.rs-907-        // A stale/foreign epoch must not clear the pending slot.
crates/blit-core/src/dial.rs:908:        dial.resize_settled(proposal.epoch + 7, 9, true);
crates/blit-core/src/dial.rs-909-        assert!(dial.resize_pending(), "stale settle ignored");
crates/blit-core/src/dial.rs-910-
crates/blit-core/src/dial.rs-911-        // Refusal: pending clears, live count and epoch stay put.
crates/blit-core/src/dial.rs:912:        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs-913-        assert!(!dial.resize_pending());
crates/blit-core/src/dial.rs-914-        assert_eq!(dial.live_streams(), 4);
crates/blit-core/src/dial.rs:915:        assert_eq!(dial.resize_epoch(), 0, "refused epoch never settles");
crates/blit-core/src/dial.rs-916-    }
crates/blit-core/src/dial.rs-917-
crates/blit-core/src/dial.rs-918-    #[test]
crates/blit-core/src/dial.rs-919-    fn resize_target_clamps_to_the_profile_ceiling() {
crates/blit-core/src/dial.rs-920-        let dial = TransferDial::conservative_within(Some(&profile(4, 0, 0)));
crates/blit-core/src/dial.rs-921-        dial.set_negotiated_streams(4); // already at the profile ceiling
crates/blit-core/src/dial.rs-922-        while dial.step_up_cheap_dials() {}
crates/blit-core/src/dial.rs-923-        burn_cooldown(&dial);
crates/blit-core/src/dial.rs-924-        for _ in 0..8 {
crates/blit-core/src/dial.rs-925-            assert_eq!(
crates/blit-core/src/dial.rs-926-                dial.resize_tick(1024, 0.0),
crates/blit-core/src/dial.rs-927-                None,
crates/blit-core/src/dial.rs-928-                "cannot add past the receiver's advertised ceiling"
crates/blit-core/src/dial.rs-929-            );
crates/blit-core/src/dial.rs-930-        }
crates/blit-core/src/dial.rs-931-    }
crates/blit-core/src/dial.rs-932-
crates/blit-core/src/dial.rs:933:    // ── sf-2 shape-correction resize ─────────────────────────────────
crates/blit-core/src/dial.rs-934-
crates/blit-core/src/dial.rs-935-    /// The plan's three measured 10 GbE cells mapped through the shape
crates/blit-core/src/dial.rs-936-    /// table (`docs/plan/SMALL_FILE_CEILING.md`): the small and mixed
crates/blit-core/src/dial.rs-937-    /// cells must NOT ride the byte tiers alone.
crates/blit-core/src/dial.rs-938-    #[test]
crates/blit-core/src/dial.rs-939-    fn shape_table_covers_the_small_file_ceiling_cells() {
crates/blit-core/src/dial.rs-940-        const KIB: u64 = 1024;
crates/blit-core/src/dial.rs-941-        const MIB64: u64 = 1024 * KIB;
crates/blit-core/src/dial.rs-942-        const GIB: u64 = 1024 * MIB64;
crates/blit-core/src/dial.rs-943-        // push/pull 10k × 4 KiB: 40 MiB is the 2-stream byte tier, but
crates/blit-core/src/dial.rs-944-        // 10_000 files must key the 8-stream file-count tier.
crates/blit-core/src/dial.rs-945-        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
crates/blit-core/src/dial.rs-946-        // 1 × 1 GiB: byte-keyed, file count is irrelevant — unchanged.
crates/blit-core/src/dial.rs-947-        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
crates/blit-core/src/dial.rs-948-        // mixed 512 MiB + 5k × 2 KiB: the byte tier already reaches 8;
crates/blit-core/src/dial.rs-949-        // the 5_001 files alone would say 4 — bytes win.
crates/blit-core/src/dial.rs-950-        assert_eq!(
crates/blit-core/src/dial.rs-951-            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
crates/blit-core/src/dial.rs-952-            8
crates/blit-core/src/dial.rs-953-        );
crates/blit-core/src/dial.rs-954-        // sf-1 loopback probe evidence: 1_000 tiny files must propose 2
crates/blit-core/src/dial.rs-955-        // (the measured transfer rode 1 — the input, not this table,
crates/blit-core/src/dial.rs-956-        // was wrong).
crates/blit-core/src/dial.rs-957-        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
crates/blit-core/src/dial.rs-958-    }
crates/blit-core/src/dial.rs-959-
crates/blit-core/src/dial.rs-960-    #[test]
crates/blit-core/src/dial.rs:961:    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
crates/blit-core/src/dial.rs-962-        let dial = TransferDial::conservative();
crates/blit-core/src/dial.rs-963-        dial.set_negotiated_streams(1);
crates/blit-core/src/dial.rs-964-
crates/blit-core/src/dial.rs-965-        // At or below live: nothing to correct.
crates/blit-core/src/dial.rs:966:        assert_eq!(dial.propose_shape_resize(0), None);
crates/blit-core/src/dial.rs:967:        assert_eq!(dial.propose_shape_resize(1), None);
crates/blit-core/src/dial.rs-968-
crates/blit-core/src/dial.rs-969-        // Target 3 from live 1: epoch 1 proposes 2 (one per epoch),
crates/blit-core/src/dial.rs-970-        // and the in-flight epoch blocks both proposers.
crates/blit-core/src/dial.rs:971:        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
crates/blit-core/src/dial.rs-972-        assert_eq!(
crates/blit-core/src/dial.rs-973-            p1,
crates/blit-core/src/dial.rs-974-            ResizeProposal {
crates/blit-core/src/dial.rs-975-                epoch: 1,
crates/blit-core/src/dial.rs-976-                target_streams: 2,
crates/blit-core/src/dial.rs-977-                add: true
crates/blit-core/src/dial.rs-978-            }
crates/blit-core/src/dial.rs-979-        );
crates/blit-core/src/dial.rs:980:        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
crates/blit-core/src/dial.rs-981-        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
crates/blit-core/src/dial.rs-982-
crates/blit-core/src/dial.rs-983-        // Settle → next step; no cooldown for the definite shape signal.
crates/blit-core/src/dial.rs:984:        dial.resize_settled(1, 2, true);
crates/blit-core/src/dial.rs:985:        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
crates/blit-core/src/dial.rs-986-        assert_eq!(p2.epoch, 2);
crates/blit-core/src/dial.rs-987-        assert_eq!(p2.target_streams, 3);
crates/blit-core/src/dial.rs:988:        dial.resize_settled(2, 3, true);
crates/blit-core/src/dial.rs-989-        assert_eq!(dial.live_streams(), 3);
crates/blit-core/src/dial.rs:990:        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
crates/blit-core/src/dial.rs-991-
crates/blit-core/src/dial.rs:992:        // A refused epoch leaves live untouched; the next call retries.
crates/blit-core/src/dial.rs:993:        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
crates/blit-core/src/dial.rs:994:        dial.resize_settled(p3.epoch, dial.live_streams(), false);
crates/blit-core/src/dial.rs-995-        assert_eq!(dial.live_streams(), 3);
crates/blit-core/src/dial.rs-996-        assert!(
crates/blit-core/src/dial.rs:997:            dial.propose_shape_resize(4).is_some(),
crates/blit-core/src/dial.rs:998:            "retry after refusal"
crates/blit-core/src/dial.rs-999-        );
crates/blit-core/src/dial.rs-1000-    }
crates/blit-core/src/dial.rs-1001-
crates/blit-core/src/dial.rs-1002-    #[test]
crates/blit-core/src/dial.rs:1003:    fn shape_resize_clamps_to_the_profile_ceiling() {
crates/blit-core/src/dial.rs-1004-        let dial = TransferDial::conservative_within(Some(&profile(2, 0, 0)));
crates/blit-core/src/dial.rs-1005-        dial.set_negotiated_streams(1);
crates/blit-core/src/dial.rs-1006-        let p = dial
crates/blit-core/src/dial.rs:1007:            .propose_shape_resize(100)
crates/blit-core/src/dial.rs:1008:            .expect("clamped, not refused");
crates/blit-core/src/dial.rs-1009-        assert_eq!(p.target_streams, 2);
crates/blit-core/src/dial.rs:1010:        dial.resize_settled(p.epoch, 2, true);
crates/blit-core/src/dial.rs-1011-        assert_eq!(
crates/blit-core/src/dial.rs:1012:            dial.propose_shape_resize(100),
crates/blit-core/src/dial.rs-1013-            None,
crates/blit-core/src/dial.rs-1014-            "at the receiver's advertised ceiling"
crates/blit-core/src/dial.rs-1015-        );
crates/blit-core/src/dial.rs-1016-    }
crates/blit-core/src/dial.rs-1017-
crates/blit-core/src/dial.rs-1018-    #[tokio::test(start_paused = true)]
crates/blit-core/src/dial.rs-1019-    async fn tuner_forwards_resize_proposals_over_the_shared_registry() {
crates/blit-core/src/dial.rs-1020-        use crate::remote::transfer::progress::{StreamId, StreamProbe};
crates/blit-core/src/dial.rs-1021-        let dial = TransferDial::conservative().shared();
crates/blit-core/src/dial.rs-1022-        dial.set_negotiated_streams(2);
crates/blit-core/src/dial.rs-1023-        while dial.step_up_cheap_dials() {}
crates/blit-core/src/dial.rs-1024-        let probe = StreamProbe::new(StreamId(0));
crates/blit-core/src/dial.rs-1025-        let registry: SharedStreamProbes =
crates/blit-core/src/dial.rs-1026-            Arc::new(std::sync::Mutex::new(vec![StreamProbe::from_telemetry(
crates/blit-core/src/dial.rs-1027-                probe.id(),
crates/blit-core/src/dial.rs-1028-                probe.telemetry(),
crates/blit-core/src/dial.rs-1029-            )]));
crates/blit-core/src/dial.rs-1030-        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
--
crates/blit-core/tests/transfer_session_roles.rs-890-                assert_eq!(list.relative_path, "partial.bin");
crates/blit-core/tests/transfer_session_roles.rs-891-                assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-892-                    list.block_size, RESUME_BS,
crates/blit-core/tests/transfer_session_roles.rs-893-                    "an in-range open block_size must ride the wire unclamped"
crates/blit-core/tests/transfer_session_roles.rs-894-                );
crates/blit-core/tests/transfer_session_roles.rs-895-                saw_hashes = true;
crates/blit-core/tests/transfer_session_roles.rs-896-            }
crates/blit-core/tests/transfer_session_roles.rs-897-            Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-898-            other => panic!("expected need choreography, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-899-        }
crates/blit-core/tests/transfer_session_roles.rs-900-    }
crates/blit-core/tests/transfer_session_roles.rs-901-
crates/blit-core/tests/transfer_session_roles.rs-902-    // The violation: a whole-file record for the resume-flagged path.
crates/blit-core/tests/transfer_session_roles.rs-903-    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-904-
crates/blit-core/tests/transfer_session_roles.rs-905-    // Bounded wait: a regression here (accepting the record) leaves the
crates/blit-core/tests/transfer_session_roles.rs-906-    // destination blocked on FileData frames this peer never sends —
crates/blit-core/tests/transfer_session_roles.rs-907-    // the pin must fail on the clock, not hang the suite.
crates/blit-core/tests/transfer_session_roles.rs:908:    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs-909-        loop {
crates/blit-core/tests/transfer_session_roles.rs-910-            match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-911-                Frame::Error(e) => break e,
crates/blit-core/tests/transfer_session_roles.rs-912-                Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-913-                other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-914-            }
crates/blit-core/tests/transfer_session_roles.rs-915-        }
crates/blit-core/tests/transfer_session_roles.rs-916-    })
crates/blit-core/tests/transfer_session_roles.rs-917-    .await
crates/blit-core/tests/transfer_session_roles.rs-918-    .expect("the violation must be answered promptly, not absorbed");
crates/blit-core/tests/transfer_session_roles.rs:919:    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
crates/blit-core/tests/transfer_session_roles.rs-920-    let dest_err = dest.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-921-    let fault = fault_of(&dest_err);
crates/blit-core/tests/transfer_session_roles.rs-922-    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs-923-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-924-        fault.message.contains("resume-flagged"),
crates/blit-core/tests/transfer_session_roles.rs-925-        "got: {}",
crates/blit-core/tests/transfer_session_roles.rs-926-        fault.message
crates/blit-core/tests/transfer_session_roles.rs-927-    );
crates/blit-core/tests/transfer_session_roles.rs-928-}
crates/blit-core/tests/transfer_session_roles.rs-929-
crates/blit-core/tests/transfer_session_roles.rs-930-/// otp-7a fault injection: a source whose reader for one path yields
crates/blit-core/tests/transfer_session_roles.rs-931-/// only the first `limit` bytes and then EOF, provably short of the
crates/blit-core/tests/transfer_session_roles.rs-932-/// manifested size — the session's mid-record fault (the same EOF-short
crates/blit-core/tests/transfer_session_roles.rs-933-/// abort a whole-file record has).
crates/blit-core/tests/transfer_session_roles.rs-934-struct TruncatedReadSource {
crates/blit-core/tests/transfer_session_roles.rs-935-    inner: FsTransferSource,
crates/blit-core/tests/transfer_session_roles.rs-936-    fail_path: &'static str,
crates/blit-core/tests/transfer_session_roles.rs-937-    limit: u64,
--
crates/blit-core/tests/transfer_session_roles.rs-1149-
crates/blit-core/tests/transfer_session_roles.rs-1150-    let source_err = source_task.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-1151-    let fault = fault_of(&source_err);
crates/blit-core/tests/transfer_session_roles.rs-1152-    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs-1153-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-1154-        fault.message.contains("without a held resume need"),
crates/blit-core/tests/transfer_session_roles.rs-1155-        "got: {}",
crates/blit-core/tests/transfer_session_roles.rs-1156-        fault.message
crates/blit-core/tests/transfer_session_roles.rs-1157-    );
crates/blit-core/tests/transfer_session_roles.rs-1158-}
crates/blit-core/tests/transfer_session_roles.rs-1159-
crates/blit-core/tests/transfer_session_roles.rs-1160-#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
crates/blit-core/tests/transfer_session_roles.rs-1161-async fn many_tiny_files_reach_shape_target_when_source_initiates() {
crates/blit-core/tests/transfer_session_roles.rs-1162-    // sf-2 pin ported onto the unified session (otp-4b-2). The responder
crates/blit-core/tests/transfer_session_roles.rs-1163-    // grants the zero-knowledge single stream (no manifest seen at
crates/blit-core/tests/transfer_session_roles.rs-1164-    // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
crates/blit-core/tests/transfer_session_roles.rs-1165-    // must re-run the shape table over the accumulated need list and grow
crates/blit-core/tests/transfer_session_roles.rs-1166-    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
crates/blit-core/tests/transfer_session_roles.rs:1167:    // push sf-2 pin (`shape_resize_e2e.rs`), now on the session: the
crates/blit-core/tests/transfer_session_roles.rs-1168-    // settled count is read from the destination's `data_plane_streams`.
crates/blit-core/tests/transfer_session_roles.rs-1169-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1170-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1171-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1172-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1173-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1174-    const FILE_COUNT: usize = 10_000;
crates/blit-core/tests/transfer_session_roles.rs-1175-    for i in 0..FILE_COUNT {
crates/blit-core/tests/transfer_session_roles.rs-1176-        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1177-    }
crates/blit-core/tests/transfer_session_roles.rs-1178-
crates/blit-core/tests/transfer_session_roles.rs-1179-    // SOURCE initiator over the TCP data plane: the control lane rides the
crates/blit-core/tests/transfer_session_roles.rs-1180-    // in-process transport; the data-plane sockets ride loopback TCP (the
crates/blit-core/tests/transfer_session_roles.rs-1181-    // responder binds 0.0.0.0:0 and the source dials 127.0.0.1).
crates/blit-core/tests/transfer_session_roles.rs-1182-    let open = SessionOpen {
crates/blit-core/tests/transfer_session_roles.rs-1183-        initiator_role: TransferRole::Source as i32,
crates/blit-core/tests/transfer_session_roles.rs-1184-        compare_mode: ComparisonMode::SizeMtime as i32,
crates/blit-core/tests/transfer_session_roles.rs-1185-        in_stream_bytes: false,
--
crates/blit-core/tests/transfer_session_roles.rs-1223-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1224-        streams, 8,
crates/blit-core/tests/transfer_session_roles.rs-1225-        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
crates/blit-core/tests/transfer_session_roles.rs-1226-         target regardless of which endpoint initiated the session"
crates/blit-core/tests/transfer_session_roles.rs-1227-    );
crates/blit-core/tests/transfer_session_roles.rs-1228-    assert_trees_identical(&src_root, &dst_root);
crates/blit-core/tests/transfer_session_roles.rs-1229-}
crates/blit-core/tests/transfer_session_roles.rs-1230-
crates/blit-core/tests/transfer_session_roles.rs-1231-#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
crates/blit-core/tests/transfer_session_roles.rs-1232-async fn pull_data_plane_single_stream_lands_bytes() {
crates/blit-core/tests/transfer_session_roles.rs-1233-    // otp-5b-1: the transport/role decoupling in the PULL direction — the
crates/blit-core/tests/transfer_session_roles.rs-1234-    // mirror of the push data-plane test above. Here the DESTINATION is the
crates/blit-core/tests/transfer_session_roles.rs-1235-    // *initiator* (dials + receives) and the SOURCE is the *responder*
crates/blit-core/tests/transfer_session_roles.rs-1236-    // (binds + accepts + sends). Control frames ride the in-process
crates/blit-core/tests/transfer_session_roles.rs-1237-    // transport; the data-plane socket rides loopback TCP (the SOURCE
crates/blit-core/tests/transfer_session_roles.rs-1238-    // responder binds 0.0.0.0:0, the DESTINATION initiator dials
crates/blit-core/tests/transfer_session_roles.rs-1239-    // 127.0.0.1). Single-stream because this 4-file tree's shape wants only
crates/blit-core/tests/transfer_session_roles.rs-1240-    // one stream — the pull data plane CAN resize (otp-5b-2), but a small
crates/blit-core/tests/transfer_session_roles.rs:1241:    // need list never crosses the shape threshold; the resize itself is
crates/blit-core/tests/transfer_session_roles.rs-1242-    // pinned by `many_tiny_files_reach_shape_target_when_destination_initiates`.
crates/blit-core/tests/transfer_session_roles.rs-1243-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1244-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1245-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1246-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1247-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1248-    write_tree(
crates/blit-core/tests/transfer_session_roles.rs-1249-        &src_root,
crates/blit-core/tests/transfer_session_roles.rs-1250-        &[
crates/blit-core/tests/transfer_session_roles.rs-1251-            ("a.txt", b"alpha".to_vec(), 1_600_000_001),
crates/blit-core/tests/transfer_session_roles.rs-1252-            ("empty.bin", b"".to_vec(), 1_600_000_002),
crates/blit-core/tests/transfer_session_roles.rs-1253-            ("dir/b.log", b"beta beta beta".to_vec(), 1_600_000_003),
crates/blit-core/tests/transfer_session_roles.rs-1254-            ("dir/deep/c.dat", b"gamma-content".to_vec(), 1_600_000_004),
crates/blit-core/tests/transfer_session_roles.rs-1255-        ],
crates/blit-core/tests/transfer_session_roles.rs-1256-    );
crates/blit-core/tests/transfer_session_roles.rs-1257-
crates/blit-core/tests/transfer_session_roles.rs-1258-    // DESTINATION initiator; SOURCE responder — the roles flipped from the
crates/blit-core/tests/transfer_session_roles.rs-1259-    // push data-plane test, the data plane following connection role.
--
crates/blit-core/tests/transfer_session_roles.rs-1299-        "both ends must hold the same summary"
crates/blit-core/tests/transfer_session_roles.rs-1300-    );
crates/blit-core/tests/transfer_session_roles.rs-1301-    assert_eq!(outcome.summary.files_transferred, 4);
crates/blit-core/tests/transfer_session_roles.rs-1302-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1303-        outcome.data_plane_streams,
crates/blit-core/tests/transfer_session_roles.rs-1304-        Some(1),
crates/blit-core/tests/transfer_session_roles.rs-1305-        "a 4-file need list stays single-stream (below the shape threshold)"
crates/blit-core/tests/transfer_session_roles.rs-1306-    );
crates/blit-core/tests/transfer_session_roles.rs-1307-    assert_trees_identical(&src_root, &dst_root);
crates/blit-core/tests/transfer_session_roles.rs-1308-}
crates/blit-core/tests/transfer_session_roles.rs-1309-
crates/blit-core/tests/transfer_session_roles.rs-1310-#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
crates/blit-core/tests/transfer_session_roles.rs-1311-async fn many_tiny_files_reach_shape_target_when_destination_initiates() {
crates/blit-core/tests/transfer_session_roles.rs-1312-    // otp-5b-2: the sf-2 shape correction in the PULL direction — the
crates/blit-core/tests/transfer_session_roles.rs-1313-    // mirror of `many_tiny_files_reach_shape_target_when_source_initiates`
crates/blit-core/tests/transfer_session_roles.rs-1314-    // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
crates/blit-core/tests/transfer_session_roles.rs-1315-    // sockets it grows to) and the SOURCE is the *responder* (accepts them
crates/blit-core/tests/transfer_session_roles.rs-1316-    // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
crates/blit-core/tests/transfer_session_roles.rs:1317:    // `DataPlaneResizeAck` frames are identical to push; only the transport
crates/blit-core/tests/transfer_session_roles.rs-1318-    // action flips. A 10k-tiny-file transfer must re-run the shape table
crates/blit-core/tests/transfer_session_roles.rs-1319-    // over the accumulated need list and grow the stream count past 1.
crates/blit-core/tests/transfer_session_roles.rs-1320-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1321-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1322-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1323-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1324-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1325-    const FILE_COUNT: usize = 10_000;
crates/blit-core/tests/transfer_session_roles.rs-1326-    for i in 0..FILE_COUNT {
crates/blit-core/tests/transfer_session_roles.rs-1327-        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1328-    }
crates/blit-core/tests/transfer_session_roles.rs-1329-
crates/blit-core/tests/transfer_session_roles.rs-1330-    // DESTINATION initiator; SOURCE responder — roles flipped from the push
crates/blit-core/tests/transfer_session_roles.rs-1331-    // shape test, the data plane following connection role.
crates/blit-core/tests/transfer_session_roles.rs-1332-    let open = SessionOpen {
crates/blit-core/tests/transfer_session_roles.rs-1333-        initiator_role: TransferRole::Destination as i32,
crates/blit-core/tests/transfer_session_roles.rs-1334-        compare_mode: ComparisonMode::SizeMtime as i32,
crates/blit-core/tests/transfer_session_roles.rs-1335-        in_stream_bytes: false,
--
crates/blit-core/tests/transfer_session_roles.rs-1401-    );
crates/blit-core/tests/transfer_session_roles.rs-1402-
crates/blit-core/tests/transfer_session_roles.rs-1403-    let (source_result, dest_result) = run_session(
crates/blit-core/tests/transfer_session_roles.rs-1404-        TransferRole::Source,
crates/blit-core/tests/transfer_session_roles.rs-1405-        &src_root,
crates/blit-core/tests/transfer_session_roles.rs-1406-        &dst_root,
crates/blit-core/tests/transfer_session_roles.rs-1407-        PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs-1408-    )
crates/blit-core/tests/transfer_session_roles.rs-1409-    .await;
crates/blit-core/tests/transfer_session_roles.rs-1410-    source_result.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1411-    dest_result.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1412-
crates/blit-core/tests/transfer_session_roles.rs-1413-    let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1414-    let mtime = filetime::FileTime::from_last_modification_time(&meta);
crates/blit-core/tests/transfer_session_roles.rs-1415-    assert_eq!(mtime.unix_seconds(), 1_555_555_555);
crates/blit-core/tests/transfer_session_roles.rs-1416-}
crates/blit-core/tests/transfer_session_roles.rs-1417-
crates/blit-core/tests/transfer_session_roles.rs-1418-// ---------------------------------------------------------------------------
crates/blit-core/tests/transfer_session_roles.rs:1419:// Handshake refusals
crates/blit-core/tests/transfer_session_roles.rs-1420-// ---------------------------------------------------------------------------
crates/blit-core/tests/transfer_session_roles.rs-1421-
crates/blit-core/tests/transfer_session_roles.rs-1422-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:1423:async fn build_mismatch_refused_under_both_initiators() {
crates/blit-core/tests/transfer_session_roles.rs-1424-    for initiator_role in [TransferRole::Source, TransferRole::Destination] {
crates/blit-core/tests/transfer_session_roles.rs-1425-        let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1426-        let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1427-        let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1428-        std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1429-        std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1430-
crates/blit-core/tests/transfer_session_roles.rs-1431-        let open = basic_open(initiator_role);
crates/blit-core/tests/transfer_session_roles.rs-1432-        let (source_endpoint, dest_endpoint) = match initiator_role {
crates/blit-core/tests/transfer_session_roles.rs-1433-            TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
crates/blit-core/tests/transfer_session_roles.rs-1434-            _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
crates/blit-core/tests/transfer_session_roles.rs-1435-        };
crates/blit-core/tests/transfer_session_roles.rs-1436-        let source_cfg = SourceSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1437-            instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-1438-            hello: HelloConfig {
crates/blit-core/tests/transfer_session_roles.rs-1439-                build_id: "0.1.0+aaaaaaaaaaaa".into(),
crates/blit-core/tests/transfer_session_roles.rs-1440-                contract_version: CONTRACT_VERSION,
crates/blit-core/tests/transfer_session_roles.rs-1441-            },
--
crates/blit-core/tests/transfer_session_roles.rs-1457-        let source = Arc::new(FsTransferSource::new(src_root.clone()));
crates/blit-core/tests/transfer_session_roles.rs-1458-        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs-1459-            tokio::join!(
crates/blit-core/tests/transfer_session_roles.rs-1460-                run_source(source_cfg, a, source),
crates/blit-core/tests/transfer_session_roles.rs-1461-                run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
crates/blit-core/tests/transfer_session_roles.rs-1462-            )
crates/blit-core/tests/transfer_session_roles.rs-1463-        })
crates/blit-core/tests/transfer_session_roles.rs-1464-        .await
crates/blit-core/tests/transfer_session_roles.rs-1465-        .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1466-
crates/blit-core/tests/transfer_session_roles.rs-1467-        for (end, err) in [
crates/blit-core/tests/transfer_session_roles.rs-1468-            ("source", source_result.unwrap_err()),
crates/blit-core/tests/transfer_session_roles.rs-1469-            ("destination", dest_result.err().unwrap()),
crates/blit-core/tests/transfer_session_roles.rs-1470-        ] {
crates/blit-core/tests/transfer_session_roles.rs-1471-            let fault = fault_of(&err);
crates/blit-core/tests/transfer_session_roles.rs-1472-            assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1473-                fault.code,
crates/blit-core/tests/transfer_session_roles.rs-1474-                session_error::Code::BuildMismatch,
crates/blit-core/tests/transfer_session_roles.rs:1475:                "{end} must refuse with BUILD_MISMATCH (initiator {initiator_role:?})"
crates/blit-core/tests/transfer_session_roles.rs-1476-            );
crates/blit-core/tests/transfer_session_roles.rs-1477-            assert!(
crates/blit-core/tests/transfer_session_roles.rs-1478-                fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
crates/blit-core/tests/transfer_session_roles.rs-1479-                "{end} must name both build ids, got: {}",
crates/blit-core/tests/transfer_session_roles.rs-1480-                fault.message
crates/blit-core/tests/transfer_session_roles.rs-1481-            );
crates/blit-core/tests/transfer_session_roles.rs-1482-        }
crates/blit-core/tests/transfer_session_roles.rs-1483-        assert!(
crates/blit-core/tests/transfer_session_roles.rs-1484-            collect_tree(&dst_root).is_empty(),
crates/blit-core/tests/transfer_session_roles.rs:1485:            "no bytes may move on a refused handshake"
crates/blit-core/tests/transfer_session_roles.rs-1486-        );
crates/blit-core/tests/transfer_session_roles.rs-1487-    }
crates/blit-core/tests/transfer_session_roles.rs-1488-}
crates/blit-core/tests/transfer_session_roles.rs-1489-
crates/blit-core/tests/transfer_session_roles.rs-1490-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:1491:async fn contract_version_mismatch_is_refused() {
crates/blit-core/tests/transfer_session_roles.rs-1492-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1493-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1494-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1495-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1496-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1497-
crates/blit-core/tests/transfer_session_roles.rs-1498-    let source_cfg = SourceSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1499-        instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-1500-        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs-1501-        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs-1502-        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs-1503-        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs-1504-    };
crates/blit-core/tests/transfer_session_roles.rs-1505-    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1506-        hello: HelloConfig {
crates/blit-core/tests/transfer_session_roles.rs-1507-            build_id: HelloConfig::default().build_id,
crates/blit-core/tests/transfer_session_roles.rs-1508-            contract_version: CONTRACT_VERSION + 1,
crates/blit-core/tests/transfer_session_roles.rs-1509-        },
--
crates/blit-core/tests/transfer_session_roles.rs-1514-    };
crates/blit-core/tests/transfer_session_roles.rs-1515-    let (a, b) = in_process_pair();
crates/blit-core/tests/transfer_session_roles.rs-1516-    let source = Arc::new(FsTransferSource::new(src_root));
crates/blit-core/tests/transfer_session_roles.rs-1517-    let (source_result, dest_result) = tokio::join!(
crates/blit-core/tests/transfer_session_roles.rs-1518-        run_source(source_cfg, a, source),
crates/blit-core/tests/transfer_session_roles.rs-1519-        run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
crates/blit-core/tests/transfer_session_roles.rs-1520-    );
crates/blit-core/tests/transfer_session_roles.rs-1521-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1522-        fault_of(&source_result.unwrap_err()).code,
crates/blit-core/tests/transfer_session_roles.rs-1523-        session_error::Code::BuildMismatch
crates/blit-core/tests/transfer_session_roles.rs-1524-    );
crates/blit-core/tests/transfer_session_roles.rs-1525-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1526-        fault_of(&dest_result.err().unwrap()).code,
crates/blit-core/tests/transfer_session_roles.rs-1527-        session_error::Code::BuildMismatch
crates/blit-core/tests/transfer_session_roles.rs-1528-    );
crates/blit-core/tests/transfer_session_roles.rs-1529-}
crates/blit-core/tests/transfer_session_roles.rs-1530-
crates/blit-core/tests/transfer_session_roles.rs-1531-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:1532:async fn mirror_enabled_without_scope_is_refused() {
crates/blit-core/tests/transfer_session_roles.rs-1533-    // otp-6b: a mirror-enabled open with no concrete scope (kind defaults to
crates/blit-core/tests/transfer_session_roles.rs:1534:    // UNSPECIFIED) is a contradiction — refuse it at OPEN with a protocol
crates/blit-core/tests/transfer_session_roles.rs-1535-    // violation, from the destination (the end that executes deletions).
crates/blit-core/tests/transfer_session_roles.rs-1536-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1537-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-1538-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1539-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1540-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1541-
crates/blit-core/tests/transfer_session_roles.rs-1542-    let mut open = basic_open(TransferRole::Source);
crates/blit-core/tests/transfer_session_roles.rs-1543-    open.mirror_enabled = true; // no mirror_kind set → UNSPECIFIED
crates/blit-core/tests/transfer_session_roles.rs-1544-    let source_cfg = SourceSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1545-        instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-1546-        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs-1547-        endpoint: SessionEndpoint::initiator(open),
crates/blit-core/tests/transfer_session_roles.rs-1548-        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs-1549-        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs-1550-    };
crates/blit-core/tests/transfer_session_roles.rs-1551-    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1552-        hello: HelloConfig::default(),
--
crates/blit-core/tests/transfer_session_roles.rs-1711-        &dst_root,
crates/blit-core/tests/transfer_session_roles.rs-1712-        MirrorMode::All,
crates/blit-core/tests/transfer_session_roles.rs-1713-        Some("*.txt"),
crates/blit-core/tests/transfer_session_roles.rs-1714-    )
crates/blit-core/tests/transfer_session_roles.rs-1715-    .await;
crates/blit-core/tests/transfer_session_roles.rs-1716-    let summary = sr.expect("source session");
crates/blit-core/tests/transfer_session_roles.rs-1717-    let _ = dr.expect("destination session");
crates/blit-core/tests/transfer_session_roles.rs-1718-
crates/blit-core/tests/transfer_session_roles.rs-1719-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-1720-        summary.entries_deleted, 2,
crates/blit-core/tests/transfer_session_roles.rs-1721-        "stale.txt and out-of-scope keep.log"
crates/blit-core/tests/transfer_session_roles.rs-1722-    );
crates/blit-core/tests/transfer_session_roles.rs-1723-    assert!(!dst_root.join("stale.txt").exists());
crates/blit-core/tests/transfer_session_roles.rs-1724-    assert!(!dst_root.join("keep.log").exists());
crates/blit-core/tests/transfer_session_roles.rs-1725-    assert!(dst_root.join("keep.txt").exists());
crates/blit-core/tests/transfer_session_roles.rs-1726-}
crates/blit-core/tests/transfer_session_roles.rs-1727-
crates/blit-core/tests/transfer_session_roles.rs-1728-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:1729:async fn mirror_refused_when_source_scan_incomplete() {
crates/blit-core/tests/transfer_session_roles.rs-1730-    // otp-6b: mirroring on an incomplete source scan could delete files the
crates/blit-core/tests/transfer_session_roles.rs-1731-    // source still has (they were merely unreadable mid-scan). The
crates/blit-core/tests/transfer_session_roles.rs:1732:    // destination must refuse at ManifestComplete{scan_complete=false} and
crates/blit-core/tests/transfer_session_roles.rs-1733-    // delete nothing. Scripted source peer so we control the flag.
crates/blit-core/tests/transfer_session_roles.rs-1734-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1735-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1736-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1737-    write_tree(
crates/blit-core/tests/transfer_session_roles.rs-1738-        &dst_root,
crates/blit-core/tests/transfer_session_roles.rs-1739-        &[("victim.txt", b"keep".to_vec(), 1_500_000_000)],
crates/blit-core/tests/transfer_session_roles.rs-1740-    );
crates/blit-core/tests/transfer_session_roles.rs-1741-
crates/blit-core/tests/transfer_session_roles.rs-1742-    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-1743-        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs-1744-        endpoint: SessionEndpoint::Responder,
crates/blit-core/tests/transfer_session_roles.rs-1745-        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs-1746-        instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-1747-        local_apply: None,
crates/blit-core/tests/transfer_session_roles.rs-1748-    };
crates/blit-core/tests/transfer_session_roles.rs-1749-    let (mut peer, dest_transport) = in_process_pair();
crates/blit-core/tests/transfer_session_roles.rs-1750-    let dest = tokio::spawn(run_destination(
--
crates/blit-core/tests/transfer_session_roles.rs-1762-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
crates/blit-core/tests/transfer_session_roles.rs-1763-
crates/blit-core/tests/transfer_session_roles.rs-1764-    // A manifest entry, then declare the scan INCOMPLETE.
crates/blit-core/tests/transfer_session_roles.rs-1765-    peer.send(wire(Frame::ManifestEntry(FileHeader {
crates/blit-core/tests/transfer_session_roles.rs-1766-        relative_path: "present.txt".into(),
crates/blit-core/tests/transfer_session_roles.rs-1767-        size: 1,
crates/blit-core/tests/transfer_session_roles.rs-1768-        mtime_seconds: 1_600_000_000,
crates/blit-core/tests/transfer_session_roles.rs-1769-        permissions: 0o644,
crates/blit-core/tests/transfer_session_roles.rs-1770-        checksum: vec![],
crates/blit-core/tests/transfer_session_roles.rs-1771-    })))
crates/blit-core/tests/transfer_session_roles.rs-1772-    .await
crates/blit-core/tests/transfer_session_roles.rs-1773-    .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1774-    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
crates/blit-core/tests/transfer_session_roles.rs-1775-        scan_complete: false,
crates/blit-core/tests/transfer_session_roles.rs-1776-    })))
crates/blit-core/tests/transfer_session_roles.rs-1777-    .await
crates/blit-core/tests/transfer_session_roles.rs-1778-    .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1779-
crates/blit-core/tests/transfer_session_roles.rs:1780:    let refusal = loop {
crates/blit-core/tests/transfer_session_roles.rs-1781-        match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-1782-            Frame::Error(e) => break e,
crates/blit-core/tests/transfer_session_roles.rs-1783-            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-1784-            other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-1785-        }
crates/blit-core/tests/transfer_session_roles.rs-1786-    };
crates/blit-core/tests/transfer_session_roles.rs:1787:    assert_eq!(refusal.code, session_error::Code::Internal as i32);
crates/blit-core/tests/transfer_session_roles.rs-1788-    assert!(
crates/blit-core/tests/transfer_session_roles.rs:1789:        refusal.message.contains("scan"),
crates/blit-core/tests/transfer_session_roles.rs:1790:        "refusal must cite the incomplete scan, got: {}",
crates/blit-core/tests/transfer_session_roles.rs:1791:        refusal.message
crates/blit-core/tests/transfer_session_roles.rs-1792-    );
crates/blit-core/tests/transfer_session_roles.rs-1793-    let dest_err = dest.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-1794-    assert_eq!(fault_of(&dest_err).code, session_error::Code::Internal);
crates/blit-core/tests/transfer_session_roles.rs-1795-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-1796-        dst_root.join("victim.txt").exists(),
crates/blit-core/tests/transfer_session_roles.rs:1797:        "nothing may be deleted on a refused mirror"
crates/blit-core/tests/transfer_session_roles.rs-1798-    );
crates/blit-core/tests/transfer_session_roles.rs-1799-}
crates/blit-core/tests/transfer_session_roles.rs-1800-
crates/blit-core/tests/transfer_session_roles.rs-1801-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs-1802-async fn cancel_frame_during_mirror_purge_aborts_the_deletions() {
crates/blit-core/tests/transfer_session_roles.rs-1803-    // codex otp-10b-2 F1: a peer fault (CancelJob on the serving
crates/blit-core/tests/transfer_session_roles.rs-1804-    // source) arriving while the DESTINATION runs its mirror delete
crates/blit-core/tests/transfer_session_roles.rs-1805-    // pass must abort the pass and surface the fault — not sit unread
crates/blit-core/tests/transfer_session_roles.rs-1806-    // on the control lane while deletions run to completion behind a
crates/blit-core/tests/transfer_session_roles.rs-1807-    // cancelled session. Scripted source peer: an EMPTY manifest makes
crates/blit-core/tests/transfer_session_roles.rs-1808-    // every destination file extraneous, and the CANCELLED frame is
crates/blit-core/tests/transfer_session_roles.rs-1809-    // queued right behind SourceDone — the purge race reads it (biased
crates/blit-core/tests/transfer_session_roles.rs-1810-    // frame-first) and flips the abort flag before the pass's next
crates/blit-core/tests/transfer_session_roles.rs-1811-    // filesystem op.
crates/blit-core/tests/transfer_session_roles.rs-1812-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1813-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-1814-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-1815-    for i in 0..2000 {
--
crates/blit-core/tests/transfer_session_roles.rs-1973-         about frame position; got: {dest_err:#}"
crates/blit-core/tests/transfer_session_roles.rs-1974-    );
crates/blit-core/tests/transfer_session_roles.rs-1975-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-1976-        fault.message.contains("cancelled by operator"),
crates/blit-core/tests/transfer_session_roles.rs-1977-        "the peer's message must survive the wire, got: {}",
crates/blit-core/tests/transfer_session_roles.rs-1978-        fault.message
crates/blit-core/tests/transfer_session_roles.rs-1979-    );
crates/blit-core/tests/transfer_session_roles.rs-1980-    // The 64 KiB partial must not be finalized as 'big.bin'.
crates/blit-core/tests/transfer_session_roles.rs-1981-    let final_path = dst_root.join("big.bin");
crates/blit-core/tests/transfer_session_roles.rs-1982-    if let Ok(meta) = std::fs::metadata(&final_path) {
crates/blit-core/tests/transfer_session_roles.rs-1983-        assert!(
crates/blit-core/tests/transfer_session_roles.rs-1984-            meta.len() < size as u64,
crates/blit-core/tests/transfer_session_roles.rs-1985-            "a cancelled record must never finalize at full size"
crates/blit-core/tests/transfer_session_roles.rs-1986-        );
crates/blit-core/tests/transfer_session_roles.rs-1987-    }
crates/blit-core/tests/transfer_session_roles.rs-1988-}
crates/blit-core/tests/transfer_session_roles.rs-1989-
crates/blit-core/tests/transfer_session_roles.rs-1990-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:1991:async fn incomplete_scan_refused_when_completeness_required() {
crates/blit-core/tests/transfer_session_roles.rs-1992-    // codex otp-9b F1 (R49-F2 on the session): an initiator that
crates/blit-core/tests/transfer_session_roles.rs-1993-    // declared require_complete_scan (`blit move` — the source is
crates/blit-core/tests/transfer_session_roles.rs-1994-    // deleted after success) must NOT get a success out of an
crates/blit-core/tests/transfer_session_roles.rs-1995-    // incomplete source scan; files the scan could not read would be
crates/blit-core/tests/transfer_session_roles.rs:1996:    // silently lost with the source. The destination refuses at
crates/blit-core/tests/transfer_session_roles.rs-1997-    // ManifestComplete{scan_complete=false} with SCAN_INCOMPLETE.
crates/blit-core/tests/transfer_session_roles.rs-1998-    // Scripted source peer so we control the flag.
crates/blit-core/tests/transfer_session_roles.rs-1999-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2000-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-2001-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2002-
crates/blit-core/tests/transfer_session_roles.rs-2003-    let dest_cfg = DestinationSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-2004-        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs-2005-        endpoint: SessionEndpoint::Responder,
crates/blit-core/tests/transfer_session_roles.rs-2006-        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs-2007-        instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-2008-        local_apply: None,
crates/blit-core/tests/transfer_session_roles.rs-2009-    };
crates/blit-core/tests/transfer_session_roles.rs-2010-    let (mut peer, dest_transport) = in_process_pair();
crates/blit-core/tests/transfer_session_roles.rs-2011-    let dest = tokio::spawn(run_destination(
crates/blit-core/tests/transfer_session_roles.rs-2012-        dest_cfg,
crates/blit-core/tests/transfer_session_roles.rs-2013-        dest_transport,
crates/blit-core/tests/transfer_session_roles.rs-2014-        DestinationTarget::Fixed(dst_root.clone()),
--
crates/blit-core/tests/transfer_session_roles.rs-2021-    peer.send(wire(Frame::Open(open))).await.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2022-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
crates/blit-core/tests/transfer_session_roles.rs-2023-
crates/blit-core/tests/transfer_session_roles.rs-2024-    peer.send(wire(Frame::ManifestEntry(FileHeader {
crates/blit-core/tests/transfer_session_roles.rs-2025-        relative_path: "present.txt".into(),
crates/blit-core/tests/transfer_session_roles.rs-2026-        size: 1,
crates/blit-core/tests/transfer_session_roles.rs-2027-        mtime_seconds: 1_600_000_000,
crates/blit-core/tests/transfer_session_roles.rs-2028-        permissions: 0o644,
crates/blit-core/tests/transfer_session_roles.rs-2029-        checksum: vec![],
crates/blit-core/tests/transfer_session_roles.rs-2030-    })))
crates/blit-core/tests/transfer_session_roles.rs-2031-    .await
crates/blit-core/tests/transfer_session_roles.rs-2032-    .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2033-    peer.send(wire(Frame::ManifestComplete(ManifestComplete {
crates/blit-core/tests/transfer_session_roles.rs-2034-        scan_complete: false,
crates/blit-core/tests/transfer_session_roles.rs-2035-    })))
crates/blit-core/tests/transfer_session_roles.rs-2036-    .await
crates/blit-core/tests/transfer_session_roles.rs-2037-    .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2038-
crates/blit-core/tests/transfer_session_roles.rs:2039:    // Bounded wait: an implementation that fails to refuse proceeds to
crates/blit-core/tests/transfer_session_roles.rs-2040-    // the payload phase and would otherwise hang this scripted peer.
crates/blit-core/tests/transfer_session_roles.rs:2041:    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs-2042-        loop {
crates/blit-core/tests/transfer_session_roles.rs-2043-            match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-2044-                Frame::Error(e) => break e,
crates/blit-core/tests/transfer_session_roles.rs-2045-                Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-2046-                other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-2047-            }
crates/blit-core/tests/transfer_session_roles.rs-2048-        }
crates/blit-core/tests/transfer_session_roles.rs-2049-    })
crates/blit-core/tests/transfer_session_roles.rs-2050-    .await
crates/blit-core/tests/transfer_session_roles.rs:2051:    .expect("destination must refuse the incomplete scan, not proceed");
crates/blit-core/tests/transfer_session_roles.rs:2052:    assert_eq!(refusal.code, session_error::Code::ScanIncomplete as i32);
crates/blit-core/tests/transfer_session_roles.rs-2053-    let dest_err = dest.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-2054-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-2055-        fault_of(&dest_err).code,
crates/blit-core/tests/transfer_session_roles.rs-2056-        session_error::Code::ScanIncomplete
crates/blit-core/tests/transfer_session_roles.rs-2057-    );
crates/blit-core/tests/transfer_session_roles.rs-2058-}
crates/blit-core/tests/transfer_session_roles.rs-2059-
crates/blit-core/tests/transfer_session_roles.rs-2060-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs-2061-async fn source_filter_limits_manifest_under_both_initiators() {
crates/blit-core/tests/transfer_session_roles.rs-2062-    // otp-6a: an include filter on the open restricts the source scan to
crates/blit-core/tests/transfer_session_roles.rs-2063-    // matching files; non-matching files are neither manifested nor
crates/blit-core/tests/transfer_session_roles.rs-2064-    // transferred, whichever end initiates. `*.txt` matches by basename,
crates/blit-core/tests/transfer_session_roles.rs-2065-    // so the nested keep2.txt is included and the .log / .bin are not.
crates/blit-core/tests/transfer_session_roles.rs-2066-    let src = vec![
crates/blit-core/tests/transfer_session_roles.rs-2067-        ("keep.txt", b"a".to_vec(), 1_600_000_001),
crates/blit-core/tests/transfer_session_roles.rs-2068-        ("drop.log", b"b".to_vec(), 1_600_000_002),
crates/blit-core/tests/transfer_session_roles.rs-2069-        ("dir/keep2.txt", b"c".to_vec(), 1_600_000_003),
crates/blit-core/tests/transfer_session_roles.rs-2070-        ("dir/skip.bin", b"d".to_vec(), 1_600_000_004),
--
crates/blit-core/tests/transfer_session_roles.rs-2282-        .await
crates/blit-core/tests/transfer_session_roles.rs-2283-        .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2284-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
crates/blit-core/tests/transfer_session_roles.rs-2285-
crates/blit-core/tests/transfer_session_roles.rs-2286-    let header = FileHeader {
crates/blit-core/tests/transfer_session_roles.rs-2287-        relative_path: "early.bin".into(),
crates/blit-core/tests/transfer_session_roles.rs-2288-        size: 4,
crates/blit-core/tests/transfer_session_roles.rs-2289-        mtime_seconds: 1_600_000_000,
crates/blit-core/tests/transfer_session_roles.rs-2290-        permissions: 0o644,
crates/blit-core/tests/transfer_session_roles.rs-2291-        checksum: vec![],
crates/blit-core/tests/transfer_session_roles.rs-2292-    };
crates/blit-core/tests/transfer_session_roles.rs-2293-    peer.send(wire(Frame::ManifestEntry(header.clone())))
crates/blit-core/tests/transfer_session_roles.rs-2294-        .await
crates/blit-core/tests/transfer_session_roles.rs-2295-        .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2296-    peer.send(wire(Frame::FileBegin(header))).await.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2297-
crates/blit-core/tests/transfer_session_roles.rs-2298-    // The destination must answer with a SessionError frame naming
crates/blit-core/tests/transfer_session_roles.rs-2299-    // the violation...
crates/blit-core/tests/transfer_session_roles.rs:2300:    let refusal = loop {
crates/blit-core/tests/transfer_session_roles.rs-2301-        match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-2302-            Frame::Error(e) => break e,
crates/blit-core/tests/transfer_session_roles.rs-2303-            // need batches may legitimately arrive first
crates/blit-core/tests/transfer_session_roles.rs-2304-            Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-2305-            other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-2306-        }
crates/blit-core/tests/transfer_session_roles.rs-2307-    };
crates/blit-core/tests/transfer_session_roles.rs:2308:    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
crates/blit-core/tests/transfer_session_roles.rs-2309-
crates/blit-core/tests/transfer_session_roles.rs-2310-    // ...and its driver must fail with the same fault.
crates/blit-core/tests/transfer_session_roles.rs-2311-    let dest_err = dest.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-2312-    assert_eq!(
crates/blit-core/tests/transfer_session_roles.rs-2313-        fault_of(&dest_err).code,
crates/blit-core/tests/transfer_session_roles.rs-2314-        session_error::Code::ProtocolViolation
crates/blit-core/tests/transfer_session_roles.rs-2315-    );
crates/blit-core/tests/transfer_session_roles.rs-2316-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-2317-        collect_tree(tmp.path()).is_empty(),
crates/blit-core/tests/transfer_session_roles.rs-2318-        "no bytes may land from a violating record"
crates/blit-core/tests/transfer_session_roles.rs-2319-    );
crates/blit-core/tests/transfer_session_roles.rs-2320-}
crates/blit-core/tests/transfer_session_roles.rs-2321-
crates/blit-core/tests/transfer_session_roles.rs-2322-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs-2323-async fn need_for_unknown_path_faults_the_source() {
crates/blit-core/tests/transfer_session_roles.rs-2324-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2325-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-2326-    std::fs::create_dir_all(&src_root).unwrap();
--
crates/blit-core/tests/transfer_session_roles.rs-2352-            other => panic!("expected manifest stream, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-2353-        }
crates/blit-core/tests/transfer_session_roles.rs-2354-    }
crates/blit-core/tests/transfer_session_roles.rs-2355-    peer.send(wire(Frame::NeedBatch(NeedBatch {
crates/blit-core/tests/transfer_session_roles.rs-2356-        entries: vec![NeedEntry {
crates/blit-core/tests/transfer_session_roles.rs-2357-            relative_path: "never-manifested.txt".into(),
crates/blit-core/tests/transfer_session_roles.rs-2358-            resume: false,
crates/blit-core/tests/transfer_session_roles.rs-2359-        }],
crates/blit-core/tests/transfer_session_roles.rs-2360-    })))
crates/blit-core/tests/transfer_session_roles.rs-2361-    .await
crates/blit-core/tests/transfer_session_roles.rs-2362-    .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2363-
crates/blit-core/tests/transfer_session_roles.rs-2364-    let source_err = source_task.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-2365-    let fault = fault_of(&source_err);
crates/blit-core/tests/transfer_session_roles.rs-2366-    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs-2367-    assert!(fault.message.contains("never-manifested.txt"));
crates/blit-core/tests/transfer_session_roles.rs-2368-
crates/blit-core/tests/transfer_session_roles.rs-2369-    // The source must have told the peer why before aborting.
crates/blit-core/tests/transfer_session_roles.rs:2370:    let refusal = match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-2371-        Frame::Error(e) => e,
crates/blit-core/tests/transfer_session_roles.rs-2372-        other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-2373-    };
crates/blit-core/tests/transfer_session_roles.rs:2374:    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
crates/blit-core/tests/transfer_session_roles.rs-2375-}
crates/blit-core/tests/transfer_session_roles.rs-2376-
crates/blit-core/tests/transfer_session_roles.rs-2377-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs:2378:async fn resume_flagged_need_is_refused_in_non_resume_session() {
crates/blit-core/tests/transfer_session_roles.rs-2379-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2380-    let src_root = tmp.path().join("src");
crates/blit-core/tests/transfer_session_roles.rs-2381-    std::fs::create_dir_all(&src_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2382-    write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
crates/blit-core/tests/transfer_session_roles.rs-2383-
crates/blit-core/tests/transfer_session_roles.rs-2384-    let source_cfg = SourceSessionConfig {
crates/blit-core/tests/transfer_session_roles.rs-2385-        instruments: Default::default(),
crates/blit-core/tests/transfer_session_roles.rs-2386-        hello: HelloConfig::default(),
crates/blit-core/tests/transfer_session_roles.rs-2387-        endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
crates/blit-core/tests/transfer_session_roles.rs-2388-        plan_options: PlanOptions::default(),
crates/blit-core/tests/transfer_session_roles.rs-2389-        data_plane_host: None,
crates/blit-core/tests/transfer_session_roles.rs-2390-    };
crates/blit-core/tests/transfer_session_roles.rs-2391-    let (source_transport, mut peer) = in_process_pair();
crates/blit-core/tests/transfer_session_roles.rs-2392-    let source = Arc::new(FsTransferSource::new(src_root));
crates/blit-core/tests/transfer_session_roles.rs-2393-    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
crates/blit-core/tests/transfer_session_roles.rs-2394-
crates/blit-core/tests/transfer_session_roles.rs-2395-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
crates/blit-core/tests/transfer_session_roles.rs-2396-    peer.send(hello_frame()).await.unwrap();
--
crates/blit-core/tests/transfer_session_roles.rs-2452-    let source = Arc::new(FsTransferSource::new(src_root));
crates/blit-core/tests/transfer_session_roles.rs-2453-    let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
crates/blit-core/tests/transfer_session_roles.rs-2454-
crates/blit-core/tests/transfer_session_roles.rs-2455-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
crates/blit-core/tests/transfer_session_roles.rs-2456-    peer.send(hello_frame()).await.unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2457-    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
crates/blit-core/tests/transfer_session_roles.rs-2458-    peer.send(wire(Frame::Accept(Default::default())))
crates/blit-core/tests/transfer_session_roles.rs-2459-        .await
crates/blit-core/tests/transfer_session_roles.rs-2460-        .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2461-    // The violation: promise need-completion before reading a single
crates/blit-core/tests/transfer_session_roles.rs-2462-    // manifest frame.
crates/blit-core/tests/transfer_session_roles.rs-2463-    peer.send(wire(Frame::NeedComplete(NeedComplete {})))
crates/blit-core/tests/transfer_session_roles.rs-2464-        .await
crates/blit-core/tests/transfer_session_roles.rs-2465-        .unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2466-
crates/blit-core/tests/transfer_session_roles.rs-2467-    // The source must abort with a SessionError before its manifest
crates/blit-core/tests/transfer_session_roles.rs-2468-    // completes — never treat the early promise as a clean empty
crates/blit-core/tests/transfer_session_roles.rs-2469-    // transfer.
crates/blit-core/tests/transfer_session_roles.rs:2470:    let refusal = loop {
crates/blit-core/tests/transfer_session_roles.rs-2471-        match recv_or_panic(&mut peer).await {
crates/blit-core/tests/transfer_session_roles.rs-2472-            Frame::ManifestEntry(_) => continue,
crates/blit-core/tests/transfer_session_roles.rs-2473-            Frame::Error(e) => break e,
crates/blit-core/tests/transfer_session_roles.rs-2474-            Frame::ManifestComplete(_) => {
crates/blit-core/tests/transfer_session_roles.rs-2475-                panic!("source completed its manifest instead of failing fast")
crates/blit-core/tests/transfer_session_roles.rs-2476-            }
crates/blit-core/tests/transfer_session_roles.rs-2477-            Frame::SourceDone(_) => panic!("source treated early NeedComplete as legitimate"),
crates/blit-core/tests/transfer_session_roles.rs-2478-            other => panic!("expected SessionError, got {other:?}"),
crates/blit-core/tests/transfer_session_roles.rs-2479-        }
crates/blit-core/tests/transfer_session_roles.rs-2480-    };
crates/blit-core/tests/transfer_session_roles.rs:2481:    assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
crates/blit-core/tests/transfer_session_roles.rs-2482-
crates/blit-core/tests/transfer_session_roles.rs-2483-    let source_err = source_task.await.unwrap().unwrap_err();
crates/blit-core/tests/transfer_session_roles.rs-2484-    let fault = fault_of(&source_err);
crates/blit-core/tests/transfer_session_roles.rs-2485-    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
crates/blit-core/tests/transfer_session_roles.rs-2486-    assert!(
crates/blit-core/tests/transfer_session_roles.rs-2487-        fault.message.contains("ManifestComplete"),
crates/blit-core/tests/transfer_session_roles.rs-2488-        "fault must name the ordering rule, got: {}",
crates/blit-core/tests/transfer_session_roles.rs-2489-        fault.message
crates/blit-core/tests/transfer_session_roles.rs-2490-    );
crates/blit-core/tests/transfer_session_roles.rs-2491-}
crates/blit-core/tests/transfer_session_roles.rs-2492-
crates/blit-core/tests/transfer_session_roles.rs-2493-#[tokio::test]
crates/blit-core/tests/transfer_session_roles.rs-2494-async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
crates/blit-core/tests/transfer_session_roles.rs-2495-    let tmp = tempfile::tempdir().unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2496-    let dst_root = tmp.path().join("dst");
crates/blit-core/tests/transfer_session_roles.rs-2497-    std::fs::create_dir_all(&dst_root).unwrap();
crates/blit-core/tests/transfer_session_roles.rs-2498-
crates/blit-core/tests/transfer_session_roles.rs-2499-    let dest_cfg = DestinationSessionConfig {
--
crates/blit-core/src/transfer_session/mod.rs-23-};
crates/blit-core/src/transfer_session/mod.rs-24-
crates/blit-core/src/transfer_session/mod.rs-25-use std::collections::{HashMap, HashSet};
crates/blit-core/src/transfer_session/mod.rs-26-use std::fmt;
crates/blit-core/src/transfer_session/mod.rs-27-use std::future::Future;
crates/blit-core/src/transfer_session/mod.rs-28-use std::path::{Path, PathBuf};
crates/blit-core/src/transfer_session/mod.rs-29-use std::pin::Pin;
crates/blit-core/src/transfer_session/mod.rs-30-use std::sync::atomic::{AtomicBool, Ordering};
crates/blit-core/src/transfer_session/mod.rs-31-use std::sync::{Arc, Mutex as StdMutex};
crates/blit-core/src/transfer_session/mod.rs-32-
crates/blit-core/src/transfer_session/mod.rs-33-use eyre::Result;
crates/blit-core/src/transfer_session/mod.rs-34-use tokio::io::{AsyncReadExt, AsyncWriteExt};
crates/blit-core/src/transfer_session/mod.rs-35-use tokio::sync::{mpsc, watch};
crates/blit-core/src/transfer_session/mod.rs-36-
crates/blit-core/src/transfer_session/mod.rs-37-use crate::copy::DEFAULT_BLOCK_SIZE;
crates/blit-core/src/transfer_session/mod.rs-38-use crate::generated::transfer_frame::Frame;
crates/blit-core/src/transfer_session/mod.rs-39-use crate::generated::{
crates/blit-core/src/transfer_session/mod.rs-40-    session_error, BlockHashList, BlockTransfer, BlockTransferComplete, ComparisonMode,
crates/blit-core/src/transfer_session/mod.rs:41:    DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, FileData, FileHeader, FilterSpec,
crates/blit-core/src/transfer_session/mod.rs-42-    ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept, SessionError,
crates/blit-core/src/transfer_session/mod.rs-43-    SessionHello, SessionOpen, SourceDone, TarShardComplete, TarShardHeader, TransferFrame,
crates/blit-core/src/transfer_session/mod.rs-44-    TransferRole, TransferSummary,
crates/blit-core/src/transfer_session/mod.rs-45-};
crates/blit-core/src/transfer_session/mod.rs-46-use crate::manifest::{header_transfer_status, CompareMode, CompareOptions, FileStatus};
crates/blit-core/src/transfer_session/mod.rs-47-use crate::remote::transfer::diff_planner;
crates/blit-core/src/transfer_session/mod.rs-48-use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
crates/blit-core/src/transfer_session/mod.rs-49-use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
crates/blit-core/src/transfer_session/mod.rs-50-use crate::remote::transfer::source::{FsTransferSource, TransferSource};
crates/blit-core/src/transfer_session/mod.rs-51-use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
crates/blit-core/src/transfer_session/mod.rs-52-use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/transfer_session/mod.rs-53-use crate::remote::transfer::{
crates/blit-core/src/transfer_session/mod.rs-54-    AbortOnDrop, FaultedPath, RemoteTransferProgress, CONTROL_PLANE_CHUNK_SIZE,
crates/blit-core/src/transfer_session/mod.rs-55-};
crates/blit-core/src/transfer_session/mod.rs-56-use crate::transfer_plan::PlanOptions;
crates/blit-core/src/transfer_session/mod.rs-57-use transport::{FrameRx, FrameTransport, FrameTx};
crates/blit-core/src/transfer_session/mod.rs-58-
crates/blit-core/src/transfer_session/mod.rs-59-/// Belt-and-braces wire-shape version, bumped on any change to the
--
crates/blit-core/src/transfer_session/mod.rs-252-    /// `ManifestBatch` per NeedBatch emitted (the pull-direction
crates/blit-core/src/transfer_session/mod.rs-253-    /// denominator — files this DESTINATION requested, the same
crates/blit-core/src/transfer_session/mod.rs-254-    /// files-to-transfer semantic the push verb reports),
crates/blit-core/src/transfer_session/mod.rs-255-    /// `Payload`/`FileComplete` per record received on either carrier.
crates/blit-core/src/transfer_session/mod.rs-256-    pub progress: Option<RemoteTransferProgress>,
crates/blit-core/src/transfer_session/mod.rs-257-    /// Live byte counter for this DESTINATION's writes (otp-9a). The
crates/blit-core/src/transfer_session/mod.rs-258-    /// session sink reports applied payload bytes against it — the same
crates/blit-core/src/transfer_session/mod.rs-259-    /// `ByteProgressSink` contract the old drivers used, so a caller
crates/blit-core/src/transfer_session/mod.rs-260-    /// that owns a jobs row (the delegated dst daemon, otp-9) can watch
crates/blit-core/src/transfer_session/mod.rs-261-    /// bytes land while the session runs. `None` = no reporting.
crates/blit-core/src/transfer_session/mod.rs-262-    pub byte_progress: Option<crate::remote::transfer::ByteProgressSink>,
crates/blit-core/src/transfer_session/mod.rs-263-    /// Emit `[data-plane-client]` connect traces on the data-plane
crates/blit-core/src/transfer_session/mod.rs-264-    /// sockets this DESTINATION initiator dials (`--trace-data-plane`).
crates/blit-core/src/transfer_session/mod.rs-265-    /// A DESTINATION responder accepts rather than dials; the flag is
crates/blit-core/src/transfer_session/mod.rs-266-    /// inert there.
crates/blit-core/src/transfer_session/mod.rs-267-    pub trace_data_plane: bool,
crates/blit-core/src/transfer_session/mod.rs-268-}
crates/blit-core/src/transfer_session/mod.rs-269-
crates/blit-core/src/transfer_session/mod.rs:270:/// A session-terminating fault: either end refusing, aborting, or
crates/blit-core/src/transfer_session/mod.rs-271-/// catching the peer in a protocol violation. Carried as the error
crates/blit-core/src/transfer_session/mod.rs-272-/// payload of the drivers' `eyre::Report`s — downcast to inspect the
crates/blit-core/src/transfer_session/mod.rs-273-/// wire code.
crates/blit-core/src/transfer_session/mod.rs-274-#[derive(Debug, Clone)]
crates/blit-core/src/transfer_session/mod.rs-275-pub struct SessionFault {
crates/blit-core/src/transfer_session/mod.rs-276-    pub code: session_error::Code,
crates/blit-core/src/transfer_session/mod.rs-277-    pub message: String,
crates/blit-core/src/transfer_session/mod.rs-278-    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
crates/blit-core/src/transfer_session/mod.rs-279-    /// which end is stale (contract §Errors).
crates/blit-core/src/transfer_session/mod.rs-280-    pub local_build_id: String,
crates/blit-core/src/transfer_session/mod.rs-281-    pub peer_build_id: String,
crates/blit-core/src/transfer_session/mod.rs-282-    /// True when the peer already knows about this fault — it sent
crates/blit-core/src/transfer_session/mod.rs-283-    /// the `SessionError` frame itself, or this end already emitted
crates/blit-core/src/transfer_session/mod.rs-284-    /// one. Drivers must not send another.
crates/blit-core/src/transfer_session/mod.rs-285-    pub peer_notified: bool,
crates/blit-core/src/transfer_session/mod.rs-286-    /// otp-7b-2 (D-2026-07-09-1 Q2 rider): the file this fault
crates/blit-core/src/transfer_session/mod.rs-287-    /// concerns, when one is known — a mid-record read/write failure
crates/blit-core/src/transfer_session/mod.rs-288-    /// names its file so the end-of-operation summary can, too.
--
crates/blit-core/src/transfer_session/mod.rs-353-             destination is preserved; re-run the same command to converge \
crates/blit-core/src/transfer_session/mod.rs-354-             (resume transfers only what is still missing)",
crates/blit-core/src/transfer_session/mod.rs-355-            self.message
crates/blit-core/src/transfer_session/mod.rs-356-        ))
crates/blit-core/src/transfer_session/mod.rs-357-    }
crates/blit-core/src/transfer_session/mod.rs-358-
crates/blit-core/src/transfer_session/mod.rs-359-    fn protocol_violation(message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-360-        Self::new(session_error::Code::ProtocolViolation, message)
crates/blit-core/src/transfer_session/mod.rs-361-    }
crates/blit-core/src/transfer_session/mod.rs-362-
crates/blit-core/src/transfer_session/mod.rs-363-    fn internal(message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-364-        Self::new(session_error::Code::Internal, message)
crates/blit-core/src/transfer_session/mod.rs-365-    }
crates/blit-core/src/transfer_session/mod.rs-366-
crates/blit-core/src/transfer_session/mod.rs-367-    fn read_only(message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-368-        Self::new(session_error::Code::ReadOnly, message)
crates/blit-core/src/transfer_session/mod.rs-369-    }
crates/blit-core/src/transfer_session/mod.rs-370-
crates/blit-core/src/transfer_session/mod.rs:371:    /// Public constructor for a caller-side refusal (e.g. the daemon's
crates/blit-core/src/transfer_session/mod.rs-372-    /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
crates/blit-core/src/transfer_session/mod.rs-373-    /// code). blit-core stays free of `tonic::Status`, so the caller
crates/blit-core/src/transfer_session/mod.rs-374-    /// picks the wire code.
crates/blit-core/src/transfer_session/mod.rs:375:    pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
crates/blit-core/src/transfer_session/mod.rs-376-        Self::new(code, message)
crates/blit-core/src/transfer_session/mod.rs-377-    }
crates/blit-core/src/transfer_session/mod.rs-378-
crates/blit-core/src/transfer_session/mod.rs-379-    fn from_wire(err: SessionError) -> Self {
crates/blit-core/src/transfer_session/mod.rs-380-        Self {
crates/blit-core/src/transfer_session/mod.rs-381-            code: session_error::Code::try_from(err.code)
crates/blit-core/src/transfer_session/mod.rs-382-                .unwrap_or(session_error::Code::SessionErrorUnspecified),
crates/blit-core/src/transfer_session/mod.rs-383-            message: err.message,
crates/blit-core/src/transfer_session/mod.rs-384-            // The peer reports its view: its "local" is our peer.
crates/blit-core/src/transfer_session/mod.rs-385-            local_build_id: err.peer_build_id,
crates/blit-core/src/transfer_session/mod.rs-386-            peer_build_id: err.local_build_id,
crates/blit-core/src/transfer_session/mod.rs-387-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-388-            // Explicit wire presence (codex 7b-2 G1): "" is the valid
crates/blit-core/src/transfer_session/mod.rs-389-            // identity of a single-file-root transfer, not absence.
crates/blit-core/src/transfer_session/mod.rs-390-            relative_path: err.relative_path,
crates/blit-core/src/transfer_session/mod.rs-391-            // Peer-reported fault: no local I/O evidence (codex
crates/blit-core/src/transfer_session/mod.rs-392-            // otp-10a F5 — io_kind is local-transport testimony only).
crates/blit-core/src/transfer_session/mod.rs-393-            io_kind: None,
--
crates/blit-core/src/transfer_session/mod.rs-446-fn frame_name(f: &Option<Frame>) -> &'static str {
crates/blit-core/src/transfer_session/mod.rs-447-    match f {
crates/blit-core/src/transfer_session/mod.rs-448-        Some(Frame::Hello(_)) => "SessionHello",
crates/blit-core/src/transfer_session/mod.rs-449-        Some(Frame::Open(_)) => "SessionOpen",
crates/blit-core/src/transfer_session/mod.rs-450-        Some(Frame::Accept(_)) => "SessionAccept",
crates/blit-core/src/transfer_session/mod.rs-451-        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
crates/blit-core/src/transfer_session/mod.rs-452-        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs-453-        Some(Frame::NeedBatch(_)) => "NeedBatch",
crates/blit-core/src/transfer_session/mod.rs-454-        Some(Frame::NeedComplete(_)) => "NeedComplete",
crates/blit-core/src/transfer_session/mod.rs-455-        Some(Frame::BlockHashes(_)) => "BlockHashList",
crates/blit-core/src/transfer_session/mod.rs-456-        Some(Frame::FileBegin(_)) => "FileBegin",
crates/blit-core/src/transfer_session/mod.rs-457-        Some(Frame::FileData(_)) => "FileData",
crates/blit-core/src/transfer_session/mod.rs-458-        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
crates/blit-core/src/transfer_session/mod.rs-459-        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
crates/blit-core/src/transfer_session/mod.rs-460-        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
crates/blit-core/src/transfer_session/mod.rs-461-        Some(Frame::Block(_)) => "BlockTransfer",
crates/blit-core/src/transfer_session/mod.rs-462-        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
crates/blit-core/src/transfer_session/mod.rs-463-        Some(Frame::Resize(_)) => "DataPlaneResize",
crates/blit-core/src/transfer_session/mod.rs:464:        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
crates/blit-core/src/transfer_session/mod.rs-465-        Some(Frame::SourceDone(_)) => "SourceDone",
crates/blit-core/src/transfer_session/mod.rs-466-        Some(Frame::Summary(_)) => "TransferSummary",
crates/blit-core/src/transfer_session/mod.rs-467-        Some(Frame::Error(_)) => "SessionError",
crates/blit-core/src/transfer_session/mod.rs-468-        None => "empty frame",
crates/blit-core/src/transfer_session/mod.rs-469-    }
crates/blit-core/src/transfer_session/mod.rs-470-}
crates/blit-core/src/transfer_session/mod.rs-471-
crates/blit-core/src/transfer_session/mod.rs-472-fn complement(role: TransferRole) -> TransferRole {
crates/blit-core/src/transfer_session/mod.rs-473-    match role {
crates/blit-core/src/transfer_session/mod.rs-474-        TransferRole::Source => TransferRole::Destination,
crates/blit-core/src/transfer_session/mod.rs-475-        TransferRole::Destination => TransferRole::Source,
crates/blit-core/src/transfer_session/mod.rs-476-        TransferRole::Unspecified => TransferRole::Unspecified,
crates/blit-core/src/transfer_session/mod.rs-477-    }
crates/blit-core/src/transfer_session/mod.rs-478-}
crates/blit-core/src/transfer_session/mod.rs-479-
crates/blit-core/src/transfer_session/mod.rs-480-/// Build a `SessionError` frame with the given code and message — the
crates/blit-core/src/transfer_session/mod.rs-481-/// wire form an end sends to tell its peer why it is aborting. Public
crates/blit-core/src/transfer_session/mod.rs-482-/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
crates/blit-core/src/transfer_session/mod.rs-483-/// fires mid-session (the session future is aborted by the select and
crates/blit-core/src/transfer_session/mod.rs-484-/// cannot send it itself — otp-4a codex F1); blit-core stays the one
crates/blit-core/src/transfer_session/mod.rs-485-/// owner of the frame grammar. The build-id fields are left empty:
crates/blit-core/src/transfer_session/mod.rs-486-/// they are only meaningful for `BUILD_MISMATCH`.
crates/blit-core/src/transfer_session/mod.rs-487-pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
crates/blit-core/src/transfer_session/mod.rs-488-    frame(Frame::Error(SessionError {
crates/blit-core/src/transfer_session/mod.rs-489-        code: code as i32,
crates/blit-core/src/transfer_session/mod.rs-490-        message: message.into(),
crates/blit-core/src/transfer_session/mod.rs-491-        local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-492-        peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-493-        relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-494-    }))
crates/blit-core/src/transfer_session/mod.rs-495-}
crates/blit-core/src/transfer_session/mod.rs-496-
crates/blit-core/src/transfer_session/mod.rs-497-/// Per-role capability check of the operation a `SessionOpen`
crates/blit-core/src/transfer_session/mod.rs:498:/// describes. otp-3 refuses what later slices implement rather than
crates/blit-core/src/transfer_session/mod.rs-499-/// silently ignoring it (fail-fast; contract §Errors).
crates/blit-core/src/transfer_session/mod.rs-500-type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
crates/blit-core/src/transfer_session/mod.rs-501-
crates/blit-core/src/transfer_session/mod.rs-502-/// The local endpoint a Responder resolves a received `SessionOpen`
crates/blit-core/src/transfer_session/mod.rs-503-/// to. The daemon maps the wire module name + path here; a test can
crates/blit-core/src/transfer_session/mod.rs-504-/// hand a fixed root with no module semantics via
crates/blit-core/src/transfer_session/mod.rs-505-/// [`DestinationTarget::Fixed`] instead.
crates/blit-core/src/transfer_session/mod.rs-506-#[derive(Debug, Clone)]
crates/blit-core/src/transfer_session/mod.rs-507-pub struct ResolvedEndpoint {
crates/blit-core/src/transfer_session/mod.rs-508-    /// Absolute local root this end targets.
crates/blit-core/src/transfer_session/mod.rs-509-    pub root: PathBuf,
crates/blit-core/src/transfer_session/mod.rs-510-    /// Whether the resolved module forbids writes. A DESTINATION
crates/blit-core/src/transfer_session/mod.rs:511:    /// responder refuses `READ_ONLY`; a SOURCE responder (otp-5,
crates/blit-core/src/transfer_session/mod.rs-512-    /// daemon-send) does not care — reading a read-only module is fine.
crates/blit-core/src/transfer_session/mod.rs-513-    pub read_only: bool,
crates/blit-core/src/transfer_session/mod.rs-514-}
crates/blit-core/src/transfer_session/mod.rs-515-
crates/blit-core/src/transfer_session/mod.rs-516-/// Async callback a Responder uses to turn a received (and
crates/blit-core/src/transfer_session/mod.rs-517-/// capability-validated) `SessionOpen` into its local endpoint. It
crates/blit-core/src/transfer_session/mod.rs-518-/// lives caller-side — the daemon resolves modules and maps its own
crates/blit-core/src/transfer_session/mod.rs-519-/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
crates/blit-core/src/transfer_session/mod.rs-520-/// of module/Status types. A returned fault (unknown module,
crates/blit-core/src/transfer_session/mod.rs-521-/// containment failure) becomes a `SessionError` at OPEN, never a
crates/blit-core/src/transfer_session/mod.rs-522-/// silent close (contract §Phase state machine).
crates/blit-core/src/transfer_session/mod.rs-523-pub type OpenResolver = dyn Fn(
crates/blit-core/src/transfer_session/mod.rs-524-        &SessionOpen,
crates/blit-core/src/transfer_session/mod.rs-525-    )
crates/blit-core/src/transfer_session/mod.rs-526-        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
crates/blit-core/src/transfer_session/mod.rs-527-    + Send
crates/blit-core/src/transfer_session/mod.rs-528-    + Sync;
crates/blit-core/src/transfer_session/mod.rs-529-
--
crates/blit-core/src/transfer_session/mod.rs-559-/// so the caller (the daemon) learns after the fact which half ran.
crates/blit-core/src/transfer_session/mod.rs-560-pub enum ResponderOutcome {
crates/blit-core/src/transfer_session/mod.rs-561-    /// The initiator was SOURCE; this end received (push-equivalent).
crates/blit-core/src/transfer_session/mod.rs-562-    Destination(DestinationOutcome),
crates/blit-core/src/transfer_session/mod.rs-563-    /// The initiator was DESTINATION; this end sent (pull-equivalent).
crates/blit-core/src/transfer_session/mod.rs-564-    Source(TransferSummary),
crates/blit-core/src/transfer_session/mod.rs-565-}
crates/blit-core/src/transfer_session/mod.rs-566-
crates/blit-core/src/transfer_session/mod.rs-567-/// otp-7a: whether this open negotiates the resume block phase. One
crates/blit-core/src/transfer_session/mod.rs-568-/// reading, both roles and both validators — the flag is in the open, so
crates/blit-core/src/transfer_session/mod.rs-569-/// resume runs identically whichever end initiated (plan D6).
crates/blit-core/src/transfer_session/mod.rs-570-fn resume_negotiated(open: &SessionOpen) -> bool {
crates/blit-core/src/transfer_session/mod.rs-571-    open.resume.as_ref().is_some_and(|r| r.enabled)
crates/blit-core/src/transfer_session/mod.rs-572-}
crates/blit-core/src/transfer_session/mod.rs-573-
crates/blit-core/src/transfer_session/mod.rs-574-fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
crates/blit-core/src/transfer_session/mod.rs-575-    // otp-6a: filters are honored on the source scan (see
crates/blit-core/src/transfer_session/mod.rs-576-    // `source_send_half`). Validate the globs here so a malformed pattern
crates/blit-core/src/transfer_session/mod.rs:577:    // from a peer is refused at OPEN — peer-notified on the responder —
crates/blit-core/src/transfer_session/mod.rs-578-    // rather than faulting mid-scan once bytes are already moving.
crates/blit-core/src/transfer_session/mod.rs-579-    if let Some(filter) = open.filter.as_ref() {
crates/blit-core/src/transfer_session/mod.rs-580-        if *filter != FilterSpec::default() {
crates/blit-core/src/transfer_session/mod.rs-581-            crate::remote::transfer::operation_spec::filter_from_spec(filter.clone())
crates/blit-core/src/transfer_session/mod.rs-582-                .map_err(|e| SessionFault::protocol_violation(format!("invalid filter: {e:#}")))?;
crates/blit-core/src/transfer_session/mod.rs-583-        }
crates/blit-core/src/transfer_session/mod.rs-584-    }
crates/blit-core/src/transfer_session/mod.rs-585-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-586-}
crates/blit-core/src/transfer_session/mod.rs-587-
crates/blit-core/src/transfer_session/mod.rs-588-fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
crates/blit-core/src/transfer_session/mod.rs-589-    // otp-6b: mirror is executed on the DESTINATION (the end that owns the
crates/blit-core/src/transfer_session/mod.rs-590-    // dest tree). An enabled mirror needs a concrete scope; reject the
crates/blit-core/src/transfer_session/mod.rs-591-    // contradictory "enabled but OFF/unspecified kind" combination here.
crates/blit-core/src/transfer_session/mod.rs-592-    if open.mirror_enabled {
crates/blit-core/src/transfer_session/mod.rs-593-        let kind = MirrorMode::try_from(open.mirror_kind).unwrap_or(MirrorMode::Unspecified);
crates/blit-core/src/transfer_session/mod.rs-594-        if !matches!(kind, MirrorMode::FilteredSubset | MirrorMode::All) {
crates/blit-core/src/transfer_session/mod.rs-595-            return Err(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-596-                "mirror_enabled requires mirror_kind FILTERED_SUBSET or ALL",
crates/blit-core/src/transfer_session/mod.rs-597-            ));
crates/blit-core/src/transfer_session/mod.rs-598-        }
crates/blit-core/src/transfer_session/mod.rs-599-    }
crates/blit-core/src/transfer_session/mod.rs-600-    // The dest enumerates its tree through this filter when scoping a
crates/blit-core/src/transfer_session/mod.rs-601-    // FilteredSubset mirror, so its globs must be valid — validate at OPEN
crates/blit-core/src/transfer_session/mod.rs:602:    // (peer-notified refusal), symmetric with `source_open_validator`.
crates/blit-core/src/transfer_session/mod.rs-603-    if let Some(filter) = open.filter.as_ref() {
crates/blit-core/src/transfer_session/mod.rs-604-        if *filter != FilterSpec::default() {
crates/blit-core/src/transfer_session/mod.rs-605-            crate::remote::transfer::operation_spec::filter_from_spec(filter.clone())
crates/blit-core/src/transfer_session/mod.rs-606-                .map_err(|e| SessionFault::protocol_violation(format!("invalid filter: {e:#}")))?;
crates/blit-core/src/transfer_session/mod.rs-607-        }
crates/blit-core/src/transfer_session/mod.rs-608-    }
crates/blit-core/src/transfer_session/mod.rs-609-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-610-}
crates/blit-core/src/transfer_session/mod.rs-611-
crates/blit-core/src/transfer_session/mod.rs-612-/// Flips an abort flag when dropped, so a blocking-pool pass whose
crates/blit-core/src/transfer_session/mod.rs-613-/// awaiting future is dropped (client disconnect, CancelJob) stops at
crates/blit-core/src/transfer_session/mod.rs-614-/// its next flag check instead of running to completion behind a dead
crates/blit-core/src/transfer_session/mod.rs-615-/// session. Introduced for the mirror delete pass (codex otp-9b F2);
crates/blit-core/src/transfer_session/mod.rs-616-/// the destination diff's hash chunks share it (codex otp-10b-1 F3).
crates/blit-core/src/transfer_session/mod.rs-617-struct AbortFlagOnDrop(Arc<AtomicBool>);
crates/blit-core/src/transfer_session/mod.rs-618-impl Drop for AbortFlagOnDrop {
crates/blit-core/src/transfer_session/mod.rs-619-    fn drop(&mut self) {
crates/blit-core/src/transfer_session/mod.rs-620-        self.0.store(true, Ordering::Release);
crates/blit-core/src/transfer_session/mod.rs-621-    }
crates/blit-core/src/transfer_session/mod.rs-622-}
crates/blit-core/src/transfer_session/mod.rs-623-
crates/blit-core/src/transfer_session/mod.rs-624-/// Operator policy a serving responder applies to every session it
crates/blit-core/src/transfer_session/mod.rs-625-/// accepts (otp-10a F3 / otp-10b-1). Defaults are the permissive
crates/blit-core/src/transfer_session/mod.rs-626-/// non-daemon posture; the daemon fills it from its runtime config.
crates/blit-core/src/transfer_session/mod.rs-627-#[derive(Clone, Copy, Default)]
crates/blit-core/src/transfer_session/mod.rs-628-pub struct ResponderPolicy {
crates/blit-core/src/transfer_session/mod.rs-629-    /// `--force-grpc-data`: never grant a TCP data plane — every
crates/blit-core/src/transfer_session/mod.rs-630-    /// served session rides the in-stream carrier regardless of what
crates/blit-core/src/transfer_session/mod.rs-631-    /// the initiator asked for.
crates/blit-core/src/transfer_session/mod.rs-632-    pub force_in_stream: bool,
crates/blit-core/src/transfer_session/mod.rs:633:    /// `--no-server-checksums`: refuse `COMPARISON_MODE_CHECKSUM`
crates/blit-core/src/transfer_session/mod.rs-634-    /// opens with `CHECKSUM_DISABLED` instead of hashing (or silently
crates/blit-core/src/transfer_session/mod.rs-635-    /// degrading the compare).
crates/blit-core/src/transfer_session/mod.rs:636:    pub refuse_checksum_compare: bool,
crates/blit-core/src/transfer_session/mod.rs-637-}
crates/blit-core/src/transfer_session/mod.rs-638-
crates/blit-core/src/transfer_session/mod.rs-639-/// Outcome of the HELLO + OPEN phases.
crates/blit-core/src/transfer_session/mod.rs-640-struct Negotiated {
crates/blit-core/src/transfer_session/mod.rs-641-    open: SessionOpen,
crates/blit-core/src/transfer_session/mod.rs-642-    /// The responder's reply. The SOURCE initiator reads
crates/blit-core/src/transfer_session/mod.rs-643-    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
crates/blit-core/src/transfer_session/mod.rs-644-    accept: SessionAccept,
crates/blit-core/src/transfer_session/mod.rs-645-    /// The write root a Responder's [`OpenResolver`] produced from the
crates/blit-core/src/transfer_session/mod.rs-646-    /// received open, if one was supplied; `None` for an Initiator or a
crates/blit-core/src/transfer_session/mod.rs-647-    /// fixed-root Responder (the caller supplies the root then).
crates/blit-core/src/transfer_session/mod.rs-648-    resolved_root: Option<PathBuf>,
crates/blit-core/src/transfer_session/mod.rs-649-    /// The bound data-plane listener + credentials a DESTINATION
crates/blit-core/src/transfer_session/mod.rs-650-    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
crates/blit-core/src/transfer_session/mod.rs-651-    /// on an Initiator, or when the responder granted no data plane
crates/blit-core/src/transfer_session/mod.rs-652-    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
crates/blit-core/src/transfer_session/mod.rs-653-    responder_data_plane: Option<data_plane::ResponderDataPlane>,
crates/blit-core/src/transfer_session/mod.rs-654-}
--
crates/blit-core/src/transfer_session/mod.rs-690-                peer_hello.build_id, peer_hello.contract_version,
crates/blit-core/src/transfer_session/mod.rs-691-            ),
crates/blit-core/src/transfer_session/mod.rs-692-            local_build_id: hello.build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs-693-            peer_build_id: peer_hello.build_id.clone(),
crates/blit-core/src/transfer_session/mod.rs-694-            peer_notified: false,
crates/blit-core/src/transfer_session/mod.rs-695-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-696-            io_kind: None,
crates/blit-core/src/transfer_session/mod.rs-697-        };
crates/blit-core/src/transfer_session/mod.rs-698-        return Err(notify_and_wrap(transport, fault).await);
crates/blit-core/src/transfer_session/mod.rs-699-    }
crates/blit-core/src/transfer_session/mod.rs-700-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-701-}
crates/blit-core/src/transfer_session/mod.rs-702-
crates/blit-core/src/transfer_session/mod.rs-703-/// The responder half of establish AFTER the `SessionOpen` is read:
crates/blit-core/src/transfer_session/mod.rs-704-/// complement check, `validate_open`, endpoint resolution, data-plane
crates/blit-core/src/transfer_session/mod.rs-705-/// prepare, and `SessionAccept`. Factored out so both `establish` (which
crates/blit-core/src/transfer_session/mod.rs-706-/// reads the open then calls this) and `run_responder` (which reads the
crates/blit-core/src/transfer_session/mod.rs-707-/// open, dispatches on the declared role, then calls this with the
crates/blit-core/src/transfer_session/mod.rs:708:/// resolved local role) share one implementation. Sends the refusal
crates/blit-core/src/transfer_session/mod.rs-709-/// `SessionError` itself; returned faults are `peer_notified`.
crates/blit-core/src/transfer_session/mod.rs-710-async fn responder_finish(
crates/blit-core/src/transfer_session/mod.rs-711-    transport: &mut FrameTransport,
crates/blit-core/src/transfer_session/mod.rs-712-    open: SessionOpen,
crates/blit-core/src/transfer_session/mod.rs-713-    local_role: TransferRole,
crates/blit-core/src/transfer_session/mod.rs-714-    validate_open: &OpenValidator,
crates/blit-core/src/transfer_session/mod.rs-715-    resolve_open: Option<&OpenResolver>,
crates/blit-core/src/transfer_session/mod.rs-716-    policy: &ResponderPolicy,
crates/blit-core/src/transfer_session/mod.rs-717-) -> Result<Negotiated> {
crates/blit-core/src/transfer_session/mod.rs-718-    // The initiator declares ITS role; this responder end must
crates/blit-core/src/transfer_session/mod.rs-719-    // hold the complement.
crates/blit-core/src/transfer_session/mod.rs-720-    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
crates/blit-core/src/transfer_session/mod.rs-721-    if declared != complement(local_role) {
crates/blit-core/src/transfer_session/mod.rs-722-        return Err(notify_and_wrap(
crates/blit-core/src/transfer_session/mod.rs-723-            transport,
crates/blit-core/src/transfer_session/mod.rs-724-            SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs-725-                "initiator declared role {} but this responder is {}",
crates/blit-core/src/transfer_session/mod.rs-726-                declared.as_str_name(),
crates/blit-core/src/transfer_session/mod.rs-727-                local_role.as_str_name()
crates/blit-core/src/transfer_session/mod.rs-728-            )),
crates/blit-core/src/transfer_session/mod.rs-729-        )
crates/blit-core/src/transfer_session/mod.rs-730-        .await);
crates/blit-core/src/transfer_session/mod.rs-731-    }
crates/blit-core/src/transfer_session/mod.rs-732-    // otp-10b-1: an operator who disabled server-side checksum hashing
crates/blit-core/src/transfer_session/mod.rs:733:    // refuses a content-compare session outright — the session never
crates/blit-core/src/transfer_session/mod.rs-734-    // silently degrades a `--checksum` request to a weaker compare.
crates/blit-core/src/transfer_session/mod.rs:735:    if policy.refuse_checksum_compare && open.compare_mode == ComparisonMode::Checksum as i32 {
crates/blit-core/src/transfer_session/mod.rs-736-        return Err(notify_and_wrap(
crates/blit-core/src/transfer_session/mod.rs-737-            transport,
crates/blit-core/src/transfer_session/mod.rs-738-            SessionFault::new(
crates/blit-core/src/transfer_session/mod.rs-739-                session_error::Code::ChecksumDisabled,
crates/blit-core/src/transfer_session/mod.rs-740-                "checksum comparison is disabled on this daemon \
crates/blit-core/src/transfer_session/mod.rs-741-                 (--no-server-checksums / server_checksums_enabled = false)",
crates/blit-core/src/transfer_session/mod.rs-742-            ),
crates/blit-core/src/transfer_session/mod.rs-743-        )
crates/blit-core/src/transfer_session/mod.rs-744-        .await);
crates/blit-core/src/transfer_session/mod.rs-745-    }
crates/blit-core/src/transfer_session/mod.rs-746-    if let Err(fault) = validate_open(&open) {
crates/blit-core/src/transfer_session/mod.rs-747-        // Refusal is a SessionError instead of SessionAccept,
crates/blit-core/src/transfer_session/mod.rs-748-        // never a silent close (contract §Phase state machine).
crates/blit-core/src/transfer_session/mod.rs-749-        return Err(notify_and_wrap(transport, fault).await);
crates/blit-core/src/transfer_session/mod.rs-750-    }
crates/blit-core/src/transfer_session/mod.rs-751-    // Responder endpoint resolution (otp-4): map the wire
crates/blit-core/src/transfer_session/mod.rs-752-    // module/path to a local root and enforce read-only, both
crates/blit-core/src/transfer_session/mod.rs:753:    // BEFORE SessionAccept so a refusal replaces the accept
crates/blit-core/src/transfer_session/mod.rs-754-    // (never follows it). The resolver is caller-supplied
crates/blit-core/src/transfer_session/mod.rs-755-    // (daemon module lookup); a fixed-root responder passes
crates/blit-core/src/transfer_session/mod.rs-756-    // None and resolves nothing here.
crates/blit-core/src/transfer_session/mod.rs-757-    let resolved_root = match resolve_open {
crates/blit-core/src/transfer_session/mod.rs-758-        Some(resolve) => match resolve(&open).await {
crates/blit-core/src/transfer_session/mod.rs-759-            Ok(resolved) => {
crates/blit-core/src/transfer_session/mod.rs-760-                // A read-only module is fatal only for a
crates/blit-core/src/transfer_session/mod.rs-761-                // DESTINATION (it would write); a SOURCE
crates/blit-core/src/transfer_session/mod.rs-762-                // responder (otp-5, daemon-send) reads happily.
crates/blit-core/src/transfer_session/mod.rs-763-                if local_role == TransferRole::Destination && resolved.read_only {
crates/blit-core/src/transfer_session/mod.rs-764-                    return Err(notify_and_wrap(
crates/blit-core/src/transfer_session/mod.rs-765-                        transport,
crates/blit-core/src/transfer_session/mod.rs-766-                        SessionFault::read_only("destination module is read-only".to_string()),
crates/blit-core/src/transfer_session/mod.rs-767-                    )
crates/blit-core/src/transfer_session/mod.rs-768-                    .await);
crates/blit-core/src/transfer_session/mod.rs-769-                }
crates/blit-core/src/transfer_session/mod.rs-770-                Some(resolved.root)
crates/blit-core/src/transfer_session/mod.rs-771-            }
--
crates/blit-core/src/transfer_session/mod.rs-799-        receiver_capacity: if local_role == TransferRole::Destination {
crates/blit-core/src/transfer_session/mod.rs-800-            Some(crate::dial::local_receiver_capacity())
crates/blit-core/src/transfer_session/mod.rs-801-        } else {
crates/blit-core/src/transfer_session/mod.rs-802-            None
crates/blit-core/src/transfer_session/mod.rs-803-        },
crates/blit-core/src/transfer_session/mod.rs-804-        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
crates/blit-core/src/transfer_session/mod.rs-805-        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
crates/blit-core/src/transfer_session/mod.rs-806-    };
crates/blit-core/src/transfer_session/mod.rs-807-    transport.send(frame(Frame::Accept(accept.clone()))).await?;
crates/blit-core/src/transfer_session/mod.rs-808-    Ok(Negotiated {
crates/blit-core/src/transfer_session/mod.rs-809-        open,
crates/blit-core/src/transfer_session/mod.rs-810-        accept,
crates/blit-core/src/transfer_session/mod.rs-811-        resolved_root,
crates/blit-core/src/transfer_session/mod.rs-812-        responder_data_plane,
crates/blit-core/src/transfer_session/mod.rs-813-    })
crates/blit-core/src/transfer_session/mod.rs-814-}
crates/blit-core/src/transfer_session/mod.rs-815-
crates/blit-core/src/transfer_session/mod.rs-816-/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
crates/blit-core/src/transfer_session/mod.rs:817:/// scoping requirement). Sends the refusal `SessionError` itself when
crates/blit-core/src/transfer_session/mod.rs-818-/// it detects the fault locally; returned faults are `peer_notified`.
crates/blit-core/src/transfer_session/mod.rs-819-async fn establish(
crates/blit-core/src/transfer_session/mod.rs-820-    transport: &mut FrameTransport,
crates/blit-core/src/transfer_session/mod.rs-821-    hello: &HelloConfig,
crates/blit-core/src/transfer_session/mod.rs-822-    endpoint: &SessionEndpoint,
crates/blit-core/src/transfer_session/mod.rs-823-    local_role: TransferRole,
crates/blit-core/src/transfer_session/mod.rs-824-    validate_open: &OpenValidator,
crates/blit-core/src/transfer_session/mod.rs-825-    // Consulted only on the Responder branch, after the received open
crates/blit-core/src/transfer_session/mod.rs-826-    // passes `validate_open` and before SessionAccept. `None` = the
crates/blit-core/src/transfer_session/mod.rs-827-    // caller supplies the root itself (Initiator, or fixed-root test).
crates/blit-core/src/transfer_session/mod.rs-828-    resolve_open: Option<&OpenResolver>,
crates/blit-core/src/transfer_session/mod.rs-829-) -> Result<Negotiated> {
crates/blit-core/src/transfer_session/mod.rs-830-    exchange_hello(transport, hello).await?;
crates/blit-core/src/transfer_session/mod.rs-831-
crates/blit-core/src/transfer_session/mod.rs-832-    match endpoint {
crates/blit-core/src/transfer_session/mod.rs-833-        SessionEndpoint::Initiator { open } => {
crates/blit-core/src/transfer_session/mod.rs-834-            let open = open.as_ref().clone();
crates/blit-core/src/transfer_session/mod.rs-835-            transport.send(frame(Frame::Open(open.clone()))).await?;
--
crates/blit-core/src/transfer_session/mod.rs-913-
crates/blit-core/src/transfer_session/mod.rs-914-/// Events the source's receive half forwards to its send half. The
crates/blit-core/src/transfer_session/mod.rs-915-/// channel is unbounded but bounded by construction: every `Need`
crates/blit-core/src/transfer_session/mod.rs-916-/// consumes a distinct sent-manifest entry (unknown or repeated paths
crates/blit-core/src/transfer_session/mod.rs-917-/// fault the session), so the queue never exceeds the source's own
crates/blit-core/src/transfer_session/mod.rs-918-/// manifest size — the contract's bounded-buffering rule holds.
crates/blit-core/src/transfer_session/mod.rs-919-enum SourceEvent {
crates/blit-core/src/transfer_session/mod.rs-920-    Need(FileHeader),
crates/blit-core/src/transfer_session/mod.rs-921-    /// A resume-flagged need (otp-7a). The send half HOLDS it until the
crates/blit-core/src/transfer_session/mod.rs-922-    /// destination's `BlockHashList` for the same path arrives — the
crates/blit-core/src/transfer_session/mod.rs-923-    /// contract's RELIABLE ordering guarantee: no byte of a resume file
crates/blit-core/src/transfer_session/mod.rs-924-    /// moves before its hash list.
crates/blit-core/src/transfer_session/mod.rs-925-    ResumeNeed(FileHeader),
crates/blit-core/src/transfer_session/mod.rs-926-    /// The destination's block hashes for a held resume need (otp-7a).
crates/blit-core/src/transfer_session/mod.rs-927-    BlockHashes(BlockHashList),
crates/blit-core/src/transfer_session/mod.rs-928-    NeedComplete,
crates/blit-core/src/transfer_session/mod.rs-929-    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
crates/blit-core/src/transfer_session/mod.rs-930-    /// send half dials the epoch-N socket on `accepted`.
crates/blit-core/src/transfer_session/mod.rs:931:    ResizeAck(DataPlaneResizeAck),
crates/blit-core/src/transfer_session/mod.rs-932-    Summary(TransferSummary),
crates/blit-core/src/transfer_session/mod.rs-933-    Fault(SessionFault),
crates/blit-core/src/transfer_session/mod.rs-934-}
crates/blit-core/src/transfer_session/mod.rs-935-
crates/blit-core/src/transfer_session/mod.rs-936-/// The receive half's event sender, mirroring every `Fault` onto a
crates/blit-core/src/transfer_session/mod.rs-937-/// `watch` signal as it is queued. The in-stream send path races this
crates/blit-core/src/transfer_session/mod.rs-938-/// signal against its (potentially blocked) record sends — codex otp-8
crates/blit-core/src/transfer_session/mod.rs-939-/// F1: a peer fault (CANCELLED above all) must interrupt a send half
crates/blit-core/src/transfer_session/mod.rs-940-/// stuck inside `reader.read()`/`tx.send()`, exactly as the data-plane
crates/blit-core/src/transfer_session/mod.rs-941-/// drain's `recv_peer_fault` arm does for socket sends. The mpsc queue
crates/blit-core/src/transfer_session/mod.rs-942-/// still carries the fault for the between-send paths; the watch is a
crates/blit-core/src/transfer_session/mod.rs-943-/// non-consuming side channel, so mid-send `Need`s stay queued.
crates/blit-core/src/transfer_session/mod.rs-944-struct SourceEventSender {
crates/blit-core/src/transfer_session/mod.rs-945-    tx: mpsc::UnboundedSender<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs-946-    fault_signal: watch::Sender<Option<SessionFault>>,
crates/blit-core/src/transfer_session/mod.rs-947-}
crates/blit-core/src/transfer_session/mod.rs-948-
crates/blit-core/src/transfer_session/mod.rs-949-impl SourceEventSender {
--
crates/blit-core/src/transfer_session/mod.rs-1177-                    )));
crates/blit-core/src/transfer_session/mod.rs-1178-                    return;
crates/blit-core/src/transfer_session/mod.rs-1179-                }
crates/blit-core/src/transfer_session/mod.rs-1180-                let _ = events.send(SourceEvent::BlockHashes(list));
crates/blit-core/src/transfer_session/mod.rs-1181-            }
crates/blit-core/src/transfer_session/mod.rs-1182-            Some(Frame::NeedComplete(_)) => {
crates/blit-core/src/transfer_session/mod.rs-1183-                if !manifest_sent.load(Ordering::Acquire) {
crates/blit-core/src/transfer_session/mod.rs-1184-                    // Fail fast at arrival time (otp-3 codex F2): the
crates/blit-core/src/transfer_session/mod.rs-1185-                    // event queue would otherwise let an early
crates/blit-core/src/transfer_session/mod.rs-1186-                    // NeedComplete be processed late and pass as
crates/blit-core/src/transfer_session/mod.rs-1187-                    // legitimate.
crates/blit-core/src/transfer_session/mod.rs-1188-                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1189-                        "NeedComplete before the source's ManifestComplete",
crates/blit-core/src/transfer_session/mod.rs-1190-                    )));
crates/blit-core/src/transfer_session/mod.rs-1191-                    return;
crates/blit-core/src/transfer_session/mod.rs-1192-                }
crates/blit-core/src/transfer_session/mod.rs-1193-                let _ = events.send(SourceEvent::NeedComplete);
crates/blit-core/src/transfer_session/mod.rs-1194-            }
crates/blit-core/src/transfer_session/mod.rs:1195:            Some(Frame::ResizeAck(ack)) => {
crates/blit-core/src/transfer_session/mod.rs:1196:                // The destination's response to a shape-resize proposal
crates/blit-core/src/transfer_session/mod.rs-1197-                // (otp-4b-2). Forward it to the send half, which owns the
crates/blit-core/src/transfer_session/mod.rs-1198-                // dial and dials the epoch-N socket on `accepted`.
crates/blit-core/src/transfer_session/mod.rs:1199:                let _ = events.send(SourceEvent::ResizeAck(ack));
crates/blit-core/src/transfer_session/mod.rs-1200-            }
crates/blit-core/src/transfer_session/mod.rs-1201-            Some(Frame::Summary(summary)) => {
crates/blit-core/src/transfer_session/mod.rs-1202-                let _ = events.send(SourceEvent::Summary(summary));
crates/blit-core/src/transfer_session/mod.rs-1203-                return;
crates/blit-core/src/transfer_session/mod.rs-1204-            }
crates/blit-core/src/transfer_session/mod.rs-1205-            Some(Frame::Error(err)) => {
crates/blit-core/src/transfer_session/mod.rs-1206-                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs-1207-                return;
crates/blit-core/src/transfer_session/mod.rs-1208-            }
crates/blit-core/src/transfer_session/mod.rs-1209-            other => {
crates/blit-core/src/transfer_session/mod.rs-1210-                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1211-                    format!("{} on the source's receive lane", frame_name(&other)),
crates/blit-core/src/transfer_session/mod.rs-1212-                )));
crates/blit-core/src/transfer_session/mod.rs-1213-                return;
crates/blit-core/src/transfer_session/mod.rs-1214-            }
crates/blit-core/src/transfer_session/mod.rs-1215-        }
crates/blit-core/src/transfer_session/mod.rs-1216-    }
crates/blit-core/src/transfer_session/mod.rs-1217-}
--
crates/blit-core/src/transfer_session/mod.rs-1316-    // were validated at OPEN (`source_open_validator`), so the conversion
crates/blit-core/src/transfer_session/mod.rs-1317-    // cannot fail on a validated open; map any error to a fault regardless.
crates/blit-core/src/transfer_session/mod.rs-1318-    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
crates/blit-core/src/transfer_session/mod.rs-1319-        Some(spec) if *spec != FilterSpec::default() => {
crates/blit-core/src/transfer_session/mod.rs-1320-            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
crates/blit-core/src/transfer_session/mod.rs-1321-                .map_err(|e| {
crates/blit-core/src/transfer_session/mod.rs-1322-                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
crates/blit-core/src/transfer_session/mod.rs-1323-                })?;
crates/blit-core/src/transfer_session/mod.rs-1324-            Arc::new(crate::remote::transfer::source::FilteredSource::new(
crates/blit-core/src/transfer_session/mod.rs-1325-                Arc::clone(&source),
crates/blit-core/src/transfer_session/mod.rs-1326-                filter,
crates/blit-core/src/transfer_session/mod.rs-1327-            ))
crates/blit-core/src/transfer_session/mod.rs-1328-        }
crates/blit-core/src/transfer_session/mod.rs-1329-        _ => Arc::clone(&source),
crates/blit-core/src/transfer_session/mod.rs-1330-    };
crates/blit-core/src/transfer_session/mod.rs-1331-    // otp-10b-1: a Checksum session fills each manifest header's
crates/blit-core/src/transfer_session/mod.rs-1332-    // checksum so the DESTINATION can skip content-equal files
crates/blit-core/src/transfer_session/mod.rs-1333-    // regardless of mtime. Wrapped OUTSIDE the filter so only
crates/blit-core/src/transfer_session/mod.rs:1334:    // in-scope files pay the hash; a serving end that refuses to hash
crates/blit-core/src/transfer_session/mod.rs-1335-    // never gets here (CHECKSUM_DISABLED at OPEN).
crates/blit-core/src/transfer_session/mod.rs-1336-    let scan_source: Arc<dyn TransferSource> =
crates/blit-core/src/transfer_session/mod.rs-1337-        if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
crates/blit-core/src/transfer_session/mod.rs-1338-            Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
crates/blit-core/src/transfer_session/mod.rs-1339-                scan_source,
crates/blit-core/src/transfer_session/mod.rs-1340-            ))
crates/blit-core/src/transfer_session/mod.rs-1341-        } else {
crates/blit-core/src/transfer_session/mod.rs-1342-            scan_source
crates/blit-core/src/transfer_session/mod.rs-1343-        };
crates/blit-core/src/transfer_session/mod.rs-1344-    // otp-10a: callers that must not treat a partial transfer as success
crates/blit-core/src/transfer_session/mod.rs-1345-    // (the push verb, `blit move`'s source-delete gate) supply their own
crates/blit-core/src/transfer_session/mod.rs-1346-    // accumulator via `SourceInstruments` and inspect it after the
crates/blit-core/src/transfer_session/mod.rs-1347-    // session returns; the wire behavior is identical either way.
crates/blit-core/src/transfer_session/mod.rs-1348-    let unreadable: Arc<StdMutex<Vec<String>>> = instruments.unreadable.clone().unwrap_or_default();
crates/blit-core/src/transfer_session/mod.rs-1349-    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
crates/blit-core/src/transfer_session/mod.rs-1350-    while let Some(header) = header_rx.recv().await {
crates/blit-core/src/transfer_session/mod.rs-1351-        sent.lock()
crates/blit-core/src/transfer_session/mod.rs-1352-            .expect("sent-manifest lock poisoned")
--
crates/blit-core/src/transfer_session/mod.rs-1405-            data_plane.as_ref(),
crates/blit-core/src/transfer_session/mod.rs-1406-            tx,
crates/blit-core/src/transfer_session/mod.rs-1407-            &mut pending_resize,
crates/blit-core/src/transfer_session/mod.rs-1408-        )
crates/blit-core/src/transfer_session/mod.rs-1409-        .await?;
crates/blit-core/src/transfer_session/mod.rs-1410-        if !pending.is_empty() {
crates/blit-core/src/transfer_session/mod.rs-1411-            let batch = std::mem::take(&mut pending);
crates/blit-core/src/transfer_session/mod.rs-1412-            match &mut data_plane {
crates/blit-core/src/transfer_session/mod.rs-1413-                Some(dp) => {
crates/blit-core/src/transfer_session/mod.rs-1414-                    // sf-2: correct the stream count toward the shape the
crates/blit-core/src/transfer_session/mod.rs-1415-                    // accumulated need list implies before queueing this
crates/blit-core/src/transfer_session/mod.rs-1416-                    // batch. Settle the whole shape-derived target before
crates/blit-core/src/transfer_session/mod.rs-1417-                    // handing payloads to the pipeline: otherwise the
crates/blit-core/src/transfer_session/mod.rs-1418-                    // one-ADD-per-epoch ramp races NeedComplete/payload
crates/blit-core/src/transfer_session/mod.rs-1419-                    // drain, so a fast transfer can finish at a different
crates/blit-core/src/transfer_session/mod.rs-1420-                    // worker count depending on which endpoint initiated.
crates/blit-core/src/transfer_session/mod.rs-1421-                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
crates/blit-core/src/transfer_session/mod.rs-1422-                        .await?;
crates/blit-core/src/transfer_session/mod.rs:1423:                    settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs-1424-                        &mut events,
crates/blit-core/src/transfer_session/mod.rs-1425-                        &mut pending,
crates/blit-core/src/transfer_session/mod.rs-1426-                        &mut resume,
crates/blit-core/src/transfer_session/mod.rs-1427-                        &mut need_complete,
crates/blit-core/src/transfer_session/mod.rs-1428-                        &mut needed_bytes,
crates/blit-core/src/transfer_session/mod.rs-1429-                        &mut needed_count,
crates/blit-core/src/transfer_session/mod.rs-1430-                        dp,
crates/blit-core/src/transfer_session/mod.rs-1431-                        tx,
crates/blit-core/src/transfer_session/mod.rs-1432-                        &mut pending_resize,
crates/blit-core/src/transfer_session/mod.rs-1433-                    )
crates/blit-core/src/transfer_session/mod.rs-1434-                    .await?;
crates/blit-core/src/transfer_session/mod.rs-1435-                    let payloads =
crates/blit-core/src/transfer_session/mod.rs-1436-                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
crates/blit-core/src/transfer_session/mod.rs-1437-                    // A cancel while earlier batches are actively moving
crates/blit-core/src/transfer_session/mod.rs-1438-                    // closes the send pipeline under backpressure, so this
crates/blit-core/src/transfer_session/mod.rs-1439-                    // queue fails with a data-plane error — prefer the
crates/blit-core/src/transfer_session/mod.rs-1440-                    // peer's framed reason (CANCELLED) the same way the
crates/blit-core/src/transfer_session/mod.rs-1441-                    // finish() drain does (otp-4b-3 codex F1). Not raced
--
crates/blit-core/src/transfer_session/mod.rs-1475-            continue;
crates/blit-core/src/transfer_session/mod.rs-1476-        }
crates/blit-core/src/transfer_session/mod.rs-1477-        if !resume.ready.is_empty() {
crates/blit-core/src/transfer_session/mod.rs-1478-            // The block phase for correlated (need, hash-list) pairs.
crates/blit-core/src/transfer_session/mod.rs-1479-            // Data plane (otp-7b): each pair becomes ONE composite
crates/blit-core/src/transfer_session/mod.rs-1480-            // ResumeFile work item, so one pipeline worker runs the
crates/blit-core/src/transfer_session/mod.rs-1481-            // whole record on one socket — strict per-file serialization
crates/blit-core/src/transfer_session/mod.rs-1482-            // without cross-socket reorder hazards. In-stream (otp-7a):
crates/blit-core/src/transfer_session/mod.rs-1483-            // control-lane BlockTransfer/Complete frames, as before.
crates/blit-core/src/transfer_session/mod.rs-1484-            let ready = std::mem::take(&mut resume.ready);
crates/blit-core/src/transfer_session/mod.rs-1485-            match &mut data_plane {
crates/blit-core/src/transfer_session/mod.rs-1486-                Some(dp) => {
crates/blit-core/src/transfer_session/mod.rs-1487-                    // codex 7b-1 F4: resume batches drive the sf-2 shape
crates/blit-core/src/transfer_session/mod.rs-1488-                    // correction exactly as plain batches do — a
crates/blit-core/src/transfer_session/mod.rs-1489-                    // resume-heavy need list must not stay pinned to the
crates/blit-core/src/transfer_session/mod.rs-1490-                    // zero-knowledge single stream.
crates/blit-core/src/transfer_session/mod.rs-1491-                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
crates/blit-core/src/transfer_session/mod.rs-1492-                        .await?;
crates/blit-core/src/transfer_session/mod.rs:1493:                    settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs-1494-                        &mut events,
crates/blit-core/src/transfer_session/mod.rs-1495-                        &mut pending,
crates/blit-core/src/transfer_session/mod.rs-1496-                        &mut resume,
crates/blit-core/src/transfer_session/mod.rs-1497-                        &mut need_complete,
crates/blit-core/src/transfer_session/mod.rs-1498-                        &mut needed_bytes,
crates/blit-core/src/transfer_session/mod.rs-1499-                        &mut needed_count,
crates/blit-core/src/transfer_session/mod.rs-1500-                        dp,
crates/blit-core/src/transfer_session/mod.rs-1501-                        tx,
crates/blit-core/src/transfer_session/mod.rs-1502-                        &mut pending_resize,
crates/blit-core/src/transfer_session/mod.rs-1503-                    )
crates/blit-core/src/transfer_session/mod.rs-1504-                    .await?;
crates/blit-core/src/transfer_session/mod.rs-1505-                    let payloads = ready
crates/blit-core/src/transfer_session/mod.rs-1506-                        .into_iter()
crates/blit-core/src/transfer_session/mod.rs-1507-                        .map(|(header, hashes)| TransferPayload::ResumeFile {
crates/blit-core/src/transfer_session/mod.rs-1508-                            header,
crates/blit-core/src/transfer_session/mod.rs-1509-                            block_size: hashes.block_size,
crates/blit-core/src/transfer_session/mod.rs-1510-                            dest_hashes: hashes.hashes,
crates/blit-core/src/transfer_session/mod.rs-1511-                        })
--
crates/blit-core/src/transfer_session/mod.rs-1617-    // CLOSING: the destination is the scorer; the next event must be
crates/blit-core/src/transfer_session/mod.rs-1618-    // its summary (the receive half ends after forwarding it).
crates/blit-core/src/transfer_session/mod.rs-1619-    match events.recv().await {
crates/blit-core/src/transfer_session/mod.rs-1620-        Some(SourceEvent::Summary(summary)) => Ok(summary),
crates/blit-core/src/transfer_session/mod.rs-1621-        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs-1622-        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs-1623-            Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1624-                format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs-1625-            )))
crates/blit-core/src/transfer_session/mod.rs-1626-        }
crates/blit-core/src/transfer_session/mod.rs-1627-        Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs-1628-            Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1629-                format!("BlockHashList for '{}' after SourceDone", l.relative_path),
crates/blit-core/src/transfer_session/mod.rs-1630-            )))
crates/blit-core/src/transfer_session/mod.rs-1631-        }
crates/blit-core/src/transfer_session/mod.rs-1632-        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs-1633-            SessionFault::protocol_violation("duplicate NeedComplete"),
crates/blit-core/src/transfer_session/mod.rs-1634-        )),
crates/blit-core/src/transfer_session/mod.rs:1635:        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
crates/blit-core/src/transfer_session/mod.rs:1636:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
crates/blit-core/src/transfer_session/mod.rs-1637-        )),
crates/blit-core/src/transfer_session/mod.rs-1638-        None => Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs-1639-            "source receive half ended before TransferSummary",
crates/blit-core/src/transfer_session/mod.rs-1640-        ))),
crates/blit-core/src/transfer_session/mod.rs-1641-    }
crates/blit-core/src/transfer_session/mod.rs-1642-}
crates/blit-core/src/transfer_session/mod.rs-1643-
crates/blit-core/src/transfer_session/mod.rs-1644-/// Process every event ready right now (needs accumulating, resize acks
crates/blit-core/src/transfer_session/mod.rs-1645-/// dialing their epoch-N socket) without blocking. Called between
crates/blit-core/src/transfer_session/mod.rs-1646-/// manifest sends and at the top of the payload loop.
crates/blit-core/src/transfer_session/mod.rs-1647-#[allow(clippy::too_many_arguments)]
crates/blit-core/src/transfer_session/mod.rs-1648-async fn drain_ready_source_events(
crates/blit-core/src/transfer_session/mod.rs-1649-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs-1650-    pending: &mut Vec<FileHeader>,
crates/blit-core/src/transfer_session/mod.rs-1651-    resume: &mut ResumeSendState,
crates/blit-core/src/transfer_session/mod.rs-1652-    need_complete: &mut bool,
crates/blit-core/src/transfer_session/mod.rs-1653-    needed_bytes: &mut u64,
crates/blit-core/src/transfer_session/mod.rs-1654-    needed_count: &mut usize,
--
crates/blit-core/src/transfer_session/mod.rs-1659-    while let Ok(event) = events.try_recv() {
crates/blit-core/src/transfer_session/mod.rs-1660-        process_source_event(
crates/blit-core/src/transfer_session/mod.rs-1661-            event,
crates/blit-core/src/transfer_session/mod.rs-1662-            pending,
crates/blit-core/src/transfer_session/mod.rs-1663-            resume,
crates/blit-core/src/transfer_session/mod.rs-1664-            need_complete,
crates/blit-core/src/transfer_session/mod.rs-1665-            needed_bytes,
crates/blit-core/src/transfer_session/mod.rs-1666-            needed_count,
crates/blit-core/src/transfer_session/mod.rs-1667-            data_plane,
crates/blit-core/src/transfer_session/mod.rs-1668-            tx,
crates/blit-core/src/transfer_session/mod.rs-1669-            pending_resize,
crates/blit-core/src/transfer_session/mod.rs-1670-        )
crates/blit-core/src/transfer_session/mod.rs-1671-        .await?;
crates/blit-core/src/transfer_session/mod.rs-1672-    }
crates/blit-core/src/transfer_session/mod.rs-1673-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-1674-}
crates/blit-core/src/transfer_session/mod.rs-1675-
crates/blit-core/src/transfer_session/mod.rs-1676-/// Handle one source event. Needs accumulate into `pending` and the
crates/blit-core/src/transfer_session/mod.rs:1677:/// shape totals; a resize ack dials its epoch-N socket and proposes the
crates/blit-core/src/transfer_session/mod.rs-1678-/// next ADD (the one-per-epoch ramp).
crates/blit-core/src/transfer_session/mod.rs-1679-#[allow(clippy::too_many_arguments)]
crates/blit-core/src/transfer_session/mod.rs-1680-async fn process_source_event(
crates/blit-core/src/transfer_session/mod.rs-1681-    event: SourceEvent,
crates/blit-core/src/transfer_session/mod.rs-1682-    pending: &mut Vec<FileHeader>,
crates/blit-core/src/transfer_session/mod.rs-1683-    resume: &mut ResumeSendState,
crates/blit-core/src/transfer_session/mod.rs-1684-    need_complete: &mut bool,
crates/blit-core/src/transfer_session/mod.rs-1685-    needed_bytes: &mut u64,
crates/blit-core/src/transfer_session/mod.rs-1686-    needed_count: &mut usize,
crates/blit-core/src/transfer_session/mod.rs-1687-    data_plane: Option<&data_plane::SourceDataPlane>,
crates/blit-core/src/transfer_session/mod.rs-1688-    tx: &mut Box<dyn FrameTx>,
crates/blit-core/src/transfer_session/mod.rs-1689-    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs-1690-) -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-1691-    match event {
crates/blit-core/src/transfer_session/mod.rs-1692-        SourceEvent::Need(header) => {
crates/blit-core/src/transfer_session/mod.rs-1693-            if *need_complete {
crates/blit-core/src/transfer_session/mod.rs-1694-                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1695-                    format!("need for '{}' after NeedComplete", header.relative_path),
--
crates/blit-core/src/transfer_session/mod.rs-1764-                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs-1765-                )));
crates/blit-core/src/transfer_session/mod.rs-1766-            }
crates/blit-core/src/transfer_session/mod.rs-1767-            // Ordered lane: the destination sends every BlockHashList
crates/blit-core/src/transfer_session/mod.rs-1768-            // before its NeedComplete, so a still-held resume need here
crates/blit-core/src/transfer_session/mod.rs-1769-            // means the peer broke the choreography — fail fast rather
crates/blit-core/src/transfer_session/mod.rs-1770-            // than hang waiting for a list that can no longer arrive.
crates/blit-core/src/transfer_session/mod.rs-1771-            if !resume.held.is_empty() {
crates/blit-core/src/transfer_session/mod.rs-1772-                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1773-                    format!(
crates/blit-core/src/transfer_session/mod.rs-1774-                        "NeedComplete with {} resume need(s) missing their BlockHashList",
crates/blit-core/src/transfer_session/mod.rs-1775-                        resume.held.len()
crates/blit-core/src/transfer_session/mod.rs-1776-                    ),
crates/blit-core/src/transfer_session/mod.rs-1777-                )));
crates/blit-core/src/transfer_session/mod.rs-1778-            }
crates/blit-core/src/transfer_session/mod.rs-1779-            *need_complete = true;
crates/blit-core/src/transfer_session/mod.rs-1780-            Ok(())
crates/blit-core/src/transfer_session/mod.rs-1781-        }
crates/blit-core/src/transfer_session/mod.rs:1782:        SourceEvent::ResizeAck(ack) => {
crates/blit-core/src/transfer_session/mod.rs-1783-            let dp = data_plane.ok_or_else(|| {
crates/blit-core/src/transfer_session/mod.rs-1784-                eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1785:                    "DataPlaneResizeAck on a session with no data plane",
crates/blit-core/src/transfer_session/mod.rs-1786-                ))
crates/blit-core/src/transfer_session/mod.rs-1787-            })?;
crates/blit-core/src/transfer_session/mod.rs-1788-            // Match the ack to the in-flight proposal; stale/unsolicited
crates/blit-core/src/transfer_session/mod.rs-1789-            // acks (wrong epoch, or none pending) are ignored, matching
crates/blit-core/src/transfer_session/mod.rs-1790-            // old push. `take()` + restore keeps the borrow simple.
crates/blit-core/src/transfer_session/mod.rs-1791-            let pending_r = match pending_resize.take() {
crates/blit-core/src/transfer_session/mod.rs-1792-                Some(p) if p.epoch == ack.epoch => p,
crates/blit-core/src/transfer_session/mod.rs-1793-                restored => {
crates/blit-core/src/transfer_session/mod.rs-1794-                    *pending_resize = restored;
crates/blit-core/src/transfer_session/mod.rs-1795-                    return Ok(());
crates/blit-core/src/transfer_session/mod.rs-1796-                }
crates/blit-core/src/transfer_session/mod.rs-1797-            };
crates/blit-core/src/transfer_session/mod.rs-1798-            if ack.accepted {
crates/blit-core/src/transfer_session/mod.rs-1799-                dp.add_stream(&pending_r.sub_token).await?;
crates/blit-core/src/transfer_session/mod.rs-1800-                dp.dial()
crates/blit-core/src/transfer_session/mod.rs:1801:                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
crates/blit-core/src/transfer_session/mod.rs-1802-                // Ramp one stream per accepted epoch: propose the next ADD.
crates/blit-core/src/transfer_session/mod.rs-1803-                maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
crates/blit-core/src/transfer_session/mod.rs-1804-            } else {
crates/blit-core/src/transfer_session/mod.rs-1805-                dp.dial()
crates/blit-core/src/transfer_session/mod.rs:1806:                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
crates/blit-core/src/transfer_session/mod.rs:1807:                // A refusal is terminal for this shape ramp. Retrying the
crates/blit-core/src/transfer_session/mod.rs-1808-                // same unattainable target under a fresh epoch would loop
crates/blit-core/src/transfer_session/mod.rs-1809-                // forever; the settled live set still carries the transfer.
crates/blit-core/src/transfer_session/mod.rs-1810-                Ok(())
crates/blit-core/src/transfer_session/mod.rs-1811-            }
crates/blit-core/src/transfer_session/mod.rs-1812-        }
crates/blit-core/src/transfer_session/mod.rs-1813-        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1814-            "TransferSummary before SourceDone",
crates/blit-core/src/transfer_session/mod.rs-1815-        ))),
crates/blit-core/src/transfer_session/mod.rs-1816-        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs-1817-    }
crates/blit-core/src/transfer_session/mod.rs-1818-}
crates/blit-core/src/transfer_session/mod.rs-1819-
crates/blit-core/src/transfer_session/mod.rs:1820:/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
crates/blit-core/src/transfer_session/mod.rs-1821-/// the stream count the accumulated need list implies, if none is in
crates/blit-core/src/transfer_session/mod.rs-1822-/// flight. A no-op when the shape wants no more than the live count (the
crates/blit-core/src/transfer_session/mod.rs-1823-/// dial returns `None`). Sends the frame and records the in-flight
crates/blit-core/src/transfer_session/mod.rs-1824-/// proposal for the ack to match.
crates/blit-core/src/transfer_session/mod.rs-1825-async fn maybe_propose_resize(
crates/blit-core/src/transfer_session/mod.rs-1826-    dp: &data_plane::SourceDataPlane,
crates/blit-core/src/transfer_session/mod.rs-1827-    tx: &mut Box<dyn FrameTx>,
crates/blit-core/src/transfer_session/mod.rs-1828-    needed_bytes: u64,
crates/blit-core/src/transfer_session/mod.rs-1829-    needed_count: usize,
crates/blit-core/src/transfer_session/mod.rs-1830-    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs-1831-) -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-1832-    if pending_resize.is_some() {
crates/blit-core/src/transfer_session/mod.rs-1833-        return Ok(());
crates/blit-core/src/transfer_session/mod.rs-1834-    }
crates/blit-core/src/transfer_session/mod.rs-1835-    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
crates/blit-core/src/transfer_session/mod.rs-1836-        tx.send(frame(Frame::Resize(DataPlaneResize {
crates/blit-core/src/transfer_session/mod.rs-1837-            op: DataPlaneResizeOp::Add as i32,
crates/blit-core/src/transfer_session/mod.rs-1838-            epoch: proposal.epoch,
crates/blit-core/src/transfer_session/mod.rs-1839-            target_stream_count: proposal.target_streams,
crates/blit-core/src/transfer_session/mod.rs-1840-            sub_token: proposal.sub_token.clone(),
crates/blit-core/src/transfer_session/mod.rs-1841-        })))
crates/blit-core/src/transfer_session/mod.rs-1842-        .await?;
crates/blit-core/src/transfer_session/mod.rs-1843-        *pending_resize = Some(proposal);
crates/blit-core/src/transfer_session/mod.rs-1844-    }
crates/blit-core/src/transfer_session/mod.rs-1845-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-1846-}
crates/blit-core/src/transfer_session/mod.rs-1847-
crates/blit-core/src/transfer_session/mod.rs-1848-/// Drive the one-stream-per-epoch shape ramp to its currently known target
crates/blit-core/src/transfer_session/mod.rs-1849-/// before payload dispatch. Needs and resume hashes may continue arriving
crates/blit-core/src/transfer_session/mod.rs-1850-/// while an ack is in flight, so process the shared SOURCE event lane rather
crates/blit-core/src/transfer_session/mod.rs-1851-/// than waiting for only an ack. Each accepted ack proposes the next epoch
crates/blit-core/src/transfer_session/mod.rs-1852-/// from the latest accumulated shape; the loop ends only when no proposal is
crates/blit-core/src/transfer_session/mod.rs:1853:/// outstanding (target reached or the destination refused growth).
crates/blit-core/src/transfer_session/mod.rs-1854-#[allow(clippy::too_many_arguments)]
crates/blit-core/src/transfer_session/mod.rs:1855:async fn settle_shape_resizes(
crates/blit-core/src/transfer_session/mod.rs-1856-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs-1857-    pending: &mut Vec<FileHeader>,
crates/blit-core/src/transfer_session/mod.rs-1858-    resume: &mut ResumeSendState,
crates/blit-core/src/transfer_session/mod.rs-1859-    need_complete: &mut bool,
crates/blit-core/src/transfer_session/mod.rs-1860-    needed_bytes: &mut u64,
crates/blit-core/src/transfer_session/mod.rs-1861-    needed_count: &mut usize,
crates/blit-core/src/transfer_session/mod.rs-1862-    data_plane: &data_plane::SourceDataPlane,
crates/blit-core/src/transfer_session/mod.rs-1863-    tx: &mut Box<dyn FrameTx>,
crates/blit-core/src/transfer_session/mod.rs-1864-    pending_resize: &mut Option<data_plane::PendingResize>,
crates/blit-core/src/transfer_session/mod.rs-1865-) -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-1866-    while pending_resize.is_some() {
crates/blit-core/src/transfer_session/mod.rs-1867-        let event = events.recv().await.ok_or_else(|| {
crates/blit-core/src/transfer_session/mod.rs-1868-            eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1869:                "source receive half ended during data-plane shape resize",
crates/blit-core/src/transfer_session/mod.rs-1870-            ))
crates/blit-core/src/transfer_session/mod.rs-1871-        })?;
crates/blit-core/src/transfer_session/mod.rs-1872-        process_source_event(
crates/blit-core/src/transfer_session/mod.rs-1873-            event,
crates/blit-core/src/transfer_session/mod.rs-1874-            pending,
crates/blit-core/src/transfer_session/mod.rs-1875-            resume,
crates/blit-core/src/transfer_session/mod.rs-1876-            need_complete,
crates/blit-core/src/transfer_session/mod.rs-1877-            needed_bytes,
crates/blit-core/src/transfer_session/mod.rs-1878-            needed_count,
crates/blit-core/src/transfer_session/mod.rs-1879-            Some(data_plane),
crates/blit-core/src/transfer_session/mod.rs-1880-            tx,
crates/blit-core/src/transfer_session/mod.rs-1881-            pending_resize,
crates/blit-core/src/transfer_session/mod.rs-1882-        )
crates/blit-core/src/transfer_session/mod.rs-1883-        .await?;
crates/blit-core/src/transfer_session/mod.rs-1884-    }
crates/blit-core/src/transfer_session/mod.rs-1885-    Ok(())
crates/blit-core/src/transfer_session/mod.rs-1886-}
crates/blit-core/src/transfer_session/mod.rs-1887-
crates/blit-core/src/transfer_session/mod.rs-1888-/// Block for the ack of the one in-flight resize and dial its socket (or
crates/blit-core/src/transfer_session/mod.rs:1889:/// settle it refused). Does NOT propose further — it resolves exactly the
crates/blit-core/src/transfer_session/mod.rs-1890-/// pending proposal so the destination's armed slot is consumed before we
crates/blit-core/src/transfer_session/mod.rs-1891-/// finish the data plane.
crates/blit-core/src/transfer_session/mod.rs-1892-async fn resolve_in_flight_resize(
crates/blit-core/src/transfer_session/mod.rs-1893-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs-1894-    dp: &data_plane::SourceDataPlane,
crates/blit-core/src/transfer_session/mod.rs-1895-    pending: data_plane::PendingResize,
crates/blit-core/src/transfer_session/mod.rs-1896-) -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-1897-    loop {
crates/blit-core/src/transfer_session/mod.rs-1898-        match events.recv().await {
crates/blit-core/src/transfer_session/mod.rs:1899:            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
crates/blit-core/src/transfer_session/mod.rs-1900-                if ack.accepted {
crates/blit-core/src/transfer_session/mod.rs-1901-                    dp.add_stream(&pending.sub_token).await?;
crates/blit-core/src/transfer_session/mod.rs-1902-                    dp.dial()
crates/blit-core/src/transfer_session/mod.rs:1903:                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
crates/blit-core/src/transfer_session/mod.rs-1904-                } else {
crates/blit-core/src/transfer_session/mod.rs-1905-                    dp.dial()
crates/blit-core/src/transfer_session/mod.rs:1906:                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
crates/blit-core/src/transfer_session/mod.rs-1907-                }
crates/blit-core/src/transfer_session/mod.rs-1908-                return Ok(());
crates/blit-core/src/transfer_session/mod.rs-1909-            }
crates/blit-core/src/transfer_session/mod.rs-1910-            // A stale ack for an already-settled epoch: ignore, keep
crates/blit-core/src/transfer_session/mod.rs-1911-            // waiting for ours.
crates/blit-core/src/transfer_session/mod.rs:1912:            Some(SourceEvent::ResizeAck(_)) => continue,
crates/blit-core/src/transfer_session/mod.rs-1913-            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
crates/blit-core/src/transfer_session/mod.rs-1914-            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs-1915-                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1916-                    format!("need for '{}' after NeedComplete", h.relative_path),
crates/blit-core/src/transfer_session/mod.rs-1917-                )))
crates/blit-core/src/transfer_session/mod.rs-1918-            }
crates/blit-core/src/transfer_session/mod.rs-1919-            Some(SourceEvent::BlockHashes(l)) => {
crates/blit-core/src/transfer_session/mod.rs-1920-                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1921-                    format!(
crates/blit-core/src/transfer_session/mod.rs-1922-                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
crates/blit-core/src/transfer_session/mod.rs-1923-                        l.relative_path
crates/blit-core/src/transfer_session/mod.rs-1924-                    ),
crates/blit-core/src/transfer_session/mod.rs-1925-                )))
crates/blit-core/src/transfer_session/mod.rs-1926-            }
crates/blit-core/src/transfer_session/mod.rs-1927-            Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs-1928-                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs-1929-                    "duplicate NeedComplete",
crates/blit-core/src/transfer_session/mod.rs-1930-                )))
--
crates/blit-core/src/transfer_session/mod.rs-1935-                )))
crates/blit-core/src/transfer_session/mod.rs-1936-            }
crates/blit-core/src/transfer_session/mod.rs-1937-            None => {
crates/blit-core/src/transfer_session/mod.rs-1938-                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs-1939-                    "source receive half ended with a resize in flight",
crates/blit-core/src/transfer_session/mod.rs-1940-                )))
crates/blit-core/src/transfer_session/mod.rs-1941-            }
crates/blit-core/src/transfer_session/mod.rs-1942-        }
crates/blit-core/src/transfer_session/mod.rs-1943-    }
crates/blit-core/src/transfer_session/mod.rs-1944-}
crates/blit-core/src/transfer_session/mod.rs-1945-
crates/blit-core/src/transfer_session/mod.rs-1946-/// Await the next terminal signal the receive half forwards while the
crates/blit-core/src/transfer_session/mod.rs-1947-/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
crates/blit-core/src/transfer_session/mod.rs-1948-/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
crates/blit-core/src/transfer_session/mod.rs-1949-/// abort the send and surface as the fault.
crates/blit-core/src/transfer_session/mod.rs-1950-///
crates/blit-core/src/transfer_session/mod.rs-1951-/// The drain runs after `resolve_in_flight_resize` and before `SourceDone`
crates/blit-core/src/transfer_session/mod.rs-1952-/// goes out, so the event channel is drained and the peer sends nothing
crates/blit-core/src/transfer_session/mod.rs:1953:/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
crates/blit-core/src/transfer_session/mod.rs-1954-/// or `Summary` is legitimate here. So a `Fault` is returned as-is and any
crates/blit-core/src/transfer_session/mod.rs-1955-/// OTHER event is surfaced as a protocol violation rather than silently
crates/blit-core/src/transfer_session/mod.rs-1956-/// dropped (codex otp-4b-3 F3): dropping it would defer or lose a
crates/blit-core/src/transfer_session/mod.rs-1957-/// fail-fast error and, if the drain is itself stuck, hang. Parks forever
crates/blit-core/src/transfer_session/mod.rs-1958-/// once the channel closes with no event so the data-plane future it
crates/blit-core/src/transfer_session/mod.rs-1959-/// races decides the outcome instead.
crates/blit-core/src/transfer_session/mod.rs-1960-async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs-1961-    match events.recv().await {
crates/blit-core/src/transfer_session/mod.rs-1962-        Some(SourceEvent::Fault(fault)) => fault,
crates/blit-core/src/transfer_session/mod.rs-1963-        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
crates/blit-core/src/transfer_session/mod.rs-1964-            SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs-1965-                "need for '{}' during the data-plane drain (after NeedComplete)",
crates/blit-core/src/transfer_session/mod.rs-1966-                h.relative_path
crates/blit-core/src/transfer_session/mod.rs-1967-            ))
crates/blit-core/src/transfer_session/mod.rs-1968-        }
crates/blit-core/src/transfer_session/mod.rs-1969-        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs-1970-            "BlockHashList for '{}' during the data-plane drain",
crates/blit-core/src/transfer_session/mod.rs-1971-            l.relative_path
crates/blit-core/src/transfer_session/mod.rs-1972-        )),
crates/blit-core/src/transfer_session/mod.rs-1973-        Some(SourceEvent::NeedComplete) => {
crates/blit-core/src/transfer_session/mod.rs-1974-            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
crates/blit-core/src/transfer_session/mod.rs-1975-        }
crates/blit-core/src/transfer_session/mod.rs:1976:        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1977:            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
crates/blit-core/src/transfer_session/mod.rs-1978-        ),
crates/blit-core/src/transfer_session/mod.rs-1979-        Some(SourceEvent::Summary(_)) => {
crates/blit-core/src/transfer_session/mod.rs-1980-            SessionFault::protocol_violation("TransferSummary before SourceDone")
crates/blit-core/src/transfer_session/mod.rs-1981-        }
crates/blit-core/src/transfer_session/mod.rs-1982-        None => std::future::pending().await,
crates/blit-core/src/transfer_session/mod.rs-1983-    }
crates/blit-core/src/transfer_session/mod.rs-1984-}
crates/blit-core/src/transfer_session/mod.rs-1985-
crates/blit-core/src/transfer_session/mod.rs-1986-/// A data-plane operation (`queue`/`finish`) failed mid-transfer. The
crates/blit-core/src/transfer_session/mod.rs-1987-/// break is usually the *symptom* of a peer abort — within
crates/blit-core/src/transfer_session/mod.rs-1988-/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
crates/blit-core/src/transfer_session/mod.rs-1989-/// its receive workers) always frames the real reason on the control
crates/blit-core/src/transfer_session/mod.rs-1990-/// lane. Prefer that framed fault; fall back to the raw data-plane error
crates/blit-core/src/transfer_session/mod.rs-1991-/// if the channel closes first or none arrives in that window.
crates/blit-core/src/transfer_session/mod.rs-1992-///
crates/blit-core/src/transfer_session/mod.rs-1993-/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
crates/blit-core/src/transfer_session/mod.rs-1994-/// fast on any stray event), this is called from BOTH error sites,
crates/blit-core/src/transfer_session/mod.rs-1995-/// including the `queue()` error inside the payload loop — where a
crates/blit-core/src/transfer_session/mod.rs:1996:/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
crates/blit-core/src/transfer_session/mod.rs-1997-/// ahead of the peer's `SessionError` (codex otp-4b-3 pass-2 F1). So it
crates/blit-core/src/transfer_session/mod.rs-1998-/// SKIPS non-fault events rather than treating them as violations: we are
crates/blit-core/src/transfer_session/mod.rs-1999-/// already unwinding on a data-plane error, and the framed fault (or the
crates/blit-core/src/transfer_session/mod.rs-2000-/// dp error) is the correct outcome, never a spurious protocol violation.
crates/blit-core/src/transfer_session/mod.rs-2001-async fn prefer_peer_fault(
crates/blit-core/src/transfer_session/mod.rs-2002-    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
crates/blit-core/src/transfer_session/mod.rs-2003-    dp_err: eyre::Report,
crates/blit-core/src/transfer_session/mod.rs-2004-) -> eyre::Report {
crates/blit-core/src/transfer_session/mod.rs-2005-    let framed = async {
crates/blit-core/src/transfer_session/mod.rs-2006-        loop {
crates/blit-core/src/transfer_session/mod.rs-2007-            match events.recv().await {
crates/blit-core/src/transfer_session/mod.rs-2008-                Some(SourceEvent::Fault(fault)) => break Some(fault),
crates/blit-core/src/transfer_session/mod.rs-2009-                // Skip a still-in-flight need/ack/complete: on this error
crates/blit-core/src/transfer_session/mod.rs-2010-                // path the transfer is aborting, so the framed reason (or
crates/blit-core/src/transfer_session/mod.rs-2011-                // the dp error) wins, not a stray-event violation.
crates/blit-core/src/transfer_session/mod.rs-2012-                Some(_) => continue,
crates/blit-core/src/transfer_session/mod.rs-2013-                // Receive half ended without framing a fault → the raw
crates/blit-core/src/transfer_session/mod.rs-2014-                // data-plane error is the best available cause.
--
crates/blit-core/src/transfer_session/mod.rs-2573-    // at the next filesystem op rather than running to completion
crates/blit-core/src/transfer_session/mod.rs-2574-    // behind a job already recorded cancelled.
crates/blit-core/src/transfer_session/mod.rs-2575-    let check_abort = || -> Result<()> {
crates/blit-core/src/transfer_session/mod.rs-2576-        if abort.load(Ordering::Acquire) {
crates/blit-core/src/transfer_session/mod.rs-2577-            return Err(eyre::eyre!("mirror delete pass aborted: session cancelled"));
crates/blit-core/src/transfer_session/mod.rs-2578-        }
crates/blit-core/src/transfer_session/mod.rs-2579-        Ok(())
crates/blit-core/src/transfer_session/mod.rs-2580-    };
crates/blit-core/src/transfer_session/mod.rs-2581-
crates/blit-core/src/transfer_session/mod.rs-2582-    let mut deleted_files = 0u64;
crates/blit-core/src/transfer_session/mod.rs-2583-    let mut deleted_dirs = 0u64;
crates/blit-core/src/transfer_session/mod.rs-2584-    for file in &plan.files {
crates/blit-core/src/transfer_session/mod.rs-2585-        check_abort()?;
crates/blit-core/src/transfer_session/mod.rs-2586-        contained(file)?;
crates/blit-core/src/transfer_session/mod.rs-2587-        if !execute {
crates/blit-core/src/transfer_session/mod.rs-2588-            deleted_files += 1;
crates/blit-core/src/transfer_session/mod.rs-2589-            continue;
crates/blit-core/src/transfer_session/mod.rs-2590-        }
crates/blit-core/src/transfer_session/mod.rs:2591:        // Windows refuses to delete a read-only file; clear the attribute
crates/blit-core/src/transfer_session/mod.rs-2592-        // first, matching the daemon purge (admin.rs) and local mirror
crates/blit-core/src/transfer_session/mod.rs-2593-        // (engine/mirror.rs) executors (codex otp-6b F2).
crates/blit-core/src/transfer_session/mod.rs-2594-        #[cfg(windows)]
crates/blit-core/src/transfer_session/mod.rs-2595-        crate::win_fs::clear_readonly_recursive(file);
crates/blit-core/src/transfer_session/mod.rs-2596-        match std::fs::remove_file(file) {
crates/blit-core/src/transfer_session/mod.rs-2597-            Ok(()) => deleted_files += 1,
crates/blit-core/src/transfer_session/mod.rs-2598-            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
crates/blit-core/src/transfer_session/mod.rs-2599-            Err(e) => return Err(eyre::eyre!("mirror delete {}: {e}", file.display())),
crates/blit-core/src/transfer_session/mod.rs-2600-        }
crates/blit-core/src/transfer_session/mod.rs-2601-    }
crates/blit-core/src/transfer_session/mod.rs-2602-    for dir in &plan.dirs {
crates/blit-core/src/transfer_session/mod.rs-2603-        check_abort()?;
crates/blit-core/src/transfer_session/mod.rs-2604-        contained(dir)?;
crates/blit-core/src/transfer_session/mod.rs-2605-        if !execute {
crates/blit-core/src/transfer_session/mod.rs-2606-            deleted_dirs += 1;
crates/blit-core/src/transfer_session/mod.rs-2607-            continue;
crates/blit-core/src/transfer_session/mod.rs-2608-        }
crates/blit-core/src/transfer_session/mod.rs-2609-        #[cfg(windows)]
--
crates/blit-core/src/transfer_session/mod.rs-2917-                            progress.as_ref(),
crates/blit-core/src/transfer_session/mod.rs-2918-                        )
crates/blit-core/src/transfer_session/mod.rs-2919-                        .await?;
crates/blit-core/src/transfer_session/mod.rs-2920-                    }
crates/blit-core/src/transfer_session/mod.rs-2921-                }
crates/blit-core/src/transfer_session/mod.rs-2922-            }
crates/blit-core/src/transfer_session/mod.rs-2923-            Some(Frame::ManifestComplete(complete)) => {
crates/blit-core/src/transfer_session/mod.rs-2924-                if manifest_complete {
crates/blit-core/src/transfer_session/mod.rs-2925-                    return Err(violation("duplicate ManifestComplete".into()));
crates/blit-core/src/transfer_session/mod.rs-2926-                }
crates/blit-core/src/transfer_session/mod.rs-2927-                // otp-6b: mirror deletions are data-loss-dangerous when the
crates/blit-core/src/transfer_session/mod.rs-2928-                // source scan was incomplete — a source file missing from an
crates/blit-core/src/transfer_session/mod.rs-2929-                // aborted scan would be misclassified extraneous and deleted
crates/blit-core/src/transfer_session/mod.rs-2930-                // at the dest. Refuse here (before any transfer or deletion)
crates/blit-core/src/transfer_session/mod.rs-2931-                // rather than partial-mirror. Matches the old paths'
crates/blit-core/src/transfer_session/mod.rs-2932-                // require-complete-scan guard.
crates/blit-core/src/transfer_session/mod.rs-2933-                if mirror_enabled && !complete.scan_complete {
crates/blit-core/src/transfer_session/mod.rs-2934-                    return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:2935:                        "mirror refused: the source scan did not complete \
crates/blit-core/src/transfer_session/mod.rs-2936-                         (unreadable paths) — deleting now could remove files \
crates/blit-core/src/transfer_session/mod.rs-2937-                         the source still has",
crates/blit-core/src/transfer_session/mod.rs-2938-                    )));
crates/blit-core/src/transfer_session/mod.rs-2939-                }
crates/blit-core/src/transfer_session/mod.rs-2940-                // codex otp-9b F1 (R49-F2 on the session): an initiator
crates/blit-core/src/transfer_session/mod.rs-2941-                // that declared "the source will be deleted after this
crates/blit-core/src/transfer_session/mod.rs-2942-                // transfer" (`blit move`) must NOT get a success out of
crates/blit-core/src/transfer_session/mod.rs-2943-                // an incomplete source scan — files the scan could not
crates/blit-core/src/transfer_session/mod.rs-2944-                // read would be silently lost when the caller deletes
crates/blit-core/src/transfer_session/mod.rs-2945-                // the source. Same abort point as the mirror guard.
crates/blit-core/src/transfer_session/mod.rs-2946-                if negotiated.open.require_complete_scan && !complete.scan_complete {
crates/blit-core/src/transfer_session/mod.rs:2947:                    return Err(eyre::Report::new(SessionFault::refusal(
crates/blit-core/src/transfer_session/mod.rs-2948-                        session_error::Code::ScanIncomplete,
crates/blit-core/src/transfer_session/mod.rs:2949:                        "transfer refused: the source scan did not complete \
crates/blit-core/src/transfer_session/mod.rs-2950-                         (unreadable paths) and the operation requires a \
crates/blit-core/src/transfer_session/mod.rs-2951-                         complete scan (move deletes the source afterwards)",
crates/blit-core/src/transfer_session/mod.rs-2952-                    )));
crates/blit-core/src/transfer_session/mod.rs-2953-                }
crates/blit-core/src/transfer_session/mod.rs-2954-                let chunk = std::mem::take(&mut pending);
crates/blit-core/src/transfer_session/mod.rs-2955-                if let Some(la) = &local_apply {
crates/blit-core/src/transfer_session/mod.rs-2956-                    diff_chunk_and_apply_local(
crates/blit-core/src/transfer_session/mod.rs-2957-                        la,
crates/blit-core/src/transfer_session/mod.rs-2958-                        &mut local_run,
crates/blit-core/src/transfer_session/mod.rs-2959-                        chunk,
crates/blit-core/src/transfer_session/mod.rs-2960-                        dst_root,
crates/blit-core/src/transfer_session/mod.rs-2961-                        canonical_dst_root.as_deref(),
crates/blit-core/src/transfer_session/mod.rs-2962-                        &compare_opts,
crates/blit-core/src/transfer_session/mod.rs-2963-                        &mut granted,
crates/blit-core/src/transfer_session/mod.rs-2964-                        &mut needed_paths,
crates/blit-core/src/transfer_session/mod.rs-2965-                        progress.as_ref(),
crates/blit-core/src/transfer_session/mod.rs-2966-                    )
crates/blit-core/src/transfer_session/mod.rs-2967-                    .await?;
--
crates/blit-core/src/transfer_session/mod.rs-3190-                        }
crates/blit-core/src/transfer_session/mod.rs-3191-                        data_plane::DestRecvPlane::Initiator(run) => {
crates/blit-core/src/transfer_session/mod.rs-3192-                            run.add_dialed_stream(&resize.sub_token).await?;
crates/blit-core/src/transfer_session/mod.rs-3193-                            true
crates/blit-core/src/transfer_session/mod.rs-3194-                        }
crates/blit-core/src/transfer_session/mod.rs-3195-                    }
crates/blit-core/src/transfer_session/mod.rs-3196-                } else {
crates/blit-core/src/transfer_session/mod.rs-3197-                    false
crates/blit-core/src/transfer_session/mod.rs-3198-                };
crates/blit-core/src/transfer_session/mod.rs-3199-                if accepted {
crates/blit-core/src/transfer_session/mod.rs-3200-                    resize_live += 1;
crates/blit-core/src/transfer_session/mod.rs-3201-                }
crates/blit-core/src/transfer_session/mod.rs-3202-                let effective = if accepted {
crates/blit-core/src/transfer_session/mod.rs-3203-                    resize.target_stream_count
crates/blit-core/src/transfer_session/mod.rs-3204-                } else {
crates/blit-core/src/transfer_session/mod.rs-3205-                    resize_live as u32
crates/blit-core/src/transfer_session/mod.rs-3206-                };
crates/blit-core/src/transfer_session/mod.rs-3207-                transport
crates/blit-core/src/transfer_session/mod.rs:3208:                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
crates/blit-core/src/transfer_session/mod.rs-3209-                        epoch: resize.epoch,
crates/blit-core/src/transfer_session/mod.rs-3210-                        effective_stream_count: effective,
crates/blit-core/src/transfer_session/mod.rs-3211-                        accepted,
crates/blit-core/src/transfer_session/mod.rs-3212-                    })))
crates/blit-core/src/transfer_session/mod.rs-3213-                    .await?;
crates/blit-core/src/transfer_session/mod.rs-3214-            }
crates/blit-core/src/transfer_session/mod.rs-3215-            Some(Frame::SourceDone(_)) => {
crates/blit-core/src/transfer_session/mod.rs-3216-                if !manifest_complete {
crates/blit-core/src/transfer_session/mod.rs-3217-                    return Err(violation("SourceDone before ManifestComplete".into()));
crates/blit-core/src/transfer_session/mod.rs-3218-                }
crates/blit-core/src/transfer_session/mod.rs-3219-                // Completion, both carriers: the shared `outstanding`
crates/blit-core/src/transfer_session/mod.rs-3220-                // set must be empty (every granted need claimed exactly
crates/blit-core/src/transfer_session/mod.rs-3221-                // once). In-stream claims inline above; the data-plane
crates/blit-core/src/transfer_session/mod.rs-3222-                // NeedListSink claims as payloads land, so joining the
crates/blit-core/src/transfer_session/mod.rs-3223-                // receive task first drains the last of them (and
crates/blit-core/src/transfer_session/mod.rs-3224-                // surfaces any receive error / stall). Set membership —
crates/blit-core/src/transfer_session/mod.rs-3225-                // not a file count — is the contract (codex F1: a count
crates/blit-core/src/transfer_session/mod.rs-3226-                // proxy let a peer substitute or duplicate paths).
crates/blit-core/src/transfer_session/mod.rs-3227-                // `finish()` drops the arm sender (no more resizes), joins
crates/blit-core/src/transfer_session/mod.rs-3228-                // the accept loop, and reports the settled stream count.
crates/blit-core/src/transfer_session/mod.rs-3229-                //
crates/blit-core/src/transfer_session/mod.rs-3230-                // otp-11: the LOCAL carrier joins its apply pipeline with
crates/blit-core/src/transfer_session/mod.rs-3231-                // the same discipline (drain every write, surface its
crates/blit-core/src/transfer_session/mod.rs-3232-                // error) and takes the write totals as this end's
crates/blit-core/src/transfer_session/mod.rs-3233-                // counters — the scorer stays the destination.
crates/blit-core/src/transfer_session/mod.rs-3234-                if let Some(run) = local_run.take() {
crates/blit-core/src/transfer_session/mod.rs-3235-                    let totals = run.finish().await?;
crates/blit-core/src/transfer_session/mod.rs-3236-                    files_written = totals.files_written as u64;
crates/blit-core/src/transfer_session/mod.rs-3237-                    bytes_written = totals.bytes_written;
crates/blit-core/src/transfer_session/mod.rs-3238-                }
crates/blit-core/src/transfer_session/mod.rs-3239-                // R46-F2 on the local carrier (codex otp-11a F4): the
crates/blit-core/src/transfer_session/mod.rs-3240-                // scan-complete guard fired at ManifestComplete, but the
crates/blit-core/src/transfer_session/mod.rs-3241-                // local apply's availability checks can record
crates/blit-core/src/transfer_session/mod.rs-3242-                // unreadables AFTER it (a file vanishing or losing
crates/blit-core/src/transfer_session/mod.rs-3243-                // permissions between enumeration and apply). The old
crates/blit-core/src/transfer_session/mod.rs:3244:                // engine refused mirror deletions on ANY unreadable
crates/blit-core/src/transfer_session/mod.rs-3245-                // entry; carry that exact posture — checked here, after
crates/blit-core/src/transfer_session/mod.rs-3246-                // the apply pipeline joined, before any deletion.
crates/blit-core/src/transfer_session/mod.rs-3247-                if mirror_enabled {
crates/blit-core/src/transfer_session/mod.rs-3248-                    if let Some(la) = &local_apply {
crates/blit-core/src/transfer_session/mod.rs-3249-                        let unreadable_count = la.unreadable.lock().map(|g| g.len()).unwrap_or(0);
crates/blit-core/src/transfer_session/mod.rs-3250-                        if unreadable_count != 0 {
crates/blit-core/src/transfer_session/mod.rs-3251-                            return Err(eyre::Report::new(SessionFault::internal(format!(
crates/blit-core/src/transfer_session/mod.rs:3252:                                "mirror refused: {unreadable_count} source entr{} could \
crates/blit-core/src/transfer_session/mod.rs-3253-                                 not be read during the transfer — deleting now could \
crates/blit-core/src/transfer_session/mod.rs-3254-                                 remove files the source still has",
crates/blit-core/src/transfer_session/mod.rs-3255-                                if unreadable_count == 1 { "y" } else { "ies" }
crates/blit-core/src/transfer_session/mod.rs-3256-                            ))));
crates/blit-core/src/transfer_session/mod.rs-3257-                        }
crates/blit-core/src/transfer_session/mod.rs-3258-                    }
crates/blit-core/src/transfer_session/mod.rs-3259-                }
crates/blit-core/src/transfer_session/mod.rs-3260-                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
crates/blit-core/src/transfer_session/mod.rs-3261-                    Some(run) => {
crates/blit-core/src/transfer_session/mod.rs-3262-                        let totals = run.finish().await?;
crates/blit-core/src/transfer_session/mod.rs-3263-                        files_written = totals.outcome.files_written as u64;
crates/blit-core/src/transfer_session/mod.rs-3264-                        bytes_written = totals.outcome.bytes_written;
crates/blit-core/src/transfer_session/mod.rs-3265-                        (false, Some(totals.streams))
crates/blit-core/src/transfer_session/mod.rs-3266-                    }
crates/blit-core/src/transfer_session/mod.rs-3267-                    None => (true, None),
crates/blit-core/src/transfer_session/mod.rs-3268-                };
crates/blit-core/src/transfer_session/mod.rs-3269-                let unfulfilled = outstanding
crates/blit-core/src/transfer_session/mod.rs-3270-                    .lock()
--
crates/blit-core/src/transfer_session/mod.rs-3389-                    files_transferred: files_written,
crates/blit-core/src/transfer_session/mod.rs-3390-                    bytes_transferred: bytes_written,
crates/blit-core/src/transfer_session/mod.rs-3391-                    entries_deleted,
crates/blit-core/src/transfer_session/mod.rs-3392-                    in_stream_carrier_used,
crates/blit-core/src/transfer_session/mod.rs-3393-                    files_resumed: files_resumed.load(Ordering::Relaxed),
crates/blit-core/src/transfer_session/mod.rs-3394-                };
crates/blit-core/src/transfer_session/mod.rs-3395-                transport.send(frame(Frame::Summary(summary))).await?;
crates/blit-core/src/transfer_session/mod.rs-3396-                return Ok(DestinationOutcome {
crates/blit-core/src/transfer_session/mod.rs-3397-                    summary,
crates/blit-core/src/transfer_session/mod.rs-3398-                    needed_paths,
crates/blit-core/src/transfer_session/mod.rs-3399-                    data_plane_streams,
crates/blit-core/src/transfer_session/mod.rs-3400-                });
crates/blit-core/src/transfer_session/mod.rs-3401-            }
crates/blit-core/src/transfer_session/mod.rs-3402-            Some(Frame::Error(err)) => {
crates/blit-core/src/transfer_session/mod.rs-3403-                return Err(eyre::Report::new(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs-3404-            }
crates/blit-core/src/transfer_session/mod.rs-3405-            other => {
crates/blit-core/src/transfer_session/mod.rs-3406-                // Everything else is off-lane or off-phase here:
crates/blit-core/src/transfer_session/mod.rs:3407:                // destination-lane frames echoed back (a ResizeAck or
crates/blit-core/src/transfer_session/mod.rs-3408-                // BlockHashList the destination would never receive),
crates/blit-core/src/transfer_session/mod.rs-3409-                // stray handshake frames, bare FileData/TarShardChunk
crates/blit-core/src/transfer_session/mod.rs-3410-                // outside a record. Fail fast, no tolerant parsing.
crates/blit-core/src/transfer_session/mod.rs-3411-                return Err(violation(format!(
crates/blit-core/src/transfer_session/mod.rs-3412-                    "{} not valid on the destination's receive lane in this phase",
crates/blit-core/src/transfer_session/mod.rs-3413-                    frame_name(&other)
crates/blit-core/src/transfer_session/mod.rs-3414-                )));
crates/blit-core/src/transfer_session/mod.rs-3415-            }
crates/blit-core/src/transfer_session/mod.rs-3416-        }
crates/blit-core/src/transfer_session/mod.rs-3417-    }
crates/blit-core/src/transfer_session/mod.rs-3418-}
crates/blit-core/src/transfer_session/mod.rs-3419-
crates/blit-core/src/transfer_session/mod.rs-3420-/// The LOCAL carrier's twin of [`diff_chunk_and_send_needs`] (otp-11):
crates/blit-core/src/transfer_session/mod.rs-3421-/// identical per-entry verdicts (the same [`destination_needs`] compare,
crates/blit-core/src/transfer_session/mod.rs-3422-/// the same `granted` dedup, the same `needed_paths` record), but the
crates/blit-core/src/transfer_session/mod.rs-3423-/// needed headers are planned into payloads and queued onto the
crates/blit-core/src/transfer_session/mod.rs-3424-/// in-process apply pipeline instead of being granted to the source —
crates/blit-core/src/transfer_session/mod.rs-3425-/// no frame is sent and nothing enters `outstanding`. Resume is
--
crates/blit-core/src/transfer_session/mod.rs-4160-                return Err(violation(format!(
crates/blit-core/src/transfer_session/mod.rs-4161-                    "{} inside tar shard record",
crates/blit-core/src/transfer_session/mod.rs-4162-                    frame_name(&other)
crates/blit-core/src/transfer_session/mod.rs-4163-                )));
crates/blit-core/src/transfer_session/mod.rs-4164-            }
crates/blit-core/src/transfer_session/mod.rs-4165-        }
crates/blit-core/src/transfer_session/mod.rs-4166-    }
crates/blit-core/src/transfer_session/mod.rs-4167-}
crates/blit-core/src/transfer_session/mod.rs-4168-
crates/blit-core/src/transfer_session/mod.rs-4169-#[cfg(test)]
crates/blit-core/src/transfer_session/mod.rs-4170-mod tests {
crates/blit-core/src/transfer_session/mod.rs-4171-    use super::*;
crates/blit-core/src/transfer_session/mod.rs-4172-
crates/blit-core/src/transfer_session/mod.rs-4173-    /// otp-10c-2 codex F4: the mirror delete pass containment-checks
crates/blit-core/src/transfer_session/mod.rs-4174-    /// every planned target against the canonical destination root
crates/blit-core/src/transfer_session/mod.rs-4175-    /// before any filesystem op. The wiring was unpinned (a mutation
crates/blit-core/src/transfer_session/mod.rs-4176-    /// deleting the `contained(...)` call survived the suite): with a
crates/blit-core/src/transfer_session/mod.rs-4177-    /// canonical root that does NOT contain the destination, the pass
crates/blit-core/src/transfer_session/mod.rs:4178:    /// must refuse before deleting anything — and with the real root
crates/blit-core/src/transfer_session/mod.rs-4179-    /// it deletes normally (the control arm, so this can't pass
crates/blit-core/src/transfer_session/mod.rs-4180-    /// vacuously).
crates/blit-core/src/transfer_session/mod.rs-4181-    #[test]
crates/blit-core/src/transfer_session/mod.rs-4182-    fn mirror_delete_pass_containment_check_gates_every_deletion() {
crates/blit-core/src/transfer_session/mod.rs-4183-        let tmp = tempfile::tempdir().unwrap();
crates/blit-core/src/transfer_session/mod.rs-4184-        let dst = tmp.path().join("dst");
crates/blit-core/src/transfer_session/mod.rs-4185-        std::fs::create_dir_all(&dst).unwrap();
crates/blit-core/src/transfer_session/mod.rs-4186-        std::fs::write(dst.join("extraneous.txt"), b"x").unwrap();
crates/blit-core/src/transfer_session/mod.rs-4187-        let elsewhere = tmp.path().join("elsewhere");
crates/blit-core/src/transfer_session/mod.rs-4188-        std::fs::create_dir_all(&elsewhere).unwrap();
crates/blit-core/src/transfer_session/mod.rs-4189-        let elsewhere = elsewhere.canonicalize().unwrap();
crates/blit-core/src/transfer_session/mod.rs-4190-
crates/blit-core/src/transfer_session/mod.rs-4191-        let source_files: HashSet<String> = HashSet::new(); // everything is extraneous
crates/blit-core/src/transfer_session/mod.rs-4192-        let filter = crate::fs_enum::FileFilter::default();
crates/blit-core/src/transfer_session/mod.rs-4193-        let abort = AtomicBool::new(false);
crates/blit-core/src/transfer_session/mod.rs-4194-
crates/blit-core/src/transfer_session/mod.rs:4195:        // Foreign canonical root → the containment check must refuse
crates/blit-core/src/transfer_session/mod.rs-4196-        // the deletion and leave the file alone.
crates/blit-core/src/transfer_session/mod.rs-4197-        let err = mirror_delete_pass(
crates/blit-core/src/transfer_session/mod.rs-4198-            &dst,
crates/blit-core/src/transfer_session/mod.rs-4199-            &source_files,
crates/blit-core/src/transfer_session/mod.rs-4200-            &filter,
crates/blit-core/src/transfer_session/mod.rs-4201-            false,
crates/blit-core/src/transfer_session/mod.rs-4202-            Some(&elsewhere),
crates/blit-core/src/transfer_session/mod.rs-4203-            &abort,
crates/blit-core/src/transfer_session/mod.rs-4204-            true,
crates/blit-core/src/transfer_session/mod.rs-4205-        )
crates/blit-core/src/transfer_session/mod.rs:4206:        .expect_err("a target outside the canonical root must refuse");
crates/blit-core/src/transfer_session/mod.rs-4207-        assert!(
crates/blit-core/src/transfer_session/mod.rs-4208-            format!("{err:#}").contains("mirror delete containment"),
crates/blit-core/src/transfer_session/mod.rs-4209-            "got: {err:#}"
crates/blit-core/src/transfer_session/mod.rs-4210-        );
crates/blit-core/src/transfer_session/mod.rs-4211-        assert!(
crates/blit-core/src/transfer_session/mod.rs-4212-            dst.join("extraneous.txt").exists(),
crates/blit-core/src/transfer_session/mod.rs:4213:            "nothing may be deleted once containment refuses"
crates/blit-core/src/transfer_session/mod.rs-4214-        );
crates/blit-core/src/transfer_session/mod.rs-4215-
crates/blit-core/src/transfer_session/mod.rs-4216-        // Control: the real canonical root deletes the extraneous file.
crates/blit-core/src/transfer_session/mod.rs-4217-        let real_root = crate::path_safety::canonical_dest_root(&dst).unwrap();
crates/blit-core/src/transfer_session/mod.rs-4218-        let deleted = mirror_delete_pass(
crates/blit-core/src/transfer_session/mod.rs-4219-            &dst,
crates/blit-core/src/transfer_session/mod.rs-4220-            &source_files,
crates/blit-core/src/transfer_session/mod.rs-4221-            &filter,
crates/blit-core/src/transfer_session/mod.rs-4222-            false,
crates/blit-core/src/transfer_session/mod.rs-4223-            Some(&real_root),
crates/blit-core/src/transfer_session/mod.rs-4224-            &abort,
crates/blit-core/src/transfer_session/mod.rs-4225-            true,
crates/blit-core/src/transfer_session/mod.rs-4226-        )
crates/blit-core/src/transfer_session/mod.rs-4227-        .expect("in-root deletion proceeds");
crates/blit-core/src/transfer_session/mod.rs-4228-        assert_eq!(deleted, (1, 0));
crates/blit-core/src/transfer_session/mod.rs-4229-        assert!(!dst.join("extraneous.txt").exists());
crates/blit-core/src/transfer_session/mod.rs-4230-    }
crates/blit-core/src/transfer_session/mod.rs-4231-
--
crates/blit-core/src/transfer_session/mod.rs-4317-    /// returns THAT fault, not the raw data-plane transport error — the
crates/blit-core/src/transfer_session/mod.rs-4318-    /// non-timeout half of the mid-transfer-cancel guard (the e2e in
crates/blit-core/src/transfer_session/mod.rs-4319-    /// `blit-daemon` guards the still-pending-drain half).
crates/blit-core/src/transfer_session/mod.rs-4320-    #[tokio::test]
crates/blit-core/src/transfer_session/mod.rs-4321-    async fn prefer_peer_fault_prefers_a_framed_fault() {
crates/blit-core/src/transfer_session/mod.rs-4322-        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs-4323-        // The peer framed CANCELLED on the control lane before we ask.
crates/blit-core/src/transfer_session/mod.rs-4324-        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs-4325-            code: session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4326-            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs-4327-            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4328-            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4329-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-4330-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-4331-            io_kind: None,
crates/blit-core/src/transfer_session/mod.rs-4332-        }))
crates/blit-core/src/transfer_session/mod.rs-4333-        .expect("send fault");
crates/blit-core/src/transfer_session/mod.rs-4334-
crates/blit-core/src/transfer_session/mod.rs:4335:        let dp_err = eyre::Report::new(SessionFault::refusal(
crates/blit-core/src/transfer_session/mod.rs-4336-            session_error::Code::DataPlaneFailed,
crates/blit-core/src/transfer_session/mod.rs-4337-            "Broken pipe (os error 32)",
crates/blit-core/src/transfer_session/mod.rs-4338-        ));
crates/blit-core/src/transfer_session/mod.rs-4339-        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
crates/blit-core/src/transfer_session/mod.rs-4340-        let fault = chosen
crates/blit-core/src/transfer_session/mod.rs-4341-            .downcast_ref::<SessionFault>()
crates/blit-core/src/transfer_session/mod.rs-4342-            .expect("a SessionFault");
crates/blit-core/src/transfer_session/mod.rs-4343-        assert_eq!(
crates/blit-core/src/transfer_session/mod.rs-4344-            fault.code,
crates/blit-core/src/transfer_session/mod.rs-4345-            session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4346-            "the framed CANCELLED must win over the data-plane break"
crates/blit-core/src/transfer_session/mod.rs-4347-        );
crates/blit-core/src/transfer_session/mod.rs-4348-    }
crates/blit-core/src/transfer_session/mod.rs-4349-
crates/blit-core/src/transfer_session/mod.rs-4350-    /// otp-4b-3 pass-2 F1: on the `queue()` error path (payload phase) a
crates/blit-core/src/transfer_session/mod.rs-4351-    /// legitimate `Need` may be queued ahead of the peer's `CANCELLED`.
crates/blit-core/src/transfer_session/mod.rs-4352-    /// `prefer_peer_fault` must SKIP it and still surface CANCELLED — not
crates/blit-core/src/transfer_session/mod.rs-4353-    /// mistake the in-flight need for a protocol violation (the strict
--
crates/blit-core/src/transfer_session/mod.rs-4357-        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
crates/blit-core/src/transfer_session/mod.rs-4358-        // A still-in-flight need queued before the abort frame.
crates/blit-core/src/transfer_session/mod.rs-4359-        tx.send(SourceEvent::Need(FileHeader {
crates/blit-core/src/transfer_session/mod.rs-4360-            relative_path: "still-needed.bin".into(),
crates/blit-core/src/transfer_session/mod.rs-4361-            ..Default::default()
crates/blit-core/src/transfer_session/mod.rs-4362-        }))
crates/blit-core/src/transfer_session/mod.rs-4363-        .expect("send need");
crates/blit-core/src/transfer_session/mod.rs-4364-        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs-4365-            code: session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4366-            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs-4367-            local_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4368-            peer_build_id: String::new(),
crates/blit-core/src/transfer_session/mod.rs-4369-            peer_notified: true,
crates/blit-core/src/transfer_session/mod.rs-4370-            relative_path: None,
crates/blit-core/src/transfer_session/mod.rs-4371-            io_kind: None,
crates/blit-core/src/transfer_session/mod.rs-4372-        }))
crates/blit-core/src/transfer_session/mod.rs-4373-        .expect("send fault");
crates/blit-core/src/transfer_session/mod.rs-4374-
crates/blit-core/src/transfer_session/mod.rs:4375:        let dp_err = eyre::Report::new(SessionFault::refusal(
crates/blit-core/src/transfer_session/mod.rs-4376-            session_error::Code::DataPlaneFailed,
crates/blit-core/src/transfer_session/mod.rs-4377-            "pipeline closed",
crates/blit-core/src/transfer_session/mod.rs-4378-        ));
crates/blit-core/src/transfer_session/mod.rs-4379-        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
crates/blit-core/src/transfer_session/mod.rs-4380-        let fault = chosen
crates/blit-core/src/transfer_session/mod.rs-4381-            .downcast_ref::<SessionFault>()
crates/blit-core/src/transfer_session/mod.rs-4382-            .expect("a SessionFault");
crates/blit-core/src/transfer_session/mod.rs-4383-        assert_eq!(
crates/blit-core/src/transfer_session/mod.rs-4384-            fault.code,
crates/blit-core/src/transfer_session/mod.rs-4385-            session_error::Code::Cancelled,
crates/blit-core/src/transfer_session/mod.rs-4386-            "an in-flight need must be skipped, not surfaced as a violation"
crates/blit-core/src/transfer_session/mod.rs-4387-        );
crates/blit-core/src/transfer_session/mod.rs-4388-    }
crates/blit-core/src/transfer_session/mod.rs-4389-
crates/blit-core/src/transfer_session/mod.rs-4390-    /// otp-7b-2 (D-2026-07-09-1 Q2 rider): the end-of-operation summary
crates/blit-core/src/transfer_session/mod.rs-4391-    /// names the affected file and suggests a re-run; a fault with no
crates/blit-core/src/transfer_session/mod.rs-4392-    /// file identity yields no summary block (nothing to converge on).
crates/blit-core/src/transfer_session/mod.rs-4393-    /// The path survives the wire round trip (`SessionError.relative_path`)

exec
/bin/zsh -lc "rg -n \"TRANSFER_STALL_TIMEOUT|DATA_PLANE_(ACCEPT|TOKEN|CONNECT)_TIMEOUT\" crates/blit-core/src -g '*.rs'; rg -n \"struct StallGuard|impl.*StallGuard\" crates/blit-core/src -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/mod.rs:41:pub use socket::{configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT};
crates/blit-core/src/remote/transfer/socket.rs:27:/// and [`DATA_PLANE_TOKEN_TIMEOUT`] — replacing three per-file
crates/blit-core/src/remote/transfer/socket.rs:35:pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/socket.rs:41:pub const DATA_PLANE_TOKEN_TIMEOUT: Duration = Duration::from_secs(15);
crates/blit-core/src/remote/transfer/socket.rs:109:/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
crates/blit-core/src/remote/transfer/socket.rs:116:/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
crates/blit-core/src/remote/transfer/socket.rs:132:        DATA_PLANE_ACCEPT_TIMEOUT,
crates/blit-core/src/remote/transfer/socket.rs:133:        DATA_PLANE_TOKEN_TIMEOUT,
crates/blit-core/src/remote/transfer/stall_guard.rs:27://!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
crates/blit-core/src/remote/transfer/stall_guard.rs:31://!   already bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` /
crates/blit-core/src/remote/transfer/stall_guard.rs:32://!   `DATA_PLANE_TOKEN_TIMEOUT` pair (`remote::transfer::socket`);
crates/blit-core/src/remote/transfer/stall_guard.rs:65:///   the shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
crates/blit-core/src/remote/transfer/stall_guard.rs:70:pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/stall_guard.rs:133:/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
crates/blit-core/src/remote/transfer/data_plane.rs:11:use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/remote/transfer/data_plane.rs:53:/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
crates/blit-core/src/remote/transfer/data_plane.rs:82:    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
crates/blit-core/src/remote/transfer/data_plane.rs:176:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
crates/blit-core/src/remote/transfer/sink.rs:844:                        crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT / 3,
crates/blit-core/src/transfer_session/mod.rs:51:use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
crates/blit-core/src/transfer_session/mod.rs:1988:/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
crates/blit-core/src/transfer_session/mod.rs:2019:    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, framed).await {
crates/blit-core/src/transfer_session/data_plane.rs:59:    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
crates/blit-core/src/transfer_session/data_plane.rs:62:use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/transfer_session/data_plane.rs:364:        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
crates/blit-core/src/transfer_session/data_plane.rs:373:    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
crates/blit-core/src/transfer_session/data_plane.rs:379:            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
crates/blit-core/src/transfer_session/data_plane.rs:394:    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
crates/blit-core/src/transfer_session/data_plane.rs:400:                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
crates/blit-core/src/transfer_session/data_plane.rs:428:    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
crates/blit-core/src/transfer_session/data_plane.rs:438:                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
crates/blit-core/src/remote/transfer/data_plane.rs:56:/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
crates/blit-core/src/remote/transfer/stall_guard.rs:76:pub struct StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:82:impl<R> StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:92:impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:140:pub struct StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:146:impl<W> StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:160:impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/stall_guard.rs | sed -n '60,138p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba crates/blit-core/tests/transfer_session_roles.rs | sed -n '1130,1405p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    60	/// - Session DESTINATION receive TCP via [`StallGuard`].
    61	/// - Daemon pull-data-plane **write progress after token acceptance**
    62	///   (`daemon::service::pull_sync` — audit-h3b; the `pull` service died
    63	///   at ue-r2-1h) via [`StallGuardWriter`] inside `DataPlaneSession`.
    64	///   The accept + token phases on those paths are separately bounded by
    65	///   the shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
    66	///   pair (`remote::transfer::socket`, w1-4).
    67	///
    68	/// The gRPC-fallback paths sit below `tonic::Streaming<T>` rather than
    69	/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
    70	pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
    71	
    72	/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
    73	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on
    74	/// every read that returns (data or clean EOF), so it is a per-gap idle
    75	/// timeout, not a cap on the whole transfer.
    76	pub struct StallGuard<R> {
    77	    inner: R,
    78	    timeout: Duration,
    79	    deadline: Pin<Box<Sleep>>,
    80	}
    81	
    82	impl<R> StallGuard<R> {
    83	    pub fn new(inner: R, timeout: Duration) -> Self {
    84	        Self {
    85	            inner,
    86	            timeout,
    87	            deadline: Box::pin(tokio::time::sleep(timeout)),
    88	        }
    89	    }
    90	}
    91	
    92	impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
    93	    fn poll_read(
    94	        self: Pin<&mut Self>,
    95	        cx: &mut Context<'_>,
    96	        buf: &mut ReadBuf<'_>,
    97	    ) -> Poll<io::Result<()>> {
    98	        let this = self.get_mut();
    99	        match Pin::new(&mut this.inner).poll_read(cx, buf) {
   100	            Poll::Ready(Ok(())) => {
   101	                // The read completed (delivered bytes, or a clean EOF) —
   102	                // that's progress, so re-arm the idle deadline.
   103	                this.deadline.as_mut().reset(Instant::now() + this.timeout);
   104	                Poll::Ready(Ok(()))
   105	            }
   106	            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
   107	            Poll::Pending => {
   108	                // No data available yet. Trip only if the whole idle
   109	                // window has elapsed since the last progress; otherwise
   110	                // stay pending (the deadline poll registers our waker).
   111	                match this.deadline.as_mut().poll(cx) {
   112	                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
   113	                        io::ErrorKind::TimedOut,
   114	                        format!("transfer stalled: no data received for {:?}", this.timeout),
   115	                    ))),
   116	                    Poll::Pending => Poll::Pending,
   117	                }
   118	            }
   119	        }
   120	    }
   121	}
   122	
   123	/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
   124	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on every
   125	/// successful `poll_write` (any byte count > 0 counts as progress), so it
   126	/// is a per-gap idle timeout, not a cap on the whole transfer.
   127	///
   128	/// audit-h3b: the daemon-side pull data plane writes bytes to the puller.
   129	/// If the puller stops reading mid-stream, TCP flow control fills the
   130	/// kernel send buffer and `write_all` blocks indefinitely (until OS-level
   131	/// TCP retransmit exhaustion, often 15+ minutes). Wrapping the inner
   132	/// stream in this adapter turns that into a clean
   133	/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
   134	/// observable write progress.
   135	///
   136	/// Symmetric in spirit with [`StallGuard`] on the read side: same idle-
   137	/// timeout semantics, same load-bearing property that a steadily-
   138	/// progressing transfer (any non-trivial network at all) is never

 succeeded in 0ms:
  1130	    peer.send(hello_frame()).await.unwrap();
  1131	    assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
  1132	    peer.send(wire(Frame::Accept(Default::default())))
  1133	        .await
  1134	        .unwrap();
  1135	    loop {
  1136	        match recv_or_panic(&mut peer).await {
  1137	            Frame::ManifestEntry(_) => continue,
  1138	            Frame::ManifestComplete(_) => break,
  1139	            other => panic!("expected manifest stream, got {other:?}"),
  1140	        }
  1141	    }
  1142	    peer.send(wire(Frame::BlockHashes(BlockHashList {
  1143	        relative_path: "real.txt".into(),
  1144	        block_size: RESUME_BS,
  1145	        hashes: Vec::new(),
  1146	    })))
  1147	    .await
  1148	    .unwrap();
  1149	
  1150	    let source_err = source_task.await.unwrap().unwrap_err();
  1151	    let fault = fault_of(&source_err);
  1152	    assert_eq!(fault.code, session_error::Code::ProtocolViolation);
  1153	    assert!(
  1154	        fault.message.contains("without a held resume need"),
  1155	        "got: {}",
  1156	        fault.message
  1157	    );
  1158	}
  1159	
  1160	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1161	async fn many_tiny_files_reach_shape_target_when_source_initiates() {
  1162	    // sf-2 pin ported onto the unified session (otp-4b-2). The responder
  1163	    // grants the zero-knowledge single stream (no manifest seen at
  1164	    // SessionAccept); a 10k-tiny-file transfer over the TCP data plane
  1165	    // must re-run the shape table over the accumulated need list and grow
  1166	    // the stream count past 1 via `DataPlaneResize{ADD}`. Mirrors the old
  1167	    // push sf-2 pin (`shape_resize_e2e.rs`), now on the session: the
  1168	    // settled count is read from the destination's `data_plane_streams`.
  1169	    let tmp = tempfile::tempdir().unwrap();
  1170	    let src_root = tmp.path().join("src");
  1171	    let dst_root = tmp.path().join("dst");
  1172	    std::fs::create_dir_all(&src_root).unwrap();
  1173	    std::fs::create_dir_all(&dst_root).unwrap();
  1174	    const FILE_COUNT: usize = 10_000;
  1175	    for i in 0..FILE_COUNT {
  1176	        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
  1177	    }
  1178	
  1179	    // SOURCE initiator over the TCP data plane: the control lane rides the
  1180	    // in-process transport; the data-plane sockets ride loopback TCP (the
  1181	    // responder binds 0.0.0.0:0 and the source dials 127.0.0.1).
  1182	    let open = SessionOpen {
  1183	        initiator_role: TransferRole::Source as i32,
  1184	        compare_mode: ComparisonMode::SizeMtime as i32,
  1185	        in_stream_bytes: false,
  1186	        ..Default::default()
  1187	    };
  1188	    let source_cfg = SourceSessionConfig {
  1189	        instruments: Default::default(),
  1190	        hello: HelloConfig::default(),
  1191	        endpoint: SessionEndpoint::initiator(open),
  1192	        plan_options: PlanOptions::default(),
  1193	        data_plane_host: Some("127.0.0.1".into()),
  1194	    };
  1195	    let dest_cfg = DestinationSessionConfig {
  1196	        hello: HelloConfig::default(),
  1197	        endpoint: SessionEndpoint::Responder,
  1198	        data_plane_host: None,
  1199	        instruments: Default::default(),
  1200	        local_apply: None,
  1201	    };
  1202	    let (a, b) = in_process_pair();
  1203	    let source = Arc::new(FsTransferSource::new(src_root.clone()));
  1204	    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
  1205	        tokio::join!(
  1206	            run_source(source_cfg, a, source),
  1207	            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
  1208	        )
  1209	    })
  1210	    .await
  1211	    .expect("session run timed out");
  1212	
  1213	    let summary = source_result.expect("source succeeds");
  1214	    let outcome = dest_result.expect("destination succeeds");
  1215	    assert!(
  1216	        !summary.in_stream_carrier_used,
  1217	        "the sf-2 pin must ride the TCP data plane"
  1218	    );
  1219	    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
  1220	    let streams = outcome
  1221	        .data_plane_streams
  1222	        .expect("data plane ran, stream count recorded");
  1223	    assert_eq!(
  1224	        streams, 8,
  1225	        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
  1226	         target regardless of which endpoint initiated the session"
  1227	    );
  1228	    assert_trees_identical(&src_root, &dst_root);
  1229	}
  1230	
  1231	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1232	async fn pull_data_plane_single_stream_lands_bytes() {
  1233	    // otp-5b-1: the transport/role decoupling in the PULL direction — the
  1234	    // mirror of the push data-plane test above. Here the DESTINATION is the
  1235	    // *initiator* (dials + receives) and the SOURCE is the *responder*
  1236	    // (binds + accepts + sends). Control frames ride the in-process
  1237	    // transport; the data-plane socket rides loopback TCP (the SOURCE
  1238	    // responder binds 0.0.0.0:0, the DESTINATION initiator dials
  1239	    // 127.0.0.1). Single-stream because this 4-file tree's shape wants only
  1240	    // one stream — the pull data plane CAN resize (otp-5b-2), but a small
  1241	    // need list never crosses the shape threshold; the resize itself is
  1242	    // pinned by `many_tiny_files_reach_shape_target_when_destination_initiates`.
  1243	    let tmp = tempfile::tempdir().unwrap();
  1244	    let src_root = tmp.path().join("src");
  1245	    let dst_root = tmp.path().join("dst");
  1246	    std::fs::create_dir_all(&src_root).unwrap();
  1247	    std::fs::create_dir_all(&dst_root).unwrap();
  1248	    write_tree(
  1249	        &src_root,
  1250	        &[
  1251	            ("a.txt", b"alpha".to_vec(), 1_600_000_001),
  1252	            ("empty.bin", b"".to_vec(), 1_600_000_002),
  1253	            ("dir/b.log", b"beta beta beta".to_vec(), 1_600_000_003),
  1254	            ("dir/deep/c.dat", b"gamma-content".to_vec(), 1_600_000_004),
  1255	        ],
  1256	    );
  1257	
  1258	    // DESTINATION initiator; SOURCE responder — the roles flipped from the
  1259	    // push data-plane test, the data plane following connection role.
  1260	    let open = SessionOpen {
  1261	        initiator_role: TransferRole::Destination as i32,
  1262	        compare_mode: ComparisonMode::SizeMtime as i32,
  1263	        in_stream_bytes: false,
  1264	        ..Default::default()
  1265	    };
  1266	    let source_cfg = SourceSessionConfig {
  1267	        instruments: Default::default(),
  1268	        hello: HelloConfig::default(),
  1269	        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
  1270	        plan_options: PlanOptions::default(),
  1271	        data_plane_host: None, // a responder never dials
  1272	    };
  1273	    let dest_cfg = DestinationSessionConfig {
  1274	        hello: HelloConfig::default(),
  1275	        endpoint: SessionEndpoint::initiator(open), // dials + receives
  1276	        data_plane_host: Some("127.0.0.1".into()),
  1277	        instruments: Default::default(),
  1278	        local_apply: None,
  1279	    };
  1280	    let (a, b) = in_process_pair();
  1281	    let source = Arc::new(FsTransferSource::new(src_root.clone()));
  1282	    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
  1283	        tokio::join!(
  1284	            run_source(source_cfg, a, source),
  1285	            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
  1286	        )
  1287	    })
  1288	    .await
  1289	    .expect("session run timed out");
  1290	
  1291	    let summary = source_result.expect("source responder succeeds");
  1292	    let outcome = dest_result.expect("destination initiator succeeds");
  1293	    assert!(
  1294	        !summary.in_stream_carrier_used,
  1295	        "the pull data plane must ride TCP, not the in-stream carrier"
  1296	    );
  1297	    assert_eq!(
  1298	        summary, outcome.summary,
  1299	        "both ends must hold the same summary"
  1300	    );
  1301	    assert_eq!(outcome.summary.files_transferred, 4);
  1302	    assert_eq!(
  1303	        outcome.data_plane_streams,
  1304	        Some(1),
  1305	        "a 4-file need list stays single-stream (below the shape threshold)"
  1306	    );
  1307	    assert_trees_identical(&src_root, &dst_root);
  1308	}
  1309	
  1310	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  1311	async fn many_tiny_files_reach_shape_target_when_destination_initiates() {
  1312	    // otp-5b-2: the sf-2 shape correction in the PULL direction — the
  1313	    // mirror of `many_tiny_files_reach_shape_target_when_source_initiates`
  1314	    // (push). Here the DESTINATION is the *initiator* (dials the epoch-N
  1315	    // sockets it grows to) and the SOURCE is the *responder* (accepts them
  1316	    // off its bound listener). The control-lane `DataPlaneResize{ADD}` /
  1317	    // `DataPlaneResizeAck` frames are identical to push; only the transport
  1318	    // action flips. A 10k-tiny-file transfer must re-run the shape table
  1319	    // over the accumulated need list and grow the stream count past 1.
  1320	    let tmp = tempfile::tempdir().unwrap();
  1321	    let src_root = tmp.path().join("src");
  1322	    let dst_root = tmp.path().join("dst");
  1323	    std::fs::create_dir_all(&src_root).unwrap();
  1324	    std::fs::create_dir_all(&dst_root).unwrap();
  1325	    const FILE_COUNT: usize = 10_000;
  1326	    for i in 0..FILE_COUNT {
  1327	        std::fs::write(src_root.join(format!("f{i:05}.bin")), b"x").unwrap();
  1328	    }
  1329	
  1330	    // DESTINATION initiator; SOURCE responder — roles flipped from the push
  1331	    // shape test, the data plane following connection role.
  1332	    let open = SessionOpen {
  1333	        initiator_role: TransferRole::Destination as i32,
  1334	        compare_mode: ComparisonMode::SizeMtime as i32,
  1335	        in_stream_bytes: false,
  1336	        // Wire contract: zero means unknown, not a one-stream cap. Pin it
  1337	        // on the destination-initiator orientation, where this end both
  1338	        // advertises and enforces the receiver ceiling.
  1339	        receiver_capacity: Some(CapacityProfile {
  1340	            max_streams: 0,
  1341	            ..Default::default()
  1342	        }),
  1343	        ..Default::default()
  1344	    };
  1345	    let source_cfg = SourceSessionConfig {
  1346	        instruments: Default::default(),
  1347	        hello: HelloConfig::default(),
  1348	        endpoint: SessionEndpoint::Responder, // binds + accepts + sends
  1349	        plan_options: PlanOptions::default(),
  1350	        data_plane_host: None, // a responder never dials
  1351	    };
  1352	    let dest_cfg = DestinationSessionConfig {
  1353	        hello: HelloConfig::default(),
  1354	        endpoint: SessionEndpoint::initiator(open), // dials + receives
  1355	        data_plane_host: Some("127.0.0.1".into()),
  1356	        instruments: Default::default(),
  1357	        local_apply: None,
  1358	    };
  1359	    let (a, b) = in_process_pair();
  1360	    let source = Arc::new(FsTransferSource::new(src_root.clone()));
  1361	    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
  1362	        tokio::join!(
  1363	            run_source(source_cfg, a, source),
  1364	            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
  1365	        )
  1366	    })
  1367	    .await
  1368	    .expect("session run timed out");
  1369	
  1370	    let summary = source_result.expect("source responder succeeds");
  1371	    let outcome = dest_result.expect("destination initiator succeeds");
  1372	    assert!(
  1373	        !summary.in_stream_carrier_used,
  1374	        "the pull sf-2 pin must ride the TCP data plane"
  1375	    );
  1376	    assert_eq!(summary.files_transferred, FILE_COUNT as u64);
  1377	    let streams = outcome
  1378	        .data_plane_streams
  1379	        .expect("data plane ran, stream count recorded");
  1380	    assert_eq!(
  1381	        streams, 8,
  1382	        "a {FILE_COUNT}-file transfer must reach the shape policy's eight-stream \
  1383	         target regardless of which endpoint initiated the session"
  1384	    );
  1385	    assert_trees_identical(&src_root, &dst_root);
  1386	}
  1387	
  1388	#[tokio::test]
  1389	async fn preserves_mtime_on_streamed_files() {
  1390	    // Not part of the role matrix — pins that the file-record write
  1391	    // path applies the manifest mtime (parity with today's receive
  1392	    // paths, which the byte-identical asserts alone wouldn't catch).
  1393	    let tmp = tempfile::tempdir().unwrap();
  1394	    let src_root = tmp.path().join("src");
  1395	    let dst_root = tmp.path().join("dst");
  1396	    std::fs::create_dir_all(&src_root).unwrap();
  1397	    std::fs::create_dir_all(&dst_root).unwrap();
  1398	    write_tree(
  1399	        &src_root,
  1400	        &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
  1401	    );
  1402	
  1403	    let (source_result, dest_result) = run_session(
  1404	        TransferRole::Source,
  1405	        &src_root,

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1638,1682p'; nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1888,1950p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
  1638	        None => Err(eyre::Report::new(SessionFault::internal(
  1639	            "source receive half ended before TransferSummary",
  1640	        ))),
  1641	    }
  1642	}
  1643	
  1644	/// Process every event ready right now (needs accumulating, resize acks
  1645	/// dialing their epoch-N socket) without blocking. Called between
  1646	/// manifest sends and at the top of the payload loop.
  1647	#[allow(clippy::too_many_arguments)]
  1648	async fn drain_ready_source_events(
  1649	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1650	    pending: &mut Vec<FileHeader>,
  1651	    resume: &mut ResumeSendState,
  1652	    need_complete: &mut bool,
  1653	    needed_bytes: &mut u64,
  1654	    needed_count: &mut usize,
  1655	    data_plane: Option<&data_plane::SourceDataPlane>,
  1656	    tx: &mut Box<dyn FrameTx>,
  1657	    pending_resize: &mut Option<data_plane::PendingResize>,
  1658	) -> Result<()> {
  1659	    while let Ok(event) = events.try_recv() {
  1660	        process_source_event(
  1661	            event,
  1662	            pending,
  1663	            resume,
  1664	            need_complete,
  1665	            needed_bytes,
  1666	            needed_count,
  1667	            data_plane,
  1668	            tx,
  1669	            pending_resize,
  1670	        )
  1671	        .await?;
  1672	    }
  1673	    Ok(())
  1674	}
  1675	
  1676	/// Handle one source event. Needs accumulate into `pending` and the
  1677	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1678	/// next ADD (the one-per-epoch ramp).
  1679	#[allow(clippy::too_many_arguments)]
  1680	async fn process_source_event(
  1681	    event: SourceEvent,
  1682	    pending: &mut Vec<FileHeader>,
  1888	/// Block for the ack of the one in-flight resize and dial its socket (or
  1889	/// settle it refused). Does NOT propose further — it resolves exactly the
  1890	/// pending proposal so the destination's armed slot is consumed before we
  1891	/// finish the data plane.
  1892	async fn resolve_in_flight_resize(
  1893	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1894	    dp: &data_plane::SourceDataPlane,
  1895	    pending: data_plane::PendingResize,
  1896	) -> Result<()> {
  1897	    loop {
  1898	        match events.recv().await {
  1899	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1900	                if ack.accepted {
  1901	                    dp.add_stream(&pending.sub_token).await?;
  1902	                    dp.dial()
  1903	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1904	                } else {
  1905	                    dp.dial()
  1906	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1907	                }
  1908	                return Ok(());
  1909	            }
  1910	            // A stale ack for an already-settled epoch: ignore, keep
  1911	            // waiting for ours.
  1912	            Some(SourceEvent::ResizeAck(_)) => continue,
  1913	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1914	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1915	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1916	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1917	                )))
  1918	            }
  1919	            Some(SourceEvent::BlockHashes(l)) => {
  1920	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1921	                    format!(
  1922	                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
  1923	                        l.relative_path
  1924	                    ),
  1925	                )))
  1926	            }
  1927	            Some(SourceEvent::NeedComplete) => {
  1928	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1929	                    "duplicate NeedComplete",
  1930	                )))
  1931	            }
  1932	            Some(SourceEvent::Summary(_)) => {
  1933	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1934	                    "TransferSummary before SourceDone",
  1935	                )))
  1936	            }
  1937	            None => {
  1938	                return Err(eyre::Report::new(SessionFault::internal(
  1939	                    "source receive half ended with a resize in flight",
  1940	                )))
  1941	            }
  1942	        }
  1943	    }
  1944	}
  1945	
  1946	/// Await the next terminal signal the receive half forwards while the
  1947	/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
  1948	/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
  1949	/// abort the send and surface as the fault.
  1950	///

exec
/bin/zsh -lc "rg -n \"receiver_capacity: Some\\(CapacityProfile|max_streams:\" crates/blit-core/tests/transfer_session_roles.rs | head -100; rg -n \"receiver_stream_ceiling\" -g '*.rs'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
1339:        receiver_capacity: Some(CapacityProfile {
1340:            max_streams: 0,
crates/blit-core/src/transfer_session/mod.rs:2803:                    let ceiling = crate::dial::receiver_stream_ceiling(
crates/blit-core/src/dial.rs:89:pub fn receiver_stream_ceiling(profile: Option<&CapacityProfile>) -> usize {
crates/blit-core/src/dial.rs:159:        let ceiling_streams = receiver_stream_ceiling(profile);

exec
/bin/zsh -lc 'for rev in a76b785''^ a76b785; do printf '"'%s ' \""'$rev"; git grep -h -E '"'"'^'"\\s*#\\[(tokio::)?test' \""'$rev" -- '"'crates/**/*.rs' 'tests/**/*.rs' 2>/dev/null | wc -l; done; rg -n \"1488 tests|1488|39/39\" .review docs REVIEW.md .agents -g '*.md' | head -80" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
a76b785^      266
a76b785      266
REVIEW.md:73:| otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
REVIEW.md:75:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
docs/STATE.md:18:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
docs/STATE.md:196:  Created `docs/history/state-archive.md`, anchored `Suite 1488 as of bb28ddd`. Full: **DEVLOG 21:10Z**.
.review/findings/otp-12-worker-parity.md:49:  `transfer_session_roles` integration target passes 39/39.
.review/findings/otp-12-worker-parity.md:52:  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
.review/findings/otp-10c-2-driver-deletion.md:212:  pins bring it to 1488 (+5 margin). otp-11 (local orchestration
docs/plan/OTP11_LOCAL_SESSION.md:18:floor (suite 1488 vs ≥1483). Three design decisions were not settled by the
docs/plan/OTP11_LOCAL_SESSION.md:296:Post-11a suite: 1513/0 (baseline 1488 + the 22 landed 11a pins + the
.review/results/otp12-perf-findings-r5.codex.md:153:- **F5 — HIGH — [OTP12_PERF_FINDINGS.md:481](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:481): the suite floor is stale.** HEAD’s floor is 1488: the recorded 1484 plus three tests in `ace91de` and one in `920c6a7`, with no removals. A 1484 floor permits silently losing four tests.
.review/results/otp12-perf-findings-r5.codex.md:170:- **F5 — HIGH — [OTP12_PERF_FINDINGS.md:481](/Users/michael/Dev/blit_v2/docs/plan/OTP12_PERF_FINDINGS.md:481): the suite floor is stale.** HEAD’s floor is 1488: the recorded 1484 plus three tests in `ace91de` and one in `920c6a7`, with no removals. A 1484 floor permits silently losing four tests.
.review/results/otp-4b3-data-plane.fix-review.codex.md:4216:+  1488	                let chunk = std::mem::take(&mut pending);
.review/results/otp-4b3-data-plane.fix-review.codex.md:5309:+  1488	    #[tokio::test]
.review/results/otp-4b3-data-plane.fix-review.codex.md:7926:  1488	                        header.relative_path
.review/results/otp-10c-2.gpt-verdict.md:94:suite after fixes: 1480 → **1488** (+8 pins; gate green: fmt,
.review/results/otp-10c-2.gpt-verdict.md:95:clippy -D warnings, `cargo test --workspace --no-fail-fast` 1488/0)
.review/results/pf-0-prereg.codex.md:758:     6	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/pf-0-prereg.codex.md:934:   182	  fmt fix (`bb28ddd`, suite **1488**).
.review/findings/otp-11a-local-session-route.md:98:Suite before → after: 1488 → 1510 (+22: the 21 pins below plus the
.review/results/macmac-harness-r7.codex.md:1392:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/otp12-perf-findings.codex.md:366:    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
.review/results/otp12-perf-findings.codex.md:383:    COMPLETES. Suite 1488 → 1513 → **1484** (≥1483 floor met at the
.review/results/otp12-perf-findings.codex.md:516:  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
.review/results/otp12-perf-findings.codex.md:520:  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
.review/results/otp12-perf-findings.codex.md:570:REVIEW.md:75:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
.review/results/otp12-perf-findings.codex.md:5982:  1488	                    // break a cancel also causes (otp-4b-3 codex F1).
.review/results/otp-11a.gpt-verdict.md:8:recombination sound, and the suite delta honest (1488 → 1510, none
.review/results/p1-adjudication-r1.codex.md:873:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/p1-adjudication-r1.codex.md:1049:  anchored `Suite 1488 as of bb28ddd`, rig IPs → `.agents/machines.md`. Full: **DEVLOG 21:10Z**.
.review/results/sf-2-shape-correction-resize.codex.md:2452:  1488	        if let Err(join_err) = response_task.join().await {
.review/results/otp-4a-daemon-serves-transfer.codex.md:4215:crates/blit-core/src/remote/transfer/sink.rs:1488:                compare_mode: ComparisonMode::SizeMtime,
.review/results/otp-4a-daemon-serves-transfer.codex.md:5338:.review/results/bench-script-fix.codex.md:12711:crates/blit-core/src/remote/transfer/sink.rs:1488:                compare_mode: ComparisonMode::SizeMtime,
.review/results/otp-4a-daemon-serves-transfer.codex.md:7206:  1488	            tokio::time::timeout(std::time::Duration::from_millis(50), fut)
.review/results/local-bistability.codex.md:365:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/local-bistability.codex.md:541:  fmt fix (`bb28ddd`, suite **1488**).
.review/results/otp-12b-run.codex.md:441:docs/STATE.md-44-    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
.review/results/macmac-harness-r2.codex.md:356:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/macmac-harness-r2.codex.md:614:| otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
.review/results/macmac-harness-r2.codex.md:616:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
.review/results/macmac-harness-r2.codex.md:5453:    12	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/otp-11a.codex.md:14:Review the diff of commit dfdddd6 (run: git show dfdddd6). It implements otp-11a per docs/plan/OTP11_LOCAL_SESSION.md (design commit 0da65d6): local transfers ride the unified TransferSession — run_local_session (new crates/blit-core/src/transfer_session/local.rs) joins run_source+run_destination over in_process_pair with a process-local LocalApply extension on DestinationSessionConfig under which the destination applies needed files itself via plan_transfer_payloads + execute_sink_pipeline_streaming + FsTransferSink (no payload bytes on any transport, no wire representation); blit_app::transfers::local::run re-pointed; mirror_delete_pass gains execute + (files,dirs) split; sink file-root File-payload ENOTDIR fix; 21 ported/new pins in crates/blit-core/tests/local_session.rs + 1 unit test (suite 1488 to 1510); A/B bench harness scripts/bench_otp11_local_ab.sh. Check: correctness regressions (especially: any behavior change on the REMOTE session paths from the shared-code edits — sink selection, record helpers widened to &dyn TransferSink, mirror_delete_pass signature; deadlock/backpressure of the local apply loop vs the in-process transport; the unreadable/move-gate posture; dry-run and null-sink semantics vs the old orchestrator; the dest-subtree exclusion; summary synthesis fidelity incl. outcome classification and the deleted files/dirs split), the slice's acceptance criteria from the plan doc, FAST/SIMPLE/RELIABLE, the one-transfer-path invariant (does LocalApply constitute a second path? grade against D-2026-07-05-1/-3), byte-identical/StallGuard/cancellation/byte-accounting invariants for the touched area, and that the test count did not drop (1488 -> 1510). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
.review/results/otp-11a.codex.md:129:**VERDICT: FAIL** — remote shared paths and the files/dirs recombination appear unchanged, and the source delta supports 1488→1510 (+22 tests, none removed), but correctness, cancellation, one-path, and mandatory performance-gate failures block acceptance.
.review/results/otp-11a.codex.md:150:**VERDICT: FAIL** — remote shared paths and the files/dirs recombination appear unchanged, and the source delta supports 1488→1510 (+22 tests, none removed), but correctness, cancellation, one-path, and mandatory performance-gate failures block acceptance.
.review/results/otp-5b-source-responder-data-plane.codex.md:3776:  1488	    )
.review/results/macmac-harness.codex.md:700:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/pf-0-rebaseline-decision.codex.md:416: - Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/pf-0-rebaseline-decision.codex.md:2290:    11	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/pf-0-rebaseline-decision.codex.md:5216:    10	- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/pf-0-rebaseline-decision.codex.md:5312: - Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/otp-12a.codex.md:2198:    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
.review/results/otp-12a.codex.md:2215:    COMPLETES. Suite 1488 → 1513 → **1484** (≥1483 floor met at the
.review/results/otp-12a.codex.md:2348:  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
.review/results/otp-12a.codex.md:2352:  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
.review/results/otp-12a.codex.md:2427:| otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
.review/results/otp-12a.codex.md:2429:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |
.review/results/otp-12a.codex.md:3300:docs/STATE.md-194-  suite 1488 → 1484 with the ≥1483 floor met by real pins; the
.review/results/otp-12a.codex.md:3304:docs/STATE.md-198-  deletion); suite 1605 → 1488. Owner ask pending: `725aa07` snapshot.
.review/results/otp-12a.codex.md:4354:.review/results/otp-1-wire-session-contract.codex.md-1488-+    CANCELLED = 8;
.review/results/otp-5a-daemon-as-source.codex.md:3717:  1488	        Err(report) => {
.review/results/otp-5a-daemon-as-source.codex.md:8541:  1488	    #[tokio::test]
.review/results/otp-4b1-data-plane.codex.md:5547:  1488	    }
.review/results/otp-4b1-data-plane.codex.md:8338:  1488	    }
.review/results/otp-11-design.codex.md:152:- [OTP11_LOCAL_SESSION.md:221](/Users/michael/Dev/blit_v2/docs/plan/OTP11_LOCAL_SESSION.md:221) — **High** — The floor does not close: exact retirements are 71 tests, so `1488 - 71 + 26 = 1443`; reaching 1483 requires at least 40 additional committed tests. All 16 manifest tests are deleted, contrary to “live-half tests stay.”
.review/results/otp-11-design.codex.md:175:- [OTP11_LOCAL_SESSION.md:221](/Users/michael/Dev/blit_v2/docs/plan/OTP11_LOCAL_SESSION.md:221) — **High** — The floor does not close: exact retirements are 71 tests, so `1488 - 71 + 26 = 1443`; reaching 1483 requires at least 40 additional committed tests. All 16 manifest tests are deleted, contrary to “live-half tests stay.”
.review/results/otp-4b2-data-plane.codex.md:4751:  1488	                {
.review/results/otp-4b3-data-plane.codex.md:2911:  1488	                let chunk = std::mem::take(&mut pending);
.review/results/otp-4b3-data-plane.codex.md:4004:  1488	    #[tokio::test]
.review/results/otp-12-worker-parity.codex.md:348:  `transfer_session_roles` integration target passes 39/39.
.review/results/otp-12-worker-parity.codex.md:351:  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
.review/results/otp-12-worker-parity.codex.md:731:+  `transfer_session_roles` integration target passes 39/39.
.review/results/otp-12-worker-parity.codex.md:734:+  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
.review/results/otp-12-worker-parity.codex.md:2412:  1488	                    // correction exactly as plain batches do — a
.review/results/otp-12-worker-parity.codex.md:4761:  1488	                    // correction exactly as plain batches do — a
.review/results/otp-12-worker-parity.codex.md:5888:  1488	                    // correction exactly as plain batches do — a
.review/results/otp-12-worker-parity.codex.md:8804:docs/plan/OTP11_LOCAL_SESSION.md-18-floor (suite 1488 vs ≥1483). Three design decisions were not settled by the
.review/results/otp-12-worker-parity.codex.md:8978:.review/findings/otp-12-worker-parity.md-49-  `transfer_session_roles` integration target passes 39/39.
.review/results/otp-12-worker-parity.codex.md:8981:.review/findings/otp-12-worker-parity.md-52-  `cargo test --workspace` (1488 tests, 2 ignored; no failures).
.review/results/otp-12-worker-parity.codex.md:9157:docs/STATE.md:18:- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
.review/results/otp-12-worker-parity.codex.md:9300:REVIEW.md:73:| otp-10c-2 | The cutover deletion — otp-10c CLOSED, one transfer path by construction: the four drivers (`remote/pull.rs` 2574 LOC, `remote/push/`, daemon `service/push/`, `service/pull_sync.rs`), `rpc Push` + `rpc PullSync` + 13 exclusive messages (incl. `DataTransferNegotiation`, the old summaries, `metadata_only`), the two wire-specific gRPC fallback sinks + `grpc_fallback.rs`, and every helper whose only callers died — out of tree AND proto, no bridge (D-2026-07-05-2). Relocated verbatim: the delegated spec builder (`DelegatedSpecOptions`/`delegated_spec_from_options` → operation_spec.rs) + `FsTransferSource`'s fs-scan helpers. A/B parity pins → absolute tree+count pins; DelegatedPull no-payload-bytes proof recorded (proto oneof + CLI byte-counter pins). Codex NEEDS FIXES → 6/6 accepted (F6 owner-gated): spec capability/capacity fields + `PeerCapabilities` deleted (orphaned since otp-9b); 5 more orphaned helpers out; the relocated builder re-pinned (7 tests) + `mirror_delete_pass` containment wiring pinned — both mutation-proven; `docs/API.md` (never swept) + 4 more doc/comment sites fixed; `w6-2b` re-scoped to the served-session dispatcher; the tracked `.claude/worktrees` snapshot deferred to the standing `725aa07` owner question. Suite 1586 → 1480 (106 retirements, all enumerated in the finding doc) → **1488** | `[x]` | `7aac28b` + review fixes `995e1cc` |
.review/results/otp-12-worker-parity.codex.md:9302:REVIEW.md:75:| otp-11a | Local transfers ride the session — the local route (`docs/plan/OTP11_LOCAL_SESSION.md` D1–D3): `run_local_session` joins both role drivers over `in_process_pair`; the LOCAL byte-carrier = process-local `LocalApply` (crate-private, NO wire shape — a peer structurally cannot select it): the destination plans (`plan_transfer_payloads`) and applies needs in-process through `FsTransferSink` — clonefile/block-clone/copy_file_range kept, `execute_sink_pipeline_streaming` stays live as the apply pipeline; `blit_app transfers/local.rs` chokepoint re-pointed (CLI+TUI call sites untouched, all verb pins green incl. the 3 move data-loss regression pins); ONE diff core both carriers (`diff_chunk_verdicts`); mirror = the in-session delete rule + apply-time unreadable guard (old R46-F2 posture, vanishing-source pin) + plan-only dry-run + split (files,dirs) counts; sink file-root File-payload ENOTDIR fix. Design-doc codex CHANGES REQUIRED → 10 findings adjudicated (3 already fixed in the slice; doc amended — D1 carrier delta stated, floor redone: 11b needs ≈+44 real pins); slice codex FAIL → 9 findings: 7 accepted+fixed, 1 doc defect (outcome parity gate kept), 1 rejected-as-regression (diff batching is session-uniform; overlap pin ports at 11b). A/B perf gate: huge/tree/small PASS (1 GiB single file 22 ms BOTH sides — clone preserved); focused noop10k surfaced the journal-skip retirement cost (~21 ms warm-journal vs ~219 ms full diff; beats the old non-journal pass at 610 ms) — OWNER question, blocks 11b per the slice doc's gate rule. Suite 1488 → 1510 → **1512**; 4 mutation guard proofs. **Addendum (owner: "neither option passes — figure out a real fix"): the old journal fast path proven UNSOUND** — `NoChanges` decays to root-dir mtime equality; deep modifications silently never synced (reproduced vs the `d2bd843` binary, transcript in the bench README); no-op cell re-baselined sound-vs-sound (session 2.8× faster) → gate PASSES, 11b unblocked (its journal deletion removes a data-loss bug); pin `deep_modification_after_warm_runs_syncs` (suite → **1513**); sound journal REPLAY filed as future session capability (slice doc D3). Addendum codex CHANGES REQUESTED → core verdict CONFIRMED (data loss real, no validation layer, Windows fallback also unsound, pin guards the shape); 4/4 record findings fixed — sound baseline re-certified by 5-run medians with the old journal cache cleared per run (old 507 ms vs session 226 ms = 2.2×, gate PASS), STATE summary line, floor redone from 1513 (≈+41), Linux ctime-arm mechanism precision. | `[x]` | design `0da65d6`+`c7b463b`; slice `dfdddd6` + review fixes `e445e8d`; bench `631255b`; addendum `d74c1ac`+`4148705` + review fixes (see verdict) |

exec
/bin/zsh -lc "rg -n \"pub fn initial_stream_proposal|fn initial_stream_proposal|FILE_LIST_EARLY|DEST_DIFF_CHUNK|SUITE_TIMEOUT\" crates/blit-core/src/dial.rs crates/blit-core/src/transfer_session/mod.rs crates/blit-core/tests/transfer_session_roles.rs; nl -ba crates/blit-core/src/dial.rs | sed -n '485,555p'; nl -ba proto/blit.proto | sed -n '125,210p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
crates/blit-core/src/dial.rs:345:    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
crates/blit-core/src/dial.rs:483:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
crates/blit-core/src/dial.rs:696:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
crates/blit-core/tests/transfer_session_roles.rs:34:const SUITE_TIMEOUT: Duration = Duration::from_secs(120);
crates/blit-core/tests/transfer_session_roles.rs:154:    tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:482:        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:908:    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1040:        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1204:    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1282:    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1361:    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1458:        let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:1864:    let dest_err = tokio::time::timeout(SUITE_TIMEOUT, dest)
crates/blit-core/tests/transfer_session_roles.rs:1963:    let dest_err = tokio::time::timeout(SUITE_TIMEOUT, dest)
crates/blit-core/tests/transfer_session_roles.rs:2041:    let refusal = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/tests/transfer_session_roles.rs:2210:    let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
crates/blit-core/src/transfer_session/mod.rs:77:const DEST_DIFF_CHUNK: usize = 128;
crates/blit-core/src/transfer_session/mod.rs:2889:                if pending.len() >= DEST_DIFF_CHUNK {
crates/blit-core/src/transfer_session/mod.rs:3611:/// hashes up to DEST_DIFF_CHUNK files (codex otp-10b-1 F3), so the
   485	        return 1;
   486	    }
   487	    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
   488	        16
   489	    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
   490	        12
   491	    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
   492	        10
   493	    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
   494	        8
   495	    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
   496	        4
   497	    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
   498	        2
   499	    } else {
   500	        1
   501	    };
   502	    proposal.min(ceiling.max(1) as u32)
   503	}
   504	
   505	/// Blocked-time ratio for one tuner tick: the share of the tick's
   506	/// wall-clock (× stream count) the senders spent inside socket writes.
   507	/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
   508	/// hysteresis band holds the dial still rather than guessing).
   509	pub(crate) fn blocked_ratio(
   510	    delta_blocked_nanos: u64,
   511	    elapsed: std::time::Duration,
   512	    streams: usize,
   513	) -> f64 {
   514	    let denom = elapsed.as_nanos().saturating_mul(streams as u128);
   515	    if denom == 0 {
   516	        return 0.0;
   517	    }
   518	    (delta_blocked_nanos as f64 / denom as f64).clamp(0.0, 1.0)
   519	}
   520	
   521	/// Growable per-transfer probe registry (`ue-r2-2`): resize adds a
   522	/// probe when a stream joins and removes it when one retires, and the
   523	/// tuner samples whatever is live each tick. Plain std mutex — locked
   524	/// only for a snapshot fold every 500ms and on resize events.
   525	pub type SharedStreamProbes =
   526	    Arc<std::sync::Mutex<Vec<crate::remote::transfer::progress::StreamProbe>>>;
   527	
   528	/// Spawn the live tuner for one transfer (ue-r2-1e): every
   529	/// [`DIAL_TUNER_TICK`] it sums the PR1 per-stream `write_blocked`
   530	/// telemetry and steps the dial's cheap dials. Holds only a `Weak` to
   531	/// the dial, so it self-terminates within one tick of the transfer
   532	/// dropping its dial; callers may also abort the handle for prompt
   533	/// shutdown (`MultiStreamSender::finish` does).
   534	pub fn spawn_dial_tuner(
   535	    dial: &Arc<TransferDial>,
   536	    probes: Vec<crate::remote::transfer::progress::StreamProbe>,
   537	) -> tokio::task::JoinHandle<()> {
   538	    spawn_dial_tuner_with_resize(dial, Arc::new(std::sync::Mutex::new(probes)), None)
   539	}
   540	
   541	/// `ue-r2-2` tuner: same cheap-dial stepping, but over a growable
   542	/// probe registry, plus the stream-resize policy when `resize_tx` is
   543	/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
   544	/// to the adapter that owns the control stream (unbounded so a
   545	/// momentarily busy adapter cannot lose a proposal while the dial
   546	/// holds it pending). Callers without resize pass `None` and get
   547	/// exactly the ue-r2-1e behavior.
   548	pub fn spawn_dial_tuner_with_resize(
   549	    dial: &Arc<TransferDial>,
   550	    probes: SharedStreamProbes,
   551	    resize_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeProposal>>,
   552	) -> tokio::task::JoinHandle<()> {
   553	    let weak = Arc::downgrade(dial);
   554	    tokio::spawn(async move {
   555	        let mut last_blocked: u64 = 0;
   125	// ── ue-r2-1b: receiver capacity profile ─────────────────────────────
   126	// The rich profile the byte RECEIVER advertises to the byte SENDER at
   127	// setup. The sender owns the live dial (chunk size, prefetch, in-flight
   128	// bytes, and — after ue-r2-2 — stream count) and must keep it within
   129	// this profile; the initial dial additionally starts BELOW the ceiling
   130	// with margin (REV4 "Risks": a receiver may over-advertise, and there
   131	// is no probe phase to catch it before the first byte).
   132	//
   133	// Travel direction (receiver → sender): the session DESTINATION
   134	// advertises its profile in SessionOpen/SessionAccept.receiver_capacity
   135	// (whichever end plays DESTINATION). Nothing else carries one — the
   136	// delegated initiator is the dst daemon itself (otp-9b), so it
   137	// advertises its own capacity at session open.
   138	//
   139	// Every field uses 0 (or UNSPECIFIED) as "unknown"; the sender treats
   140	// unknown as "no information — stay conservative", never as "unlimited".
   141	message CapacityProfile {
   142	  // Logical CPU cores the receiver can devote to this transfer.
   143	  uint32 cpu_cores = 1;
   144	  // Storage class of the receive target, the coarse drain-speed signal.
   145	  DrainClass drain_class = 2;
   146	  // Receiver's current overall load estimate, percent (0-100+; may
   147	  // exceed 100 when oversubscribed, e.g. loadavg > cores). 0 = unknown
   148	  // or idle — senders must not distinguish the two.
   149	  uint32 load_percent = 3;
   150	  // Maximum parallel data-plane streams the receiver will accept for
   151	  // this transfer (the dial's hard ceiling; floor is always 1).
   152	  // 0 = unknown → sender stays at today's negotiated stream_count.
   153	  uint32 max_streams = 4;
   154	  // Estimated sustainable drain (write-to-storage) rate in bytes/sec.
   155	  uint64 drain_rate_bytes_per_sec = 5;
   156	  // Largest single chunk the receiver wants on the wire, bytes.
   157	  uint64 max_chunk_bytes = 6;
   158	  // Ceiling on prefetch / un-acked in-flight bytes the receiver can
   159	  // buffer safely.
   160	  uint64 max_inflight_bytes = 7;
   161	}
   162	
   163	// Coarse storage class for CapacityProfile.drain_class. Deliberately
   164	// coarse: a hint for the sender's starting dial, not a benchmark.
   165	enum DrainClass {
   166	  DRAIN_CLASS_UNSPECIFIED = 0;
   167	  DRAIN_CLASS_HDD = 1;
   168	  DRAIN_CLASS_SSD_SATA = 2;
   169	  DRAIN_CLASS_SSD_NVME = 3;
   170	  DRAIN_CLASS_NETWORK_FS = 4; // receive target is itself remote (NFS/SMB/…)
   171	  DRAIN_CLASS_MEMORY = 5;     // tmpfs/ramdisk-class target
   172	}
   173	
   174	// ── ue-r2-1b: mid-transfer stream resize (consumed at ue-r2-2) ──────
   175	// Control-plane request to grow/shrink the live data-plane stream set.
   176	// Carried on the transfer control streams (never as a blind TCP
   177	// data-plane record), and only when the session negotiated resize
   178	// support at open.
   179	// Shape carried over from the adaptive-streams PR3 prior art (d9d4ec7).
   180	enum DataPlaneResizeOp {
   181	  DATA_PLANE_RESIZE_OP_UNSPECIFIED = 0;
   182	  DATA_PLANE_RESIZE_OP_ADD = 1;
   183	  DATA_PLANE_RESIZE_OP_REMOVE = 2;
   184	}
   185	
   186	// Controller → peer request to resize the live stream set. `epoch` is a
   187	// monotonic resize id (0 is reserved for the initial streams);
   188	// `target_stream_count` is the absolute desired live count (idempotent),
   189	// bounded by CapacityProfile.max_streams. For ADD, `sub_token` is the
   190	// 16-byte credential the newly dialed data socket must present after
   191	// the one_time_token; the accepting side registers it before the dialer
   192	// dials. For REMOVE, sub_token is empty.
   193	message DataPlaneResize {
   194	  DataPlaneResizeOp op = 1;
   195	  uint32 epoch = 2;
   196	  uint32 target_stream_count = 3;
   197	  bytes sub_token = 4;
   198	}
   199	
   200	// Ack of a DataPlaneResize. `accepted` is false if the peer could not
   201	// honor it (e.g. registration refused, dial failed);
   202	// `effective_stream_count` is the live count the acking side now
   203	// believes is in effect.
   204	message DataPlaneResizeAck {
   205	  uint32 epoch = 1;
   206	  uint32 effective_stream_count = 2;
   207	  bool accepted = 3;
   208	}
   209	
   210	// Shared transfer frames (session manifest / payload building blocks).

exec
/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '80,365p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    80	/// before sending that file's `BlockHashList`, and claims it inline on
    81	/// the in-stream carrier) and the data-plane receive (which validates
    82	/// and claims it as block records land on the sockets) — the same
    83	/// sharing shape as [`OutstandingNeeds`].
    84	pub(super) type ResumeHeaders = Arc<StdMutex<HashMap<String, FileHeader>>>;
    85	
    86	/// otp-7b: the resume half of the data-plane receive contract — present
    87	/// only when the session negotiated resume. `headers` is the shared
    88	/// grant map above; `resumed` is the destination's `files_resumed`
    89	/// counter, incremented here because the control loop never sees
    90	/// data-plane block records.
    91	pub(super) struct ResumeRecv {
    92	    pub(super) headers: ResumeHeaders,
    93	    pub(super) resumed: Arc<AtomicU64>,
    94	}
    95	
    96	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    97	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    98	}
    99	
   100	/// [`dp_fault`] for failures that stringify an underlying I/O-bearing
   101	/// report (socket dials): carry the `io::ErrorKind` so the retry
   102	/// classifier still sees a transient transport condition (codex
   103	/// otp-10a F5).
   104	fn dp_fault_io(err: &eyre::Report, msg: impl Into<String>) -> eyre::Report {
   105	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg).with_io_kind_from(err))
   106	}
   107	
   108	// ---------------------------------------------------------------------------
   109	// Responder (DESTINATION) — bind, grant, accept, receive
   110	// ---------------------------------------------------------------------------
   111	
   112	/// A bound data-plane listener plus the credentials the responder
   113	/// advertises in its `SessionAccept`. Held by the responder driver
   114	/// across the handshake so the accept loop can run after establish.
   115	pub(super) struct ResponderDataPlane {
   116	    listener: TcpListener,
   117	    session_token: Vec<u8>,
   118	    epoch0_sub_token: Vec<u8>,
   119	    initial_streams: u32,
   120	    port: u16,
   121	}
   122	
   123	/// Bind a data-plane listener and mint credentials for the grant. Any
   124	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
   125	/// issues a grant-less `SessionAccept` and the session falls back to the
   126	/// in-stream carrier (contract §Transport selection: a responder that
   127	/// cannot bind grants no data plane).
   128	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
   129	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
   130	        Ok(listener) => listener,
   131	        Err(err) => {
   132	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
   133	            return None;
   134	        }
   135	    };
   136	    let port = match listener.local_addr() {
   137	        Ok(addr) => addr.port(),
   138	        Err(err) => {
   139	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   140	            return None;
   141	        }
   142	    };
   143	    // Two independent 16-byte credentials (contract §Transport: a socket
   144	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   145	    // is the fallible-RNG minter — a missing system RNG is an error, not
   146	    // a weaker credential.
   147	    let session_token = match generate_sub_token() {
   148	        Ok(token) => token,
   149	        Err(err) => {
   150	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   151	            return None;
   152	        }
   153	    };
   154	    let epoch0_sub_token = match generate_sub_token() {
   155	        Ok(token) => token,
   156	        Err(err) => {
   157	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   158	            return None;
   159	        }
   160	    };
   161	    // The grant is issued before any manifest is seen, so the proposal
   162	    // has zero knowledge: initial_streams == 1. All growth is via resize
   163	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   164	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   165	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   166	    Some(ResponderDataPlane {
   167	        listener,
   168	        session_token,
   169	        epoch0_sub_token,
   170	        initial_streams,
   171	        port,
   172	    })
   173	}
   174	
   175	/// Aggregated destination-side receive result: the write outcome plus
   176	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   177	/// which IS the settled live stream count this end observed. The sf-2
   178	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   179	pub(super) struct ReceiveTotals {
   180	    pub(super) outcome: SinkOutcome,
   181	    pub(super) streams: usize,
   182	}
   183	
   184	/// Live handle to a running responder data plane. The control loop arms
   185	/// resize credentials through [`Self::arm`] and joins the accept loop at
   186	/// `SourceDone` via [`Self::finish`].
   187	pub(super) struct ResponderDataPlaneRun {
   188	    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
   189	    task: AbortOnDrop<Result<ReceiveTotals>>,
   190	    /// The `session_token` half of every socket credential (the control
   191	    /// loop does not need it, but keeping it here documents the shape).
   192	    #[allow(dead_code)]
   193	    session_token: Vec<u8>,
   194	    /// The receiver's advertised `max_streams` — the control loop refuses
   195	    /// a resize that would grow past it (defense in depth; the source's
   196	    /// dial already clamps to the same ceiling).
   197	    pub(super) ceiling: usize,
   198	}
   199	
   200	impl ResponderDataPlane {
   201	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   202	    pub(super) fn grant(&self) -> DataPlaneGrant {
   203	        DataPlaneGrant {
   204	            tcp_port: self.port as u32,
   205	            session_token: self.session_token.clone(),
   206	            initial_streams: self.initial_streams,
   207	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   208	        }
   209	    }
   210	
   211	    /// The epoch-0 stream count this responder granted (always 1 — the
   212	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   213	    /// counter from it.
   214	    pub(super) fn initial_streams(&self) -> u32 {
   215	        self.initial_streams
   216	    }
   217	
   218	    /// Spawn the accept+receive loop and return a live handle. The loop
   219	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   220	    /// socket per armed resize credential until the control loop signals
   221	    /// `SourceDone` (drops the arm sender) and every receive worker has
   222	    /// drained its END. Runs concurrently with the control-stream diff
   223	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   224	    pub(super) fn spawn(
   225	        self,
   226	        sink: Arc<dyn TransferSink>,
   227	        progress: Option<RemoteTransferProgress>,
   228	    ) -> ResponderDataPlaneRun {
   229	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   230	        let session_token = self.session_token.clone();
   231	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
   232	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, progress, arm_rx)));
   233	        ResponderDataPlaneRun {
   234	            arm_tx,
   235	            task,
   236	            session_token,
   237	            ceiling,
   238	        }
   239	    }
   240	
   241	    async fn accept_loop(
   242	        self,
   243	        sink: Arc<dyn TransferSink>,
   244	        progress: Option<RemoteTransferProgress>,
   245	        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
   246	    ) -> Result<ReceiveTotals> {
   247	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   248	        let mut epoch0 = self.session_token.clone();
   249	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   250	
   251	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   252	        let mut total = SinkOutcome::default();
   253	        let mut streams = 0usize;
   254	
   255	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   256	        // grant is always 1; the loop handles N for symmetry).
   257	        for _ in 0..self.initial_streams {
   258	            let socket = accept_authenticated(&self.listener, &epoch0).await?;
   259	            streams += 1;
   260	            spawn_receive(&mut receives, socket, &sink, progress.clone());
   261	        }
   262	
   263	        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
   264	        // whose socket the SOURCE dials right after its ack. `no_more` is
   265	        // set when the control loop drops the arm sender at `SourceDone`;
   266	        // the loop then drains the last armed sockets and workers. Because
   267	        // the SOURCE only dials a credential it was acked for (and a dial
   268	        // failure faults the whole session, aborting this task via
   269	        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
   270	        let mut armed: Vec<Vec<u8>> = Vec::new();
   271	        let mut arm_rx = Some(arm_rx);
   272	        let mut no_more = false;
   273	        loop {
   274	            if no_more && armed.is_empty() && receives.is_empty() {
   275	                break;
   276	            }
   277	            // A closed arm channel resolves `recv()` instantly to `None`
   278	            // every poll; parking it on `pending()` once closed keeps the
   279	            // biased select from starving the accept/join arms (otherwise
   280	            // the None arm wins every race and the loop spins without ever
   281	            // collecting a finished worker).
   282	            let arm_recv = async {
   283	                match arm_rx.as_mut() {
   284	                    Some(rx) => rx.recv().await,
   285	                    None => std::future::pending().await,
   286	                }
   287	            };
   288	            tokio::select! {
   289	                biased;
   290	                // Control FIRST: an arm must register before its socket
   291	                // (which the SOURCE dials only after the ack the control
   292	                // loop sends right after arming), so the accept arm below
   293	                // always sees a populated `armed` set.
   294	                arm = arm_recv => match arm {
   295	                    Some(sub_token) => armed.push(sub_token),
   296	                    // Arm sender dropped at SourceDone: no more resizes.
   297	                    None => {
   298	                        arm_rx = None;
   299	                        no_more = true;
   300	                    }
   301	                },
   302	                // Accept only when a resize credential is armed. `accept`
   303	                // is cancel-safe, so losing this arm to another (its
   304	                // pending connection stays queued) drops no socket. The
   305	                // credential read happens OUTSIDE the select (below) so a
   306	                // select cancel can never truncate a half-read socket.
   307	                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
   308	                    let socket = accepted?;
   309	                    let socket =
   310	                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
   311	                    streams += 1;
   312	                    spawn_receive(&mut receives, socket, &sink, progress.clone());
   313	                }
   314	                joined = receives.join_next(), if !receives.is_empty() => {
   315	                    let outcome = joined
   316	                        .expect("join_next is None only when empty, guarded above")
   317	                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   318	                    total.files_written += outcome.files_written;
   319	                    total.bytes_written += outcome.bytes_written;
   320	                }
   321	            }
   322	        }
   323	        Ok(ReceiveTotals {
   324	            outcome: total,
   325	            streams,
   326	        })
   327	    }
   328	}
   329	
   330	impl ResponderDataPlaneRun {
   331	    /// Arm a resize credential so the next socket presenting
   332	    /// `session_token ‖ sub_token` is accepted. Returns false if the
   333	    /// accept loop is gone (its receiver dropped) — the control loop then
   334	    /// acks the resize as refused.
   335	    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
   336	        self.arm_tx.send(sub_token).is_ok()
   337	    }
   338	
   339	    /// Signal `SourceDone` (no more resizes) and join the accept loop for
   340	    /// the aggregated receive totals.
   341	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   342	        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
   343	        // Dropping the arm sender is the "no more resizes" signal.
   344	        drop(arm_tx);
   345	        task.join()
   346	            .await
   347	            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
   348	    }
   349	}
   350	
   351	/// Spawn one receive worker draining `socket` into `sink` via the shared
   352	/// receive pipeline, guarded by the transfer stall timeout (carried REV4
   353	/// RELIABLE invariant, matching the old push receive: a peer that
   354	/// authenticates then stalls mid-record trips the stall timeout rather
   355	/// than pinning the task until TCP keepalive).
   356	fn spawn_receive(
   357	    receives: &mut JoinSet<Result<SinkOutcome>>,
   358	    socket: TcpStream,
   359	    sink: &Arc<dyn TransferSink>,
   360	    progress: Option<RemoteTransferProgress>,
   361	) {
   362	    let sink = Arc::clone(sink);
   363	    receives.spawn(async move {
   364	        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
   365	        execute_receive_pipeline(&mut guarded, sink, progress.as_ref()).await

exec
/bin/zsh -lc "sed -n '1,120p' docs/STATE.md; sed -n '120,230p' docs/STATE.md" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "rg -n -C 16 \"stream|ramp|worker|parity|settle|epoch|latency\" docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,900p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-15 (52nd handoff — round 11 fixed + round-12 consensus: P1 IS REAL, the Mac↔Mac run is parked; owner to pick direction)

- **NEXT ACTION — OWNER DECISION, then execute it. NO DATA HAS EVER BEEN TAKEN and none is queued.** Round 11 is fully fixed (instrument at `bfae311`, prereg **rev 11**), and the round-12 review — reframed per **D-2026-07-14-5** to ask "is this the best experiment", not "is the code correct per my plan" — reached a **two-reviewer consensus that changes the plan**: read `.review/results/macmac-r12.{codex-design,codex-harness,grok-design}.md` and `.review/results/p1-adjudication-r1.{codex,grok}.md`.
  - **P1 IS REAL — settled by independent adjudication of the RECORDED data (codex + grok, high confidence).** A prior review claimed P1 might be a free-writeback timing artifact of the old harness (`bench_otp12_win.sh` flushes with no settle) and should be re-measured first. **The data refute that:** on `wm_tcp_mixed` the flush is **symmetric** (72 vs 73 ms) against a **~300 ms** effect, the effect is entirely in **transfer time** (remove flush and the ratio *rises*, 1.385→1.417, with zero arm overlap), the **same-fixture gRPC control passes at 1.020** (a writeback artifact would hit it identically), and Linux's identical immediate-flush method shows **no P1**. The precedent both cite: a *real* accounting artifact was caught here once (`2c0af86`) because it polluted the gRPC control — P1 is carrier-specific, so it passes that test. **The release blocker is genuine, not measurement error.**
  - **BOTH REVIEWERS: the Mac↔Mac run is NOT the next move — no outcome of it changes the release-critical action.** It answers only "can P1 occur without a Windows peer?", and every outcome still routes to fixing P1 on the pair where it lives (macOS↔Windows). Grok's power analysis: with four independent full-range controls that must ALL be clean and rig W's fast arm known-bimodal, the *most likely* successful outcome is `CONTROLS-NOT-CLEAN` — a re-run, not an answer.
  - **THE DECISION FOR THE OWNER**: (1) **instrument the TCP dial/accept transfer path on rig W** — both reviewers' recommendation; P1 is now pinned to TCP + destination-initiated + mixed, so add timing spans to `SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`; the fastest route to a fix. Or (2) **run Mac↔Mac anyway** for the 2×2 map — instrument is READY (BLOCKER closed and proved: pointing it at the 1GbE NIC trips three independent gates), but see the consensus above. **No agent may pick this; owner call.**
  - **The Mac↔Mac instrument is DONE and REVIEWED** — engine 40 cases / 19 mutations, harness self-test 0-blind on both Macs, fabric gates proved by mutation. If run: nagatha↔`q`, 10GbE MTU 9000, build `f35702a` (nagatha's worktree + build were MISSING and were rebuilt this session), both Macs codex-quiet and Time Machine off. Host facts: `.agents/machines.md`.
  - **⚠ ROUND-12 STILL-OPEN correctness findings (real, not yet fixed — apply before any Mac↔Mac run):** the threshold `min(src/10, 230)` can report `REPRODUCES` on a cell whose ratio (1.092) *passes* the 1.10 bar (codex BLOCKER — the `min` gives EITHER standard, the prose says BOTH); the end-fabric gate re-checks MSS/IP but **not link speed** (a 10GbE→1GbE renegotiation keeping MTU 9000 grades — my own duplicate-site bug); the `B ≥ T/2` refusal guards only the positive margin, not the smaller `src/11` negative one; two mutations "kill" for the wrong reason. Detail in `macmac-r12.codex-design.md`.
- **THE INSTRUMENT IS THE RISK — ~110 findings across TEN reviews of this ONE harness, all accepted, none rejected, and it has still never run.** Three project claims were already retracted to harness bugs. **TWO DEFECT CLASSES recur in EVERY round; the next review must assume both are present.** (1) **"Fixed the branch I was shown, not the class"** — the same materiality bug escaped **four** rounds; a fail-open `pgrep` was fixed in one gate and left in its duplicate; the drain was fixed by VALUE and left failing by STATUS; Spotlight coerced a non-number to 0 exactly as the drain once accepted `"."`. **And a deletion regressed the build pin**: cutting the escalation block out took the adjacent `EXPECT_SHA` check with it, so any sha — including `.dirty` — was accepted. (2) **"A protection that never executes, or cannot fail"** — `SETTLE_MS` **had never run in any revision** (a quoting bug killed the `sleep` and its status was discarded), while the prereg asserted it for three revisions; the ssh-dispatch **bound** was measured once at preflight and never enforced on a run. Earned rules: **verify the instrument before believing the measurement**; **`bash -n` is not an execution**; **a protection that cannot be observed is not a protection**; **a mutation that cannot be killed is not a proof.**
- **⚠ THE MAC↔MAC RIG IS *NOT* AN H1 DISCRIMINATOR — retracted 2026-07-14.** "Reproduces ⇒ H1 dies" was **WRONG**: H1 accuses **blit's own code paths**, not Windows, and that code runs on macOS too — so a reproduction is *consistent with* H1. It answers one thing, scoped to this pair: **can P1 occur WITHOUT a Windows peer?** A reproduction ⇒ P1 is not waivable as "Windows residue" (it does **not** prove a platform-*general* cost, and leaves macOS/APFS and host×role open). A null ⇒ it did not reproduce *on this pair* — consistent with "Windows required", **not proof** of it, and reportable only if the run could have SEEN the effect. Detail: the pre-registration.
- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488 as of `bb28ddd`** — the last commit to touch `crates/`+`proto/`; every commit since is docs/scripts, so the count stands unre-run. SMALL_FILE_CEILING paused (D-2026-07-05-1).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go").** The invariant (plan doc,
  verbatim): ONE block of transfer code; direction/initiator/verb can
  NEVER affect wall time by blit's doing — impossible by construction
  because the per-direction drivers and `Push`/`PullSync` are deleted
  at cutover. Slices otp-1..13; converge-up per cell (±10%);
  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.**
  - **Slices otp-1 … otp-11 are all `[x]` CLOSED** — the session
    machine, the baselines, the cutover deletion (−13.8k lines) and
    otp-11b's deletion of the old orchestration (−6.2k). The
    deletion-proof acceptance line COMPLETES. The closed-slice record
    was rotated verbatim to `docs/history/state-archive.md`
    (2026-07-14 drift); per-slice detail lives in DEVLOG + `.review/`.
  - **Open: otp-12d and otp-13** — both DEFERRED behind pf-final, see
    Queue 1.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
  in Blocked); the codex loop governs all changes (D-2026-07-04-1).

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
   gap shapes recorded for the walk
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
   pre-fix new arms for acceptance. Assembling the acceptance matrix now
   would build otp-13's artifact from void rows.
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
   reviewloop slice by slice"; implementation proceeds, each slice
   through the codex loop).
   Two experiments come BEFORE any code; both docs own their detail.
   **(i) The A-B-B-A MTU run on `q` — `[x]` DONE 2026-07-14: MTU KILLED**
   (`r = −3.1%`; `docs/bench/otp12-jumbo-win-2026-07-13/`). See the pf-0
   bullet at the top for the two limits it puts on pf-1.
   **(ii) THE MAC↔MAC RIG — the missing cell of the 2×2** (owner,
   2026-07-13). Linux↔Linux = **no P1** (8/8 PASS); macOS↔Windows = **P1**
   (1.237/1.300/1.385/1.362); macOS↔macOS = **?** Design, decision rule and
   the retraction of the "H1 dies" framing: **see NEXT ACTION at the top**
   and the rev-2 pre-registration. **Both Macs are bench ENDS: the codex
   loop CANNOT run during the session** (the gate enforces it).
   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
   *cross-direction* miss for a cell that ALREADY passes invariance — P1
   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
   acceptance criterion 1.** Not assumed either way. P2
   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
   → pf-final (ALL rigs) → otp-12d → otp-13.**
1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
   path, opposite directions: a fidelity fix ADDS per-file work to a path
   already losing to robocopy, so planning them apart optimises one against
   the other). Both docs own their detail; do not restate it here.
   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
     — Windows attributes + ADS silently dropped, exit 0, **both routes
     (measured)**; loss is **conditional on file count**
     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
     apply **does not scale** (8 workers buy 1.05×; robocopy gets ~2.2× from 8
     threads) and ships **one** worker. At EQUAL concurrency blit BEATS
     robocopy; at 8-vs-8 it loses 1.9×. `docs/bench/win-local-ab-2026-07-13/`.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   absorbed by otp-2/otp-12's rig matrices.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   likely landed inside otp-6's one-delete-rule slice; re-check first).
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
6. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
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

- **The Mac↔Mac run is BLOCKED and NOT clearable by an agent** — round 11's
  findings are unfixed (engine 2 HIGH, harness 1 BLOCKER + 4 HIGH) and both
  Macs must be codex-quiet. Basis and detail: NEXT ACTION at the top of this
  file; never restated here (re-verified 2026-07-14 against
  `.review/results/macmac-harness-r11.*` and `git log -- scripts/bench_otp12pf_mac.sh`,
  whose newest commit is still round 10's `8997f92`).
- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
  `.agents/machines.md`.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
  evidence, NOT acceptance evidence; Queue 1a):
  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)

## Open questions

- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
  592) describes the deleted `determine_remote_tuning` — fix folded
  into **w10-docs-batch**; no one-off edit.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; **windows-latest CI has never been
  observed green — check it live, do not record push state here.**
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-15 (52nd)** — **Round 11 FIXED (7 commits, each guard-proved; instrument at `bfae311`,
  prereg rev 11), round 12 REVIEWED, and the reframe changed the plan.** Framed per D-2026-07-14-5
  around the end goal, codex + grok BOTH said **DO NOT RUN** Mac↔Mac; codex raised a would-be
  BLOCKER (P1 measured with an un-settled harness — re-measure first). **I relayed that as a pivot
  WITHOUT checking the data; owner refused and demanded consensus.** Adjudication (both, from the
  CSVs): **P1 IS REAL** — flush symmetric (72/73 ms) vs a ~300 ms effect in *transfer* time, gRPC
  control passes at 1.020, Linux no-P1. The release blocker is genuine. D-2026-07-14-4 (`B≥T/2`
  refuses) and D-2026-07-14-5 (first-review reframe) recorded. Also: rebuilt nagatha's missing
  `f35702a` worktree+binaries. No crates/proto, no rig time, no data. Full: **DEVLOG 02:30Z**.
- **2026-07-14 (51st)** — **BOTH MACS CONFIRMED READY (owner); DEVLOG backfilled for rounds
  7–11. No code, no rig time, no data.** TM autobackup = 0 on both; zero `blit-daemon`. A ready rig
  is not a ready instrument — round 11's BLOCKER stood (now fixed, 52nd). Full: **DEVLOG 22:45Z**.
- **2026-07-14 (50th, `f933097`)** — **`drift`: STATE hygiene.** Handoff log was four rounds stale.
  Created `docs/history/state-archive.md`, anchored `Suite 1488 as of bb28ddd`. Full: **DEVLOG 21:10Z**.
- *(49th and earlier pruned to the cap — full entries in DEVLOG 2026-07-06..15.)*

 succeeded in 0ms:
1-# otp-12 perf findings — investigate + fix before acceptance (design)
2-
3-**Status**: Active
4-**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
5-**"one more round with codex on the plan then just write the code and
6-reviewloop slice by slice. that converges faster than plans with no
7-ground truth to test."** The final round ran (round 5, verdict NOT READY,
8-3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
9-counterfactual, F3 the inexecutable decision rule); all three are fixed
10-in this revision, and implementation now proceeds **slice by slice, each
11-through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
12-verdict is no longer a gate — the plan's earlier "flip to Active at codex
13-convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
14-were increasingly finding defects in the *prose* while the plan's central
15:factual claim was settled by *measurement* (the same-OS rig refuted a
16-claim four review rounds had left standing). pf-1 exists to generate
17-ground truth; it starts now.
18-
19-**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
20-assumed by this plan):** P1 has **no escape hatch on the books**.
21-D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
22-that is *already* invariance-passing; P1 is the invariance failure
23-itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
24-plan pursues) or the owner must **amend acceptance criterion 1** in a new
25-decision. pf-1 proceeds either way: it produces the evidence that
26-decision would rest on.
27-**Created**: 2026-07-12
28-**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
29-say the quiet part: "Unification that slows the fast direction fails
30-review." P1 is a miss of the parent's HEADLINE acceptance criterion
31-(initiator/verb invariance, ±10%) — not a nice-to-have.
32-**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
33-if an investigation slice needs one, it stops and this doc is amended
34-through the loop first.
35-
36-**Sequencing (corrected 2026-07-13).** This doc originally deferred
37-otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
38:fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
39-at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
40-That does not change this plan's standing, and the rows are not lost
41-work — under `pf-final` they are **pre-fix rows, void for acceptance**,
42-but they serve two real purposes: (a) an **independent replication** of
43-both findings at the shipped sha (below), which is exactly the
44-independent corroboration the round-2 review said P1 lacked; and (b) the
45-pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
46-deferred** until P1/P2 are fixed or explained at code level — assembling
47-an acceptance matrix out of pre-fix rows would build the artifact otp-13
48-walks from rows this plan declares void.
49-
50-## The two findings (evidence, both committed)
51-
52-**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
53-(`docs/bench/otp12-win-2026-07-12/`, replicated in
54-`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
--
172-> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
173-> never by verb — the otp-2w rule, re-learned). The retraction is recorded
174-> rather than quietly overwritten because the wrong number was reported to
175-> the owner and briefly drove this plan.
176-
177-### The residual confound (WHICH code) still needs a counterfactual
178-
179-Breaking platform-vs-code does NOT tell us *which* layout property costs
180-the time. On any two-host rig, host identity remains welded to role, so
181-"the accepting end" cannot be separated from "that host" by more runs:
182-
183-- **pf-1 must compare all four rig-W arms** (both cells × both
184-  initiators), not two, and report the interaction — not a single ratio.
185-- **The disambiguator is a dial/accept inversion counterfactual, not a
186-  rig** — but it is **NOT sufficient on its own** (review round 5): the
187-  inversion swaps the source's `Accept`, the destination's `Dial`, AND
188:  the epoch-0 topology **simultaneously**, so a positive result implicates
189-  *the topology pair*, not H1 specifically. It cannot distinguish
190-  source-accept serialization from synchronous destination dialing
191-  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
192-  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
193-  1. **dial/accept inversion** — same direction, same hosts, same fixture;
194-     only who dials changes. Implicates the topology pair (or exonerates it).
195:  2. **no-resize / pre-opened streams** — force the final stream count at
196:     epoch 0 so no resize epoch ever fires. If the gap survives with zero
197-     resizes, H1's resize-specific mechanism is **KILLED** regardless of
198-     what (1) shows (and note `dial.rs:474`: all three fixtures already
199:     target 8 streams, so resize *count* was never the discriminator).
200-  3. **per-side ordering** — hold the topology fixed and vary only whether
201-     the destination's dial-before-ACK is synchronous. Separates the two
202-     halves the inversion conflates.
203-  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
204-  across (1) AND survives (2); it is KILLED if the gap persists with no
205-  resizes, or if (3) shows the cost is the synchronous dial rather than the
206-  accept branch. Any of these that changes connection topology — (1) and
207-  (2) do — **trips this plan's Contract stop-and-amend rule**
208-  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
209-  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
210-  surface is created.
211-  **H1 is also WEAKENED by the Linux null** (it predicts a layout cost that
212-  did not appear on a real-network same-OS pair), so pf-1 must be prepared
213-  to kill it and fall through to H5/H6/H7.
214-- **The same-platform loopback run is a ONE-WAY test** (corrected — an
215-  earlier draft of this section had it backwards). A dest-initiator
216-  penalty that still appears on Mac↔Mac loopback proves **pure layout**
217-  (code). Its ABSENCE proves **nothing**: loopback has no NIC, near-zero
218:  RTT and a huge MTU, so it erases exactly the per-epoch accept/dial
219-  round-trip cost H1 accuses. A negative local result is **INCONCLUSIVE**
220-  and never reads as "no code bug" — it escalates to the inversion
221-  counterfactual and the rig-side instrumented run (Method 2).
222-
223-This refines rather than weakens H1: H1 accuses the **source's accept
224-branch** under resize, and the source in the slow arm is Windows —
225-consistent. But consistency is not confirmation, and the confound above
226-is exactly why pf-1 exists.
227-
228-The rest of the signature is unchanged and sharp:
229-- **carrier**: TCP only — `wm_grpc_mixed` **1.021 PASS** (12b: 1.013);
230-- **fixture**: mixed only — `wm_tcp_large` **1.039** and `wm_tcp_small`
231-  **1.027** both PASS;
232-- **isolation**: in 12c-win, 11 of 12 invariance cells pass at
233-  1.003–1.044. `wm_tcp_mixed` is the sole outlier, by a wide margin.
234-
--
245-|---|---|---|---|---|
246-| 12a zoey (RUNS=8, tight) | `e757dcc` old arm | — | — | **1.105** |
247-| 12b netwatch-01 (3–4% spreads) | `e21cf84` | 2080 | 1811 | **1.149** |
248-| 12c-win (2026-07-13) | `f35702a` (cutover) | 1975 | 1644 | **1.201** |
249-
250-**gRPC small push did NOT regress** (correction, review round 2: the
251-earlier "win 0.98-ish per cells" was wrong against the committed CSVs;
252-range corrected again in round 3). `push_grpc_small` new-vs-old,
253-same-session / committed:
254-
255-| rig | same-session | committed |
256-|---|---|---|
257-| zoey | **1.001** | 0.907 |
258-| netwatch-01 (12b) | **0.801** | 0.835 |
259-| netwatch-01 (12c-win) | **0.852** | 0.802 |
260-
261:So the cross-rig range is **0.801–1.001**: gRPC small push is at parity
262-on zoey and materially FASTER on Windows. The honest statement is **"TCP
263-regressed while gRPC did not"** — not "gRPC is uniformly faster".
264-
265-That asymmetry is the finding's sharpest constraint on mechanism:
266-whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
267-small-file-heavy (10k×4 KiB). **But it is a constraint, not a proof of
268-innocence** (review round 3): an aggregate gRPC *improvement* cannot
269-exclude a shared regression on both carriers that a larger
270-gRPC-specific gain simply masks. Shared controller/planner/sink code is
271-therefore NOT exonerated by the gRPC numbers, and pf-1 must attribute
272-the TCP gap to a named delta rather than infer "TCP-only ⇒ not shared".
273-
274-Cross-block note (12b README): block-2 `mw_tcp_small` mac_init measured
275-1922 vs block-1 new 2080 in the same session — the only mechanical
276-difference is block-2's precreated destination container and per-arm
277-path shapes; the investigation must confirm or kill that lead. It is a
--
367-`bench_otp12_win.sh:105` (re-records on `0f922de`); rig Z
368-`bench_otp12_zoey.sh:102` (re-records on a **clean `e757dcc` pair** — its
369-original daemon was a *dirty* `731023b` whose committed code is identical, so a
370-clean rebuild is not a new reference build). Rig D has no old baseline and is
371-unaffected. The remaining constraints (`BASELINE_SUMMARY` stays override-free and
372-is re-pointed by a reviewed source edit; pf-0's start-AND-end MSS gate applies)
373-live in D-2026-07-14-1 and are not restated here.
374-
375-Same-session references (`old_session`) are MTU-matched by construction and were
376-never at risk.
377-
378-## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)
379-
380-- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
381-  connection-initiating end DIALS; byte direction is role-set
382-  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
383:  session the SOURCE is the responder: each sf-2 resize epoch is
384-  ACCEPTED off the source's listener while the DESTINATION dials
385-  (otp-5b-2: `SourceSockets` Dial/Accept branches;
386:  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
387-  accept/dial round-trips or serialization in the accept branch that the
388-  dial branch does not pay.
389-  **⚠ H1 ACCUSES CODE, NOT A PLATFORM (canonical; added 2026-07-14 after the
390-  shorthand misled two sessions).** The word "Windows" appears nowhere above.
391-  Windows is merely *who happens to be the accepting source* in P1's slow arm on
392-  rig W, so other docs say "H1's Windows accept branch" as **shorthand for where
393-  the accused code runs on that rig** — it is NOT a claim that H1 requires
394-  Windows. Two consequences, both load-bearing: (a) **a reproduction of P1 on a
395-  non-Windows pair does NOT kill H1** — the accused code runs there too, so it is
396-  *consistent with* H1 (and "consistent with H1" is not confirmation, below);
397-  (b) **a disappearance of P1 without Windows does not CONFIRM H1** either — it
398-  would only mean the accused cost is platform-conditional, which is a further
399:  claim. Only the dial/accept inversion counterfactual in pf-1 can settle H1.
400-  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
401-  was "mixed exercises resize hardest", but **all three fixtures target
402:  eight streams before clamping** (`src/dial.rs:474`) — so resize
403-  *count* cannot explain mixed-only behaviour, and H1 must name what
404-  about mixed differs (shard-boundary timing? the tar-shard small half
405:  interleaving with the big-file stream at the moment epochs fire?) or
406-  be killed. **H1 also names the wrong half without proof**: it accuses
407-  `Accept` while the destination's **synchronous dial-before-ACK** path
408-  (`transfer_session/mod.rs:3113`) is an equally good suspect. pf-1 must
409-  separate them with the dial/accept inversion counterfactual below —
410-  "consistent with H1" is not confirmation.
411-- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
412-  interleave cannot happen — resize begins only after
413-  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
414-  layouts drain the same fixed 128-entry destination need loop, so
415-  batch emission cannot interleave with the resize controller during
416-  manifest/need emission in either layout. Kept only as a residual: if
417-  pf-1 timing shows a layout-dependent need-batch delta anyway, the
418-  mechanism must be re-derived from the trace, not from this text.
419-- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
420-  already killed its named candidates (the small half is tar-sharded and
421-  written with parallel per-file `create_dir_all`/`fs::write`, NO
--
424-  the same served sink, so fsync/flush policy and progress emission are
425-  NOT old/new deltas). What was left — "dest-side directory work/handle
426-  churn" — **names no old/new code delta at all**, and its only probe
427-  (precreate-vs-not) is explicitly environmental and cannot attribute
428-  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
429-  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
430-  shrug. It is therefore retired, and its one code-attributable
431-  descendant — a per-member cost on the TCP receive path that old push
432-  did not pay — lives on as **H6**, which names an executed-path delta.
433-  H3 may only be revived if the pf-1 trace names a concrete old/new
434-  delta in the destination directory/handle path; the 12b cross-block
435-  precreated-container lead (8%, NTFS) is recorded as an environmental
436-  lead for that trace, not as an attribution.
437-- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
438-  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
439-  earlier `dial.rs` attribution was wrong), and old small push ALSO
440:  opened at one stream (after its 128-file early flush) then resized
441-  live — so neither framing nor "fixed-count opening" discriminates.
442:  What survives of H4 is ramp cadence/shard-boundary timing only, and
443-  it is subordinate to H5.
444-- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
445-  scan/diff/transfer overlap on the TCP plane — current code withholds
446-  every TCP payload until `ManifestComplete`
447-  (`transfer_session/mod.rs`), while old push negotiated and queued
448-  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
449:  gRPC's in-stream carrier did not change comparably — which matches
450-  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
451:  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
452-  review round 3). NOTE: an H5 fix
453:  reorders session phases and multi-ADD/pipelined epochs conflict with
454-  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
455-  ordering), so any H5 fix triggers this plan's Contract
456-  stop-and-amend rule BEFORE implementation.
457-- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
458-  need-claim locking on the TCP receive plane — TCP receive
459-  (`NeedListSink`) takes a separate mutex/hash-set claim per member
460-  (`transfer_session/data_plane.rs:1167`), while the gRPC path claims
461-  a whole shard under one lock (`transfer_session/mod.rs:3047`).
462-  TCP-only and per-member (so small-file-heavy) — matches the P2
463-  signature independently of H5. Discriminated by the pf-1 per-member
464-  locking timings (Method 3(e), now unconditional).
465-  **Historical control — corrected (review round 3): test the EXECUTED
466-  path, not source presence.** `NeedListSink` *exists* in the tree at
467-  `0f922de`, so "does the symbol exist there" is the wrong question and
468-  would wrongly force H6 into a "multiplied claim frequency" story. What
469-  matters is what old push actually RAN: at `0f922de` the served push
--
499-  **task-local map and handled need batches inline**, with no lock and no
500-  channel hop per entry. This is **per-entry**, so it scales with FILE
501-  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
502-  **shared by BOTH carriers**. That is the precise class the round-3
503-  gRPC caveat warned about: a shared regression can hide under gRPC's
504-  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
505-  shared code. No prior hypothesis tested it. Discriminated by: per-entry
506-  bookkeeping timings scaled against file count, plus the wall-time
507-  counterfactual (a task-local/batch-inline path behind a debug flag).
508-  H7 and H6 are independent and may BOTH contribute.
509-
510-## Method (the investigation slice — no behavior changes)
511-
512-1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
513-   in-process/two-process rigs on the Mac with the otp-2 fixture
514-   shapes; `--trace-data-plane` + targeted `tracing` spans (added
515:   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
516-   ack), need-batch emission times, per-file sink open/write/close in
517-   the receive path, shard planner in/out timestamps.
518-2. **A/B the role layouts in one process**: the role suite already
519-   runs both initiator layouts over identical fixtures (otp-3) — but
520:   it forces the in-stream carrier (`transfer_session_roles.rs`), so
521-   the timing-harness variant MUST add a TCP-carrier mode; it reports
522-   phase timings per layout for mixed and small fixtures. A positive
523-   layout-dependent delta in a named phase confirms; local ABSENCE
524-   does not kill H1 (loopback removes the Windows↔Mac topology). So
525-   that H1 stays falsifiable: if the local run is negative, pf-1
526-   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
527-   CELLS fixtures) before pf-1 may close — every hypothesis exits
528-   pf-1 confirmed or killed, never "unfalsified" (review round 2).
529-3. **Historical control, then bisect P2**: old push is deleted from
530-   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
531-   build and run; the control is an old-vs-new run on identical
532-   fixtures. The new tracing spans do NOT exist in `0f922de` (review
533-   round 2), so the control is observed externally — phase boundaries
534-   from wire + filesystem timestamps and stdout progress, with event
535-   semantics mapped span-for-span to the new names — or, where that is
536-   too coarse, a minimal probe backport onto the pinned `0f922de`
537-   source with identical event names. Either way every timed
538-   configuration runs an instrumentation-on/off pair to bound observer
539-   overhead (per-member tracing across ~10k files can perturb a
540-   double-digit share of the measured gap). Experiments, corrected per
541-   review 2026-07-12: (a) precreate-vs-not stays but is
542-   environmental-only (it cannot attribute code); (b) the flush/
543-   instrument toggles missed the tar-shard path — instrument the
544-   tar-shard write path itself; (c) REPLACED (review round 2) — the
545:   ramp pin discriminated nothing (old push also opened at one
546:   stream), but H4 keeps a code-level counterfactual: a batch-cadence
547-   replay toggle that processes need batches at the recorded old-push
548-   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
549-   metric DEFINED (review round 2: "manifest-complete→first-payload
550-   gap" was underdefined, and for old push the quantity is expected to
551-   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
552-   run, on ONE common clock with a SIGNED offset from the
553-   `ManifestComplete` event, three separately-named events on the
554-   source side plus one on the destination:
555-   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
556-   the send queue); `t_first_socket_write` (first byte handed to the
557-   TCP data plane); `t_first_payload_received` (destination side —
558-   requires the two clocks to be reconciled, so record the ssh/NTP
559-   offset per run and report it with the number, or state that the
560-   destination event was not usable). The overlap DIFFERENCE is
561-   established only if `t_first_socket_write − t_manifest_complete` is
562-   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
--
695-  pre-pf-1 evidence.
696-- **pf-2..n**: one fix slice per confirmed root cause (smallest
697-  change that moves the phase timing; A/B'd locally before rig time).
698-- **pf-final**: NOT just the two escalation cells — the final build
699-  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
700-  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
701-  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
702-  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
703-  (corrected, review round 2 — "every row" was impossible: the
704-  same-session `old` arms and the committed baselines are OLD builds by
705-  construction, which is the entire point of a reference). Pre-fix
706-  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
707-  which are **replication and control evidence, not acceptance
708-  evidence**.
709-  **Rig D is included even though it is not a suspect (review round
710-  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
711:  would leave the parent plan's **delegated-parity bar**
712-  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
713-  evidence at all. "Not implicated" scopes what pf-1 must
714-  *instrument* — it does not waive an acceptance bar. Rig D's TCP
715-  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
716-  both arms are new-build by construction there (rig D has no old
717-  baseline), so the whole cell is re-measured.
718-  **Every gRPC row the acceptance method requires reruns
719-  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
720-  earlier "if shared code changed, the gRPC cells rerun too" left the
721-  decision to the author's own judgement of what counts as shared, which
722-  is exactly the loophole H7 exploits: a shared regression can hide under
723-  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
724-  complete Z/W gRPC converge and invariance rows, so those are
725-  final-build rows, full stop — no conditional. Results land in fresh
726-  dated evidence dirs. **Then** otp-12d assembles the matrix from
727-  final-build rows, and the otp-13 owner walk reads it.
--
734-  source/binaries diff and run fine — historical claims get live
735-  controls in pf-1, not pin-archaeology.
736-- zoey never measured P1: its rig anchors converge-up only, so there
737-  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
738-  says nothing about layout asymmetry (review 2026-07-12). pf-1's
739-  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
740-  qualifies per the 12b wm numbers).
741-- **The 12c-win rows are replication, not acceptance** (2026-07-13).
742-  They are pre-fix by definition, so `pf-final` voids them for
743-  acceptance; their value is that they (a) reproduce P1 and P2 on an
744-  independent session at the shipped sha, (b) supply the
745-  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
746-  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
747-  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
748-  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
749-  P2 1.149→1.201), so neither is drifting toward the bar on its own.
750:- **Rig-D delegated parity is not a SUSPECT, but it is still an
751-  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
752-  delegated-vs-direct matrix passed 7/7
753-  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
754-  measurable cost and pf-1 need not instrument the delegated trigger
755-  path. That is a statement about *where to look for the bug* — it does
756:  **not** waive the parent plan's delegated-parity bar, whose evidence
757-  is pre-fix and therefore void under pf-final. Rig D reruns on the
758-  final build (see pf-final).

exec
/bin/zsh -lc 'git show a76b785''^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'1385,1515p'; git show a76b785"'^:crates/blit-core/src/transfer_session/mod.rs | nl -ba | sed -n '"'1755,1865p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-PpqKnFhy' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Xp4X7xP4' (errno=Operation not permitted)
  1385	    // Payload phase. The byte carrier is either the TCP data plane
  1386	    // (dialed above) or the in-stream record grammar (fallback). Needs
  1387	    // accumulated while a batch was being sent become the next planner
  1388	    // batch (contract §Transport selection); payloads only flow after
  1389	    // ManifestComplete.
  1390	    // The in-stream carrier reuses one read buffer across records; the
  1391	    // data plane owns its own pooled buffers, so skip that allocation.
  1392	    let mut read_buf = if data_plane.is_none() {
  1393	        vec![0u8; IN_STREAM_CHUNK]
  1394	    } else {
  1395	        Vec::new()
  1396	    };
  1397	    loop {
  1398	        drain_ready_source_events(
  1399	            &mut events,
  1400	            &mut pending,
  1401	            &mut resume,
  1402	            &mut need_complete,
  1403	            &mut needed_bytes,
  1404	            &mut needed_count,
  1405	            data_plane.as_ref(),
  1406	            tx,
  1407	            &mut pending_resize,
  1408	        )
  1409	        .await?;
  1410	        if !pending.is_empty() {
  1411	            let batch = std::mem::take(&mut pending);
  1412	            match &mut data_plane {
  1413	                Some(dp) => {
  1414	                    // sf-2: correct the stream count toward the shape the
  1415	                    // accumulated need list implies before queueing this
  1416	                    // batch (one ADD per epoch; a no-op while one is in
  1417	                    // flight or the shape wants no more).
  1418	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1419	                        .await?;
  1420	                    let payloads =
  1421	                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1422	                    // A cancel while earlier batches are actively moving
  1423	                    // closes the send pipeline under backpressure, so this
  1424	                    // queue fails with a data-plane error — prefer the
  1425	                    // peer's framed reason (CANCELLED) the same way the
  1426	                    // finish() drain does (otp-4b-3 codex F1). Not raced
  1427	                    // against events like finish(): live `Need`s still
  1428	                    // arrive here, and `recv_peer_fault` would consume them.
  1429	                    if let Err(dp_err) = dp.queue(payloads).await {
  1430	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1431	                    }
  1432	                }
  1433	                None => {
  1434	                    // codex otp-8 F1: race the record sends against the
  1435	                    // receive half's fault signal — the in-stream twin of
  1436	                    // the data-plane drain's `recv_peer_fault` arm. A peer
  1437	                    // cancel (framed CANCELLED, then RPC teardown) must
  1438	                    // interrupt a send blocked in `reader.read()` or in
  1439	                    // flow-controlled `tx.send()` and surface the framed
  1440	                    // reason, not hang or decay to INTERNAL. Biased:
  1441	                    // when both are ready, the framed fault wins.
  1442	                    tokio::select! {
  1443	                        biased;
  1444	                        fault = peer_fault_signalled(&mut fault_signal) => {
  1445	                            return Err(eyre::Report::new(fault));
  1446	                        }
  1447	                        res = send_payload_records(
  1448	                            tx,
  1449	                            &source,
  1450	                            plan_options,
  1451	                            batch,
  1452	                            &mut read_buf,
  1453	                            instruments.progress.as_ref(),
  1454	                        ) => {
  1455	                            res?;
  1456	                        }
  1457	                    }
  1458	                }
  1459	            }
  1460	            continue;
  1461	        }
  1462	        if !resume.ready.is_empty() {
  1463	            // The block phase for correlated (need, hash-list) pairs.
  1464	            // Data plane (otp-7b): each pair becomes ONE composite
  1465	            // ResumeFile work item, so one pipeline worker runs the
  1466	            // whole record on one socket — strict per-file serialization
  1467	            // without cross-socket reorder hazards. In-stream (otp-7a):
  1468	            // control-lane BlockTransfer/Complete frames, as before.
  1469	            let ready = std::mem::take(&mut resume.ready);
  1470	            match &mut data_plane {
  1471	                Some(dp) => {
  1472	                    // codex 7b-1 F4: resume batches drive the sf-2 shape
  1473	                    // correction exactly as plain batches do — a
  1474	                    // resume-heavy need list must not stay pinned to the
  1475	                    // zero-knowledge single stream.
  1476	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
  1477	                        .await?;
  1478	                    let payloads = ready
  1479	                        .into_iter()
  1480	                        .map(|(header, hashes)| TransferPayload::ResumeFile {
  1481	                            header,
  1482	                            block_size: hashes.block_size,
  1483	                            dest_hashes: hashes.hashes,
  1484	                        })
  1485	                        .collect();
  1486	                    // Same cancel posture as the plain-batch queue above:
  1487	                    // prefer the peer's framed reason over the transport
  1488	                    // break a cancel also causes (otp-4b-3 codex F1).
  1489	                    if let Err(dp_err) = dp.queue(payloads).await {
  1490	                        return Err(prefer_peer_fault(&mut events, dp_err).await);
  1491	                    }
  1492	                }
  1493	                None => {
  1494	                    for (header, hashes) in ready {
  1495	                        // codex 7b-2 G2: the whole in-stream record names
  1496	                        // its file on failure, matching the data-plane
  1497	                        // carrier's outer wrap. Same fault race as the
  1498	                        // plain-batch send above (codex otp-8 F1).
  1499	                        tokio::select! {
  1500	                            biased;
  1501	                            fault = peer_fault_signalled(&mut fault_signal) => {
  1502	                                return Err(eyre::Report::new(fault));
  1503	                            }
  1504	                            res = send_resume_block_records(
  1505	                                tx,
  1506	                                &source,
  1507	                                &header,
  1508	                                &hashes,
  1509	                                instruments.progress.as_ref(),
  1510	                            ) => {
  1511	                                res.map_err(|e| tag_path(e, &header.relative_path))?;
  1512	                            }
  1513	                        }
  1514	                    }
  1515	                }
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EUCXFNFH' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-O0IKeIa4' (errno=Operation not permitted)
  1755	        SourceEvent::ResizeAck(ack) => {
  1756	            let dp = data_plane.ok_or_else(|| {
  1757	                eyre::Report::new(SessionFault::protocol_violation(
  1758	                    "DataPlaneResizeAck on a session with no data plane",
  1759	                ))
  1760	            })?;
  1761	            // Match the ack to the in-flight proposal; stale/unsolicited
  1762	            // acks (wrong epoch, or none pending) are ignored, matching
  1763	            // old push. `take()` + restore keeps the borrow simple.
  1764	            let pending_r = match pending_resize.take() {
  1765	                Some(p) if p.epoch == ack.epoch => p,
  1766	                restored => {
  1767	                    *pending_resize = restored;
  1768	                    return Ok(());
  1769	                }
  1770	            };
  1771	            if ack.accepted {
  1772	                dp.add_stream(&pending_r.sub_token).await?;
  1773	                dp.dial()
  1774	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1775	            } else {
  1776	                dp.dial()
  1777	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1778	            }
  1779	            // Ramp one stream per accepted epoch: propose the next ADD.
  1780	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1781	        }
  1782	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1783	            "TransferSummary before SourceDone",
  1784	        ))),
  1785	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1786	    }
  1787	}
  1788	
  1789	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1790	/// the stream count the accumulated need list implies, if none is in
  1791	/// flight. A no-op when the shape wants no more than the live count (the
  1792	/// dial returns `None`). Sends the frame and records the in-flight
  1793	/// proposal for the ack to match.
  1794	async fn maybe_propose_resize(
  1795	    dp: &data_plane::SourceDataPlane,
  1796	    tx: &mut Box<dyn FrameTx>,
  1797	    needed_bytes: u64,
  1798	    needed_count: usize,
  1799	    pending_resize: &mut Option<data_plane::PendingResize>,
  1800	) -> Result<()> {
  1801	    if pending_resize.is_some() {
  1802	        return Ok(());
  1803	    }
  1804	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1805	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1806	            op: DataPlaneResizeOp::Add as i32,
  1807	            epoch: proposal.epoch,
  1808	            target_stream_count: proposal.target_streams,
  1809	            sub_token: proposal.sub_token.clone(),
  1810	        })))
  1811	        .await?;
  1812	        *pending_resize = Some(proposal);
  1813	    }
  1814	    Ok(())
  1815	}
  1816	
  1817	/// Block for the ack of the one in-flight resize and dial its socket (or
  1818	/// settle it refused). Does NOT propose further — it resolves exactly the
  1819	/// pending proposal so the destination's armed slot is consumed before we
  1820	/// finish the data plane.
  1821	async fn resolve_in_flight_resize(
  1822	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1823	    dp: &data_plane::SourceDataPlane,
  1824	    pending: data_plane::PendingResize,
  1825	) -> Result<()> {
  1826	    loop {
  1827	        match events.recv().await {
  1828	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1829	                if ack.accepted {
  1830	                    dp.add_stream(&pending.sub_token).await?;
  1831	                    dp.dial()
  1832	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1833	                } else {
  1834	                    dp.dial()
  1835	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1836	                }
  1837	                return Ok(());
  1838	            }
  1839	            // A stale ack for an already-settled epoch: ignore, keep
  1840	            // waiting for ours.
  1841	            Some(SourceEvent::ResizeAck(_)) => continue,
  1842	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1843	            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
  1844	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1845	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1846	                )))
  1847	            }
  1848	            Some(SourceEvent::BlockHashes(l)) => {
  1849	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1850	                    format!(
  1851	                        "BlockHashList for '{}' after NeedComplete resolved every resume need",
  1852	                        l.relative_path
  1853	                    ),
  1854	                )))
  1855	            }
  1856	            Some(SourceEvent::NeedComplete) => {
  1857	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1858	                    "duplicate NeedComplete",
  1859	                )))
  1860	            }
  1861	            Some(SourceEvent::Summary(_)) => {
  1862	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1863	                    "TransferSummary before SourceDone",
  1864	                )))
  1865	            }

codex
The parity assertions and zero-capacity interpretation are wired correctly, and the test inventory is unchanged across the commit. I’ve found one concrete refusal-state gap and one startup/stall risk; I’m doing a focused test run and final severity check before reporting.
exec
/bin/zsh -lc 'cargo test -p blit-core --test transfer_session_roles many_tiny_files_reach_shape_target -- --nocapture' in /Users/michael/Dev/blit_v2
 succeeded in 11434ms:
   Compiling blit-core v0.1.0 (/Users/michael/Dev/blit_v2/crates/blit-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.02s
     Running tests/transfer_session_roles.rs (target/debug/deps/transfer_session_roles-a5a0e2d3c2f7aafe)

running 2 tests
Manifest enumeration complete in 421.99ms (10000 entries)
Manifest enumeration complete in 418.64ms (10000 entries)
test many_tiny_files_reach_shape_target_when_destination_initiates ... ok
test many_tiny_files_reach_shape_target_when_source_initiates ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 5.77s


exec
/bin/zsh -lc "cargo test --workspace -- --list 2>/dev/null | awk '/: test"'$/{n++} END{print n+0}'"'" in /Users/michael/Dev/blit_v2
 succeeded in 284ms:
0

exec
/bin/zsh -lc "cargo test -p blit-core -- --list | sed -n '1,30p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
error: failed to open: /Users/michael/Dev/blit_v2/target/debug/.cargo-build-lock

Caused by:
  Operation not permitted (os error 1)

codex
- [crates/blit-core/src/transfer_session/mod.rs:1423](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1423) — **HIGH** — Payload dispatch now waits for every one-stream resize epoch serially. A 1→16 ramp costs 15 control RTTs plus connects; meanwhile receive workers’ 30-second StallGuards are already running ([data_plane.rs:364](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:364)). This can time out before the first byte and necessarily adds startup latency, violating RELIABLE/FAST and converge-up. Loopback tests do not expose it.

- [crates/blit-core/src/transfer_session/mod.rs:1804](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1804) — **MEDIUM** — Resize refusal is not actually terminal. `resize_settled(..., false)` clears pending state without recording refusal or advancing the epoch ([dial.rs:385](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:385)); a concurrent need/resume batch therefore re-proposes the same target and epoch with a new token. That contradicts the monotonic/idempotent epoch contract and can repeatedly stall dispatch.

**VERDICT: FAIL — parity pins pass, byte/tree results remain guarded, and test count did not drop, but the startup/StallGuard regression and incomplete refusal handling block acceptance.**
tokens used
152,298
- [crates/blit-core/src/transfer_session/mod.rs:1423](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1423) — **HIGH** — Payload dispatch now waits for every one-stream resize epoch serially. A 1→16 ramp costs 15 control RTTs plus connects; meanwhile receive workers’ 30-second StallGuards are already running ([data_plane.rs:364](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/data_plane.rs:364)). This can time out before the first byte and necessarily adds startup latency, violating RELIABLE/FAST and converge-up. Loopback tests do not expose it.

- [crates/blit-core/src/transfer_session/mod.rs:1804](/Users/michael/Dev/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1804) — **MEDIUM** — Resize refusal is not actually terminal. `resize_settled(..., false)` clears pending state without recording refusal or advancing the epoch ([dial.rs:385](/Users/michael/Dev/blit_v2/crates/blit-core/src/dial.rs:385)); a concurrent need/resume batch therefore re-proposes the same target and epoch with a new token. That contradicts the monotonic/idempotent epoch contract and can repeatedly stall dispatch.

**VERDICT: FAIL — parity pins pass, byte/tree results remain guarded, and test count did not drop, but the startup/StallGuard regression and incomplete refusal handling block acceptance.**
