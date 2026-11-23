# cr-ecm30-3: under a mask that removes owner read, the staging directory cannot be opened, and is left

**Severity**: MEDIUM
**Status**: Verified on Unix — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f4b1249b` (cr-ecm30-8)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` (Codex openreview over `fd603571..93ecf7b2`, 2026-11-22) (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`.

## Evidence
"With umask `0400`, the head exited 2 and left two `0300` staging directories, one per attempt. `open_stage` requests read access before it can repair the mode. Acquire the directory through a handle that works under this mask and repair it through that handle."

## Intake
Admitted; it was a fault of the staging, which the owner then ruled out (D-2026-11-23-1). Closed by the rework that ruling asked for, not by a fix of its own.

## What
The staging directory is gone (cr-ecm30-8, `f4b1249b`, D-2026-11-23-1): the fresh partial is made at the name by `O_CREAT|O_EXCL` with read and write asked of the creation itself, and given 600 through its handle, whatever the mask.

## Guard proof
`local_copy_mode::a_resume_under_a_narrow_mask_makes_its_fresh_partial` (`blit-cli`, Unix) runs under masks 0200 and 0400: both copies complete, the file 600, nothing left beside it. Run against the staging code of the commit before cr-ecm30-8, mask 400 failed the copy.

## Known gaps
- Unix only; macOS by cross-clippy alone.
