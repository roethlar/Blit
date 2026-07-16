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
- **Live dial policy carries from REV4.** SOURCE continuously adjusts
  cheap dials and stream count from measured send telemetry, including
  mid-transfer ADD and REMOVE. Workload shape informs payload planning;
  it does not map file/byte totals to a terminal stream count. Both
  connection layouts inherit the one SOURCE-owned controller.
- **The bounded-unilateral dial contract carries unchanged**
  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
  dial, bounded by the byte RECEIVER's advertised capacity profile
  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
  unlimited). The session's role model must express this — profile
  travels DESTINATION→SOURCE at setup regardless of who initiated —
  and otp-1's contract names it explicitly.
- **Live dial correction (2026-07-16):**
  `docs/plan/LIVE_DIAL_TUNING.md` (D-2026-07-16-2) owns the reviewed
  correction. Its ldt-2 cutover removed sf-2's static ADD-only shape target
  and put the same telemetry-driven ADD/REMOVE controller behind both SOURCE
  socket layouts. ldt-3 supplies lifecycle/observer closure; review of candidate
  `436e1bb` admitted one Low observer-ordering defect whose isolated fix is
  mutation-proved and awaits re-review. Hardware evidence remains ldt-4.
- Wire contract discipline (REV4 rule): the unified session's proto —
  messages, field numbers, capability negotiation, transport
  selection — is a reviewed doc+proto slice **before** any behavior
  depends on it.
- Every slice through neutral Claude openreview (D-2026-07-04-1,
  D-2026-07-16-1); tree green
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
      (1483); all REV4 invariant pins and live ADD/REMOVE dial guards
      pass role-parameterized.
- [ ] Benchmark methodology corrected and recorded: symmetric-fs
      cells are the verdict cells; tmpfs cells remain only as
      explicitly-labeled wire-reference rows (never compared across
      directions with asymmetric endpoints).
- [ ] Windows: full suite green (owner machine) + windows-latest CI.

## Design

**What already is one code** (kept, becomes the session's engine):
`remote/transfer/` — pipeline, sink/source abstractions, data plane,
diff planner, tar-shard, stall guard, progress, `operation_spec` (the
REV4 unified contract), and the live engine dial. The defect layer is
above it: four driver loops
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
4. The data plane opens at a conservative receiver-bounded dial floor.
   SOURCE then tunes the live stream count up or down from measured
   per-stream telemetry. Initiator role and push/pull-facing verb never
   select policy; the receiver's advertised capacity is only a safety
   bound.
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
   old push (byte-identical trees and summary parity). The sf-2 static
   shape pin was ported here as transitional behavior; it is not the
   final stream policy and is retired by `LIVE_DIAL_TUNING.md`.
5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   equivalent) — the same code with roles flipped; the parity suite
   reruns with no per-direction test code.
6. **otp-6 mirror + filters** on the session (one delete rule).
7. **otp-7 resume** block phase (ordering + stale-partial pins per
   the Design's RELIABLE exception). Slice design: `docs/plan/OTP7_RESUME.md`
   (staged 7a in-stream / 7b data-plane).
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
