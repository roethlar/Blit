# cr-ecm30-7: macOS strict Clippy fails: the `Replacing` variants and the `wider_destination` helper are unused there

**Severity**: MEDIUM
**Status**: Resolved by cr-ecm30-8, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f4b1249b` (cr-ecm30-8)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` (Codex openreview over `fd603571..93ecf7b2`, 2026-11-22) (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`.

## Evidence
"The default macOS cross-check stopped in BLAKE3's C build. A diagnostic run with `blake3/pure` reached Blit and found two strict-Clippy failures in the `Replacing` variants and the `wider_destination` test helper."

## Intake
Admitted; it was a fault of the staging, which the owner then ruled out (D-2026-11-23-1). Closed by the rework that ruling asked for, not by a fix of its own.

## What
`Replacing` is gone, and `wider_destination`'s users are all Unix tests now (cr-ecm30-8, `f4b1249b`).

## Guard proof
macOS cross-clippy (`aarch64-apple-darwin`, every target of the workspace, BLAKE3's pure-Rust build through `CARGO_FEATURE_PURE=1`, strict) is clean at cr-ecm30-8 and at cr-ecm30-1; at the commit before cr-ecm30-8 it found exactly the reviewer's two errors (`Done`/`Vacant` never constructed; `wider_destination` never used).

## Known gaps
- Unix only; macOS by cross-clippy alone.
