# cr-ecm32-4: on Windows the receiver discards the comparison's baseline, and a hard-linked destination is resumed in place

**Severity**: HIGH
**Status**: Admitted — NOT FIXED. Blocked: Windows-only code that nothing here can run, and no Windows host answers (`10.1.10.77` timed out; `.222` and `.173` no route; 2026-11-22).
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a`; `.review/results/combined-r33.codex.md`. It restates cr-ecm31-5 and cr-ecm29-3, whose records hold what it needs.

## Restated
`combined-r34` (Codex openreview, 2026-11-23) reviewed `70efef8a..0f912b06` — the `combined-r33` fixes — with `guard_confirmed: true` restated it within cr-ecm33-3: "On Unix, if a previously present partial's parent vanishes before hashing, `place` becomes `None`. If the parent returns as a symlink to another in-root directory, an empty name there can receive the file. Fail that pass when the parent cannot be established. On Windows the receiver discards even `Unjustified`, so a failed comparison can still authorize a later write. Enforce that refusal on every platform and retain the validated Windows handle for writes." Still blocked: off Unix the hold must refuse `ResumeBaseline::Unjustified`, honour `WholeOf`, and write through a validated handle.
