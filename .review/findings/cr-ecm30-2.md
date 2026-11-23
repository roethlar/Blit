# cr-ecm30-2: the staging directory made 700 drops its inherited setgid bit, and the file takes the wrong group (brought in by cr-ecm29-5)

**Severity**: HIGH
**Status**: Verified on Unix — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f4b1249b` (cr-ecm30-8)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r31` (Codex openreview over `fd603571..93ecf7b2`, 2026-11-22) (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r31.codex.md`.

## Evidence
"I copied the same `0640` source over a `0644` partial in a setgid directory. Both revisions exited successfully. The base retained the directory's group, while the head assigned the process's primary group. Setting the stage to `0700` clears its inherited setgid bit, so group-read permission on the finished file can go to the wrong group. Preserve that bit when repairing owner permissions."

## Intake
Admitted; it was a fault of the staging, which the owner then ruled out (D-2026-11-23-1). Closed by the rework that ruling asked for, not by a fix of its own.

## What
The staging directory is gone (cr-ecm30-8, `f4b1249b`, D-2026-11-23-1): the fresh partial is made in the destination's directory itself, so it takes that directory's group where the directory gives one (setgid), as any file made there does.

## Guard proof
`resume::tests::a_fresh_partial_takes_its_setgid_directorys_group` (Unix): in a setgid directory of another of this run's groups, the finished file has the directory's group. Run against the staging code of the commit before cr-ecm30-8, it went red (group 1000 where 3 was expected).

## Known gaps
- Unix only; macOS by cross-clippy alone.
