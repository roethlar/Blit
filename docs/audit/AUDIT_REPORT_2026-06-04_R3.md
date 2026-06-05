# Blit Codebase + Plan Audit — 2026-06-04 (Revision 3 — delta over R2)

R3 is a small delta over `AUDIT_REPORT_2026-06-04_R2.md`. R2 stays as the base document; R3
records the adjustments from GPT's R2 critique of R2 (severity rebalance + two added
findings). Read R2 for the inventory and full prose; read R3 for the changes. New readers
start at `AUDIT_REPORT_2026-06-04_INDEX.md` — a short pointer that names exactly which
file owns which content and lists every R3-overrides-R2 ID.

R3 changes only severity scoring and adds two findings GPT R2 surfaced. Every finding's
**evidence** (file + line) is unchanged from R2 unless explicitly noted.

## Why a delta and not a rewrite

R2's recommendation ordering (Round 1 → 6) leads with H1/H2/H3/H7/H8/H11/H13/H22, none of
which R3 touches. The severity rebalance affects H9/H10/H14/H17/H23 — items that sit in
Round 4 (doc-of-record alignment) and Round 2 (TUI rework). Under R3 the implementation
order is:

- **Round 1**: unchanged from R2 (data-loss / DoS class first).
- **Round 2**: reordered. R3-M28 (TUI doc-SoT sweep) moves to the **first** item; R3-H23
  (dual-pane path bars + `/` dispatch) is inserted after H5 (model types). R2-H14 is
  demoted to R3-M25 (still in Round 2). The rest of Round 2 inherits from R2.
- **Rounds 3–6**: inherit from R2 except for the demotions and split listed in the
  severity-changes section — R2-H9 → R3-M24, R2-H17 → R3-M26, and R2-H23 split into
  R3-M27 + R3-L39. None of those moves changes when the work lands; they only change how
  it's labeled.

## Severity changes (from R2 HIGH → R3 MED, except H10)

### R2-H9 → R3-M24 — `--detach` shipped despite REMOTE_REMOTE_DELEGATION_PLAN §9

**Reasoning**: Real doc drift, not a behavior defect. Later TUI design + M-Jobs milestone
work made detach intentional; the delegation plan §9 simply wasn't updated. No operator
sees worse behavior because of it; readers of the delegation plan see stale scope.
**Remediation reframe**: One-line update to REMOTE_REMOTE_DELEGATION_PLAN.md §9 and §4.2
step 12, plus a cross-link to TUI_DESIGN §M-Jobs.

### R2-H10 → R3-H10 (HIGH, split into subparts H10a + H10b)

**Split in wording (per GPT R3)** — the finding stays one HIGH ID with two subparts:

**R3-H10a — Reliability bug, HIGH, fix regardless**: The synchronous orchestrator
(`orchestrator.rs:540-574`) awaits `scan_handle.await` with no outer timeout. A stuck
local scan (network FS hang, hung enumerator, kernel-side wedge) parks the orchestrator
indefinitely. This is a real reliability bug and the fix (wrap `scan_handle.await` in
`tokio::time::timeout` with a generous bound; emit a clear error on expiry) is owed
regardless of which plan doc is canonical. Mirrors the `feedback_server_await_timeouts`
rule.

**R3-H10b — Plan claim, HIGH, ratify or retire**: `greenfield_plan_v6.md` §1.1 + WORKFLOW_PHASE_2
specify a streaming planner, 1 s default heartbeat (500 ms when workers starved), 10 s
stall detector covering planner + workers. None of this exists in code. Decide:
- (Ratify) keep §1.1 canonical, build the streaming planner — large work, multi-slice.
- (Retire) update §1.1 to describe the synchronous orchestrator + 30 s pull stall guard
  that actually shipped — one-PR doc edit.
This decision is gated on settling the active source-of-truth pointer in
`docs/plan/README.md` (see R3-M28, L3) — until that's done, no agent can know which §1.1
is current.

R3-H10a does **not** wait on R3-H10b; fix the timeout first. Both subparts count as a
single HIGH finding (R3-H10) for severity-tally purposes.

### R2-H14 → R3-M25 — Push verb table omits (delegated, Mirror) and (delegated, Move) labels

**Reasoning**: Genuine observability gap, but the operator does see *some* footer text;
there's no false-affordance and no destructive divergence between what the footer says
and what runs. MED restoring R1's original ranking.

### R2-H17 → R3-M26 — TarShardExecutor on TCP push hot path vs POST_REVIEW §1.2

