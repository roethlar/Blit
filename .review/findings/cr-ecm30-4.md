# cr-ecm30-4: the stage's cleanup and the recovery path it reports are not reliable under renames and errors

**Severity**: MEDIUM
**Status**: Verified on Unix — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f4b1249b` (cr-ecm30-8)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` (Codex openreview over `fd603571..93ecf7b2`, 2026-11-22) (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`.

## Evidence
"Checking a stage's identity and then removing its name leaves a window in which an unrelated empty directory can be removed. Moving the stage or its parent also makes the recorded recovery path stale. After a successful exchange, an unlink failure is silently reported as success. Restrict destructive cleanup to names Blit controls, propagate cleanup errors, and verify the location reported for retained data."

## Intake
Admitted; it was a fault of the staging, which the owner then ruled out (D-2026-11-23-1). Closed by the rework that ruling asked for, not by a fix of its own.

## What
Nothing is staged any more (cr-ecm30-8, `f4b1249b`, D-2026-11-23-1): there is no stage to clean up or name in a recovery path, and no exchange after which a removal could fail. What remains of the request — removing only what is this run's — holds for the fresh partial itself: a partial that cannot be made ready is removed only while it is still the one made.

## Guard proof
cr-ecm30-8's guards; the staging's own guards went with it (named in cr-ecm30-8's record).

## Known gaps
- Unix only; macOS by cross-clippy alone.
