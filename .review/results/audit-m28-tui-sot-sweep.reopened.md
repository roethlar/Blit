# Reopened: audit-m28-tui-sot-sweep

**Reviewer**: gpt (relayed via owner), 2026-06-05.

## Finding

The SoT sweep introduced a contradiction between the plan README and the audit
chain it points readers at:

- `docs/plan/README.md:23` (this slice) states `greenfield_v6 §1.1` is canonical
  per owner ratification.
- `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md:47` (existing, unchanged this slice)
  still lists **R3-H10b** under "Open owner decisions" — "is `greenfield_v6 §1.1`
  still canonical?".
- `docs/audit/AUDIT_REPORT_2026-06-04_R3.md:245` (existing, unchanged this slice)
  same — H10b open in "Outstanding owner decisions."

A SoT sweep cannot leave the index it claims as the open-finding source out of
sync with the README's ratification claim. Either:

(a) Update the audit chain (INDEX + R3) to record the 2026-06-04 owner
    ratification of H10b, **OR**
(b) Remove the ratification claim from the plan README and re-frame as "open
    pending owner decision."

(a) is the correct move — the owner did ratify in this session ("blit needs to
start moving bits instantly and tune parameters as the transfer progresses…
yes, that's a giant gap we need to close"). Update the audit chain accordingly.

## Required fixes

1. `docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md` — remove H10b from "Open owner
   decisions" or move it to a new "Resolved" / "Ratified" section noting:
   "R3-H10b RATIFIED 2026-06-04: §1.1 is canonical. Streaming planner +
   1 s heartbeat + 10 s stall detector queued for implementation after Round 1
   hardening closes." H10a remains as a HIGH bug to fix regardless.
2. `docs/audit/AUDIT_REPORT_2026-06-04_R3.md` §"Outstanding owner decisions" —
   same treatment.
3. Optional: add a "Resolved owner decisions" section to the INDEX that
   captures this kind of ratification (and L39/M27 ratification too, which
   landed in `audit-l39-m27-env-var-purge`).

## Validation expected after fix

- A reader landing on the plan README, following its pointer to the audit
  INDEX, sees H10b consistently described as ratified rather than open.
- The other unresolved items (L39 carve-out — wait, that's also resolved now;
  see L39/M27 slice; M28 itself is in flight) are accurate.

## Scope

Same finding, fix-up. Original analysis at
`.review/findings/audit-m28-tui-sot-sweep.md` still applies.