**Reasoning**: Doc/comment contradiction in a single file (`data_plane.rs:327` vs `:620-647`)
with no proven perf or correctness impact. Risk is "future contributor deletes hot code by
mistake," which is real but contained by the cargo build and test suite. MED.

### R2-H23 → split into R3-M27 + R3-L39 — Env-var policy violations + missing documented overrides

**Reasoning**: R2 conflated three distinct things into one HIGH. Splitting:

- **R3-M27** (MED): `BLIT_FORCE_GRPC_DATA=1` and `BLIT_DISABLE_LOCAL_TELEMETRY=1` are
  documented in `greenfield_plan_v6.md` §1.2 line 161 and §1.3 line 168 as operator
  escape hatches. They do not exist in code. Operators following the plan see no effect
  with no warning. This is doc/feature drift that affects user-facing behavior.
- **R3-L39** (LOW, but flag for owner policy decision): `BLIT_TUI_INPUT_TRACE` and
  `BLIT_TEST_COUNTER_FILE` are diagnostic / test instrumentation, not user config. The
  "no env vars" rule was (in context) about runtime configuration; diagnostic
  instrumentation is a different category. Owner decision: is the rule absolute, or does
  it carve out test/diagnostic? Either way, gate `BLIT_TEST_COUNTER_FILE` behind
  `#[cfg(test)]` or feature flag to be safe.

## New findings (added in R3)

### R3-H23 (NEW) — Dual-pane path bars are display-only; `/` is mapped globally but Dual dispatch drops it

**Source**: GPT R2 + GPT R3 correction
**Class**: drift / user-facing UX failure
**Where**:
- `crates/blit-tui/src/dual_pane.rs:172-183` — `PaneState` carries `path_editor: String`
  not the spec'd `PathEditorState`
- `crates/blit-tui/src/screens/dual_pane.rs:93-99` (renderer) — path bar drawn as
  `Span::raw(pane.path_editor().to_string())` inside a one-line Paragraph; no edit cursor,
  no selection state, no character input dispatch
- `crates/blit-tui/src/main.rs:5281` — global key map **does** translate `/` to
  `UserAction::F3FilterBegin` (verified — even has a regression test at `:7509-7515`)
- `crates/blit-tui/src/main.rs:2219-2234` — Screen::Dual dispatch arm only handles
  navigation (`Refresh`, `SelectNext/Prev/First/Last`, `Descend`, `Ascend`,
  `DualSwitchPane`, `F3ToggleMark`); `F3FilterBegin` and every transfer action fall
  through to `_ => {}`
**Plan**: `TUI_REWORK.md:64` — path bars are editable, `/` opens search. §10 testing
contract requires asserting path-edit and search-state transitions.
**Why this matters**: This is a **direct user-facing rework promise broken** that R2
folded under H5 "TransferDraft/BatchTransferDraft missing." Two operationally distinct
failures sharing a root cause with R2-H4:
1. Path bar is rendered but receives no character input (no `PathEditorState` model, no
   "active pane is in edit mode" state on Dual).
2. `/` is correctly captured by the global key map *and routed to nothing* on Dual,
   because `Screen::Dual` discards every action it doesn't navigate with — including
   `F3FilterBegin`. Operator presses `/`, sees nothing happen, has no signal that the
   key was even received.
Same class as R2-H2 (Esc quits) and R2-H4 (action bar render-only): visible affordance,
unwired dispatch. Promoted to HIGH.
**Remediation** (depends on H5 model types):
1. After H5 introduces `PathEditorState`, add an `editing` state to `PaneState`. Wire
   keyboard input on the active pane to drive it when active.
2. Route `F3FilterBegin` into the `Screen::Dual` arm — either reuse F3's filter state on
   the active pane, or introduce a `DualSearchBegin` action and route both bindings to
   it for consistency.
3. Add a regression test pinning `/` → search state transition on Dual, mirroring the
   existing `:7509-7515` test for the global mapping.
4. W1-W3 workflow tests assert path-bar edit + search filter end-to-end.

### R3-M28 (NEW) — TUI source-of-truth split is a doc-governance failure, not just a doc edit

**Source**: GPT R2 (separating from R2-H15 which conflated security drift with TUI SoT)
**Class**: drift / doc governance
**Where**:
- `docs/plan/TUI_REWORK.md:3` — current spec, says dual-pane active and is the rework's
  source of truth
