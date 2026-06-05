# audit-m28 — TUI source-of-truth doc sweep

**Source**: 2026-06-04 audit chain, R3 finding **M28** (promoted to first Round-2 item;
GPT R3 critique gated all Round-2 code work behind this).
**Class**: drift / doc-governance — owner-directed.

## What

`docs/plan/README.md`, `docs/plan/TUI_DESIGN.md`, and `docs/ARCHITECTURE.md` all named
different documents as the active TUI / release source of truth, despite
`TUI_REWORK.md` having shipped 2026-05-31 as the Phase 6 active plan:

- `docs/plan/README.md:8-13` still pointed at `RELEASE_PLAN_v2_2026-05-04.md` as the live
  SoT and described the directory as "release-readiness work for 0.1.0," even though
  0.1.0 had shipped.
- `docs/plan/TUI_DESIGN.md:3` had `Status: Active planning`. Its only mention of
  TUI_REWORK was a parenthetical "Rework note" in the revision-history block at the top
  — easy to miss; agents reading the file's stated Status would keep implementing
  against the F1–F4 model.
- `docs/ARCHITECTURE.md:140-150` §"blit-tui" described F1–F4 as the active architecture
  with no mention of the dual-pane plan.

GPT-5 / GPT R2 surfaced the split as three independent findings; R3 reframed it as a
single doc-governance failure that **causes** the F1–F4 drift agents have been
re-implementing (R3-H4, R3-H5, R3-H23).

## Approach

Three coordinated edits, one commit:

1. `docs/plan/README.md` rewrite: name TUI_REWORK.md as the live active plan + the
   2026-06-04 audit INDEX as the open-finding source. RELEASE_PLAN_v2 demoted to
   "shipped, frozen reference." Quick-start sequence directs new readers at TUI_REWORK
   + the audit INDEX. Note that greenfield_v6 §1.1 (streaming planner + 1 s heartbeat
   + 10 s stall detector) is canonical per owner ratification but not yet built —
   queued for after Round 1 hardening.
2. `docs/plan/TUI_DESIGN.md`: explicit ⚠️ SUPERSEDED banner at the top of the header.
   Status line changed to "SUPERSEDED — see TUI_REWORK.md for the active plan."
   Document retained for historical context; agents told explicitly not to implement
   against it.
3. `docs/ARCHITECTURE.md` §"blit-tui": rewrite the first paragraph to lead with the
   Phase 6 dual-pane M1–M6 plan as the active model. Add a sentence noting the v0.1.0
   F1–F4 baseline that shipped and pointing at TUI_DESIGN as the historical reference.

The owner-ratified streaming-planner direction (R3-H10b RATIFY per 2026-06-04 directive,
"blit needs to start moving bits instantly and tune parameters as the transfer
progresses") is now visible in `docs/plan/README.md` as a not-yet-built canonical item,
so future contributors don't strike it from greenfield_v6 thinking it's stale plan
drift.

## Files changed

- `docs/plan/README.md` — full rewrite of Current Status + Document Index. Last-Updated
  stamped 2026-06-04.
- `docs/plan/TUI_DESIGN.md` — header banner + Status line.
- `docs/ARCHITECTURE.md` — §"blit-tui" first paragraph rewrite.

No code changes. No tests added — the slice is doc-only.

## Tests added

None (doc-only slice). The validation suite is unaffected; nothing to re-run.

## Known gaps

- **`docs/plan/TUI_DESIGN.md` body content** still describes the F1–F4 implementation
  details. Banner at top is enough to signal that the body is historical; rewriting
  the body would be redundant churn given TUI_REWORK is now the authoritative
  reference. If a future contributor mines TUI_DESIGN for shared patterns (e.g.,
  daemon-Subscribe scaffolding), the relevant parts that survived into the rework
  are in TUI_REWORK §8 anyway.
- **Other historical artifacts** (`docs/grok_review.md`, `docs/forklift_audit/`,
  `docs/reviews/`) flagged by GPT-24 / R3-L5 are not banner'd in this slice. Separate
  follow-up.
- **`docs/plan/REMOTE_TRANSFER_PARITY.md` status** is described as "shipped" in the
  new README; if any part of it is still aspirational the README claim needs tightening.

## Cross-references

- R3 finding M28, see `docs/audit/AUDIT_REPORT_2026-06-04_R3.md` (added in R3 as a
  doc-governance finding extending R2-H15).
- R3 finding L3 (plan index points to stale live docs) — closed by this slice's
  README rewrite.
- Memory `feedback-ai-review-is-not-owner-decision` — applies: AI reviewer
  endorsement of TUI_REWORK doesn't ratify owner-level decisions in §6 (still open);
  those stay open in `TUI_REWORK.md` and are unaffected by this slice.
- R3-H10b owner ratification (2026-06-04): streaming planner / 1 s heartbeat /
  10 s stall detector is canonical but not yet built. Now visible in
  `docs/plan/README.md`.
