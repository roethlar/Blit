# Blit Codebase + Plan Audit — 2026-06-04 (Index)

**Current audit state of truth**: read **R2** + **R3 delta**. R3 overrides the IDs listed
in its severity-changes section; everything else inherits from R2.

## Files

| File | Role | Read when |
|---|---|---|
| `AUDIT_REPORT_2026-06-04.md` | **R1** — workflow-driven base | Historical reference only. R2 supersedes it. |
| `AUDIT_REPORT_2026-06-04_R2.md` | **R2** — merge of R1 + GPT first review | Always; this is the inventory + cross-cutting analysis + full prose. |
| `AUDIT_REPORT_2026-06-04_R3.md` | **R3** — delta after GPT R2 critique | After R2; applies severity rebalance + 2 added findings. |
| `inventory/` | Per-cluster file inventories (plan + code) | When tracing a specific finding back to source. |
| `findings/` | Per-cluster drift + inconsistency notes | When tracing a specific finding back to source. |

## ID overrides (R3 → R2)

When R2 and R3 disagree on a finding's severity or content, R3 wins. The overrides are:

| R2 ID | R3 ID | Change |
|---|---|---|
| R2-H9 | R3-M24 | Demoted to MED — doc drift, not behavior bug |
| R2-H10 | R3-H10 (split into subparts H10a + H10b) | Same HIGH ID, two subparts: H10a scan-await timeout (reliability bug, fix regardless) + H10b streaming planner (plan claim, ratify or retire, gated on SoT). Counts as one HIGH in the tally. |
| R2-H14 | R3-M25 | Demoted to MED — observability, not destructive |
| R2-H17 | R3-M26 | Demoted to MED — doc contradiction, no proven impact |
| R2-H23 | R3-M27 + R3-L39 | Split: documented-missing overrides (MED) vs diagnostic env vars (LOW, owner policy) |
| — | R3-H23 (NEW) | Dual-pane path bars + `/` → F3FilterBegin dropped by Screen::Dual |
| — | R3-M28 (NEW) | TUI source-of-truth doc-governance failure |

R3-H23 reuses the H23 slot freed by R2-H23's split — these are different findings.

## Tallies

- **R1**: 21 HIGH + 16 MED + 33 LOW = 70 findings (`AUDIT_REPORT_2026-06-04.md`)
- **R2**: 23 HIGH + 23 MED + 38 LOW = 84 findings (`_R2.md`)
- **R3 (current)**: 20 HIGH + 28 MED + 39 LOW = 87 findings (R2 + R3 delta)

## Implementation order (current)

R3's recommendations override R2's. The current six-round order is in R3 §"Updated
recommendations" — Round 1 (data-loss/DoS) unchanged from R2; Round 2 (TUI rework) leads
with R3-M28 doc-SoT sweep, then H4 → H5 → R3-H23 → H2 → H6 → H7 → H8 → M2 → M3 → M4 →
M25.

## Open owner decisions (R3 final section)

1. R3-L39 — env-var carve-out for diagnostics: absolute rule or carved?
2. R3-H10b (plan-claim subpart of R3-H10) — is `greenfield_v6.md` §1.1 still canonical?
   (Gated on R3-M28 first.) R3-H10a (reliability-bug subpart) is fixed regardless.
3. R3-M28 — OK to land doc-SoT sweep as a single PR before Round 2 code work? (Strong
   recommendation: yes.)
4. R3-M27 — implement `BLIT_FORCE_GRPC_DATA` / `BLIT_DISABLE_LOCAL_TELEMETRY`, or strike
   from `greenfield_v6.md` §1.2/§1.3?

Plus R2's still-open: TUI_REWORK §6 owner-level decisions that AI reviewers endorsed but
owner has not ratified.

## When to write an R4

R4 if any of:
- Owner answers one or more decisions above (carries into the next ratification cycle).
- A subsequent independent review (GPT R3 critique-of-R3, a third reviewer, owner walk-
  through) surfaces new findings OR overturns the R3 severity calls.
- Round 1 implementation closes and the punch list needs re-ranking.

Otherwise R3 stands.