- `docs/plan/TUI_DESIGN.md:3` — header still says "active planning for F1-F4"
- `docs/ARCHITECTURE.md:140` — architecture chapter still describes F1-F4 as the TUI's
  navigation model
- `docs/plan/README.md:8` — plan index still names the 0.1.0 release plan as live source
  of truth (also R3-L3)
**Why this matters**: This is the meta-failure that *causes* drift like R2-H4 and R2-H5.
An agent (human or AI) opening this repo without prior context cannot determine which
document is current. The MASTER_WORKFLOW.md mandates updating SoT pointers on every
phase transition; that didn't happen for the F1-F4 → dual-pane rework. Repeated
implementation churn (last seen this week) traces directly to this confusion.
**Remediation** (single sweep, must be atomic):
1. `docs/plan/README.md`: rewrite §"Live plans" to name `TUI_REWORK.md` as active source
   of truth; mark `TUI_DESIGN.md` as historical reference; mark `RELEASE_PLAN_v2` as
   shipped-and-frozen.
2. `docs/plan/TUI_DESIGN.md:3`: change header banner to "SUPERSEDED by `TUI_REWORK.md`
   as of 2026-05-31. Retained for historical reference."
3. `docs/ARCHITECTURE.md:140`: rewrite §"TUI" to reference the dual-pane M1-M6 plan;
   delete F1-F4 narrative or move to an "Historical TUI" appendix.
4. Add `MASTER_WORKFLOW.md` line item: "On phase transition, sweep `docs/plan/README.md`,
   `ARCHITECTURE.md`, all superseded plan headers." Enforce via a docs-lint check
   (proposed, low priority).

## Updated severity tallies

R2: 23 HIGH, 23 MED, 38 LOW (84 total)
R3: 20 HIGH, 28 MED, 39 LOW (87 total)

### R3 HIGH list (20)

H1, H2, H3, H4, H5, H6, H7, H8, **H10 (split into H10a reliability bug + H10b plan claim;
counts as one HIGH)**, H11, H12, H13, H15, H16, H18, H19, H20, H21, H22, **H23 (NEW —
dual-pane path bars + `/` dispatch drop)**.

Note: R3 reuses the slot "H23" for the NEW dual-pane path-bars finding (R2-H23 demoted to
M27 + L39). R2-H9, R2-H14, R2-H17 demoted to M24, M25, M26 respectively. R2 numbering for
H1-H8, H11-H13, H15-H16, H18-H22 is unchanged.

### R3 MED list (28)

M1 through M23 (verbatim from R2), plus:
- **M24** (was R2-H9): `--detach` shipped despite REMOTE_REMOTE_DELEGATION_PLAN §9
- **M25** (was R2-H14): Push verb table delegated-Mirror / delegated-Move labels
- **M26** (was R2-H17): TarShardExecutor doc contradiction
- **M27** (split from R2-H23): Documented overrides `BLIT_FORCE_GRPC_DATA` /
  `BLIT_DISABLE_LOCAL_TELEMETRY` do not exist
- **M28** (NEW): TUI source-of-truth split is a doc-governance failure

### R3 LOW list (39)

L1 through L38 from R2, plus:
- **L39** (split from R2-H23): `BLIT_TUI_INPUT_TRACE` + `BLIT_TEST_COUNTER_FILE`
  diagnostic env vars; owner policy decision needed on env-var carve-out

## Updated recommendations (Round 2 only)

Round 1 unchanged from R2.

Round 2 — TUI rework alignment, with R3-M28 first, R3-H23 inserted after H5:

6. **R3-M28 (FIRST in Round 2)** TUI source-of-truth doc sweep. Do this **before** any
   TUI code change so agents implementing Round 2 read the right spec. Mechanically:
   update `docs/plan/README.md` to name `TUI_REWORK.md` active; banner
   `TUI_DESIGN.md` as SUPERSEDED; rewrite `ARCHITECTURE.md` §"TUI" away from F1-F4.
   One commit, one PR.
7. **H4** Wire `TransferCopy`/`Mirror`/`Move`/`Delete`/`Verify` into Dual screen
   dispatch. Until landed, flip default screen back to F1.
8. **H5** Introduce `TransferDraft` / `BatchTransferDraft` / `BrowseProvider`. Move
   existing per-pane fetch logic behind the trait.
9. **R3-H23** Wire path-bar character input + route `F3FilterBegin` (or a new
   `DualSearchBegin`) into `Screen::Dual` dispatch. Depends on H5 (PathEditorState).
