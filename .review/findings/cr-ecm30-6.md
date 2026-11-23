# cr-ecm30-6: macOS refuses every replacement, which the Unix CLI test expects to succeed; Windows still patches a hard-linked destination

**Severity**: HIGH
**Status**: macOS's half resolved by cr-ecm30-8 (`f4b1249b`), awaiting review; Windows's half NOT FIXED — blocked (it is cr-ecm29-3: no Windows host)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f4b1249b` (cr-ecm30-8), macOS's half
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` (Codex openreview over `fd603571..93ecf7b2`, 2026-11-22) (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`.

## Evidence
"On macOS, every occupied destination requiring a fresh partial reaches the unsupported branch, while the new Unix CLI test requires that copy to succeed. Use macOS's native exchange on supporting volumes and retain the safe fallback elsewhere. On Windows, the previously admitted NTFS hard-link issue remains open. Local and wire resume need a handle-based link-count check before patching."

## What
- macOS: nothing waits on an exchange any more (cr-ecm30-8, D-2026-11-23-1). An occupied destination that needs a fresh partial is removed and the partial made at the empty name, on every Unix, so the Unix CLI tests' expectations hold there too. Checked by macOS cross-clippy only; never run on a Mac.
- Windows: the NTFS hard-link check is cr-ecm29-3, still blocked — no Windows host answers (`10.1.10.77` timed out; `.222` and `.173` no route, 2026-11-22).

## Known gaps
- macOS never run here. Windows's half not done.
