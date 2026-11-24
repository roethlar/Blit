# cr-ecm38-4: a Windows resume that makes its partial discards that it made it

**Severity**: HIGH
**Status**: Admitted — NOT FIXED. Blocked: Windows-only code that nothing here can run, and no Windows host answers (2026-11-24: `10.1.10.77` refuses SSH; `10.1.10.222` and `10.1.10.173` no route).
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It is the Windows side of cr-ecm35-4, cr-ecm36-5 and cr-ecm37-3, and it stands beside cr-ecm29-3, cr-ecm31-5 and cr-ecm32-4.

## Evidence
"By inspection, if a resume target disappears before the first block, the sink can create a partial, discard the creation result and report nothing left when the source aborts. Retain evidence of creation or report the disposition as unknown."

## Intake
Admitted as the reviewer put it. Off Unix:
- **The open.** The resume opens its target with `create(true)`, which makes the file when it is gone, and reports it not made.
- **The hold.** The hold is discarded (`Ok(None)`).

A record its source fails then has nothing held, and reports only the held failure's word: nothing left. On Unix the hold keeps whether it made the partial (cr-ecm35-4), and the close looks at what stands (cr-ecm36-5, cr-ecm37-3).

## What would fix it
On Windows, the open would make the partial by `create_new` first and keep that it did. The hold, or a record of the creation, would reach the close, which would say the partial is left, or not known.

## Why not fixed
This is Windows-only runtime behaviour. Cross-compiling it checks that it builds, not what it does, and no Windows host is reachable to run it or a guard. Landing it unproven would be the unverifiable code the owner's rule keeps out.