10. **H2** Wire Esc to per-screen back; reserve quit to `q` / Ctrl-C.
11. **H6** Flip TUI delegated `detach: true` + banner + regression test.
12. **H7** Stream `BytesProgress` from delegated_pull.
13. **H8** Plumb data-plane byte counters into `ActiveJob.to_proto`.
14. **M2** Define favorites / recents / known-endpoints schema in `tui.toml` + dedicated
    JSONL.
15. **M3** Batch-table renderer for fan-out (depends on H7/H8).
16. **M4** Thread capabilities through `BrowserEntry`; gate action labels.
17. **M25** Complete the (delegated, Mirror) and (delegated, Move) verb table entries.

Rounds 3-6 unchanged from R2 except:

- **M24 (was H9)** Update REMOTE_REMOTE_DELEGATION_PLAN §9 / §4.2 step 12 — moved from
  Round 4 priority order to wherever convenient; it's a one-line doc edit.
- **M26 (was H17)** Same — single-file doc-comment fix.
- **M27 (split from H23)** Either implement `BLIT_FORCE_GRPC_DATA` /
  `BLIT_DISABLE_LOCAL_TELEMETRY`, or strike them from `greenfield_plan_v6.md` §1.2/§1.3.
- **L39** Owner policy decision on env-var carve-out.

## "What's solidly aligned" — caveat preserved

R2's caveated wording in this section (e.g., `require_complete_scan` purge gate honored
*but see H1 for the relay-mirror bypass*; BlitAuth removal complete in code *but
ARCHITECTURE.md hasn't caught up*; mDNS shipped *but DAEMON_CONFIG lists only 2 of 4
fields*; receive-pipeline unified *on the TCP fast path; gRPC fallbacks still hand-
written*) is the right pattern and stays. R3 adds one more:

- Dual-pane M1-M2 plan-mandated infrastructure landed *but the user-facing affordances
  (action bar, path bars, `/` search) are unwired — see H4, H23, H5*.

## Resolved owner decisions (2026-06-04)

The following R3 decisions were resolved during the 2026-06-04 audit session.
Implementing slices are on master.

1. **R3-L39 — RESOLVED**: env vars are out for app + diagnostic config. No
   carve-out. Diagnostics get CLI flags, sparingly added, marked diagnostics-
   only. `BLIT_TUI_INPUT_TRACE` and `BLIT_TEST_COUNTER_FILE` purged in
   `audit-l39-m27-env-var-purge` (master `7c3ffd5`).
2. **R3-H10b — RATIFIED, not yet implemented**: `greenfield_v6 §1.1`
   (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical.
   Owner directive 2026-06-04: "blit needs to start moving bits instantly and
   tune parameters as the transfer progresses … yes, that's a giant gap we
   need to close." Implementation is a multi-slice piece queued for after
   Round 1 hardening closes. R3-H10a (reliability-bug subpart — outer
   `scan_handle.await` timeout) is independent and fixed regardless.
3. **R3-M28 — RESOLVED, IMPLEMENTED**: doc-SoT sweep landed as a single
   commit on master (`e8a5280`). `docs/plan/README.md` names TUI_REWORK as
   active + audit INDEX as open-finding source. `TUI_DESIGN.md` banner'd
   SUPERSEDED. `ARCHITECTURE.md §"blit-tui"` rewritten to lead with
   dual-pane M1–M6.
4. **R3-M27 — RESOLVED**: both `BLIT_FORCE_GRPC_DATA` and
   `BLIT_DISABLE_LOCAL_TELEMETRY` struck from `greenfield_v6.md §1.2/§1.3`.
   Replaced with "no env-var form" prohibition; any future locked-down
   override or telemetry opt-out will be a CLI flag (sparingly added,
   diagnostics-only).

## Outstanding owner decisions (only)

R2's still-open: the TUI_REWORK §6 locked-decisions that AI reviewers
endorsed but you haven't ratified.

## What R3 does NOT change

- Method, scope-gap-callout, cross-comparison table, Appendix A coverage attestation,
  Appendix B file references — all unchanged from R2.
- R2's 5 cross-cutting inconsistency dimensions — unchanged.
- R1's 70 findings inventory — unchanged.
- Implementation order leading with data-loss/DoS-class — unchanged.

R3 is purely a severity rebalance + 2 added findings + 1 owner decision-tree update.

---

*End of Revision 3 delta. Combined with R2 inventory, this is the working audit. Total:
20 HIGH + 28 MED + 39 LOW = 87 findings.*
