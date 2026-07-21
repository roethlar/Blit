# ldt-4-live-f9 — accept an already-exited exact Windows launcher at teardown

**Severity**: MEDIUM — a completed Windows-responder arm can void the entire
session when its exact `cmd.exe` launcher exits normally between teardown's
existence check and `Stop-Process`.
**Status**: Fixed, mutation-proved, full-gate green, and tactically reviewed clean; additive staging and live retry pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `ef9ef0b6f5317dec4ef609c8e9e59f731c72e501`

## Evidence

Exact reviewed and staged harness `c2e1284` cleared every prior live finding
and completed 25 byte-identical arms in retained session
`ldt4-20260721T212142Z-c2e12846bcb1`. Arm `ldt4-026` also completed its
Windows-SOURCE transfer through `summary_received`, then exact teardown stopped
daemon PID 7352. Its registered `cmd.exe` launcher PID 1172 exited before the
immediately following `Stop-Process -Id 1172 -ErrorAction Stop`, which failed
with `Cannot find a process with the process identifier 1172`.

The launcher had already passed exact name and normalized command-line identity
checks, and its daemon had passed exact executable, command, parent, and PID
checks. The post-failure read-only audit found no q or Windows port-9031
listener and no session or Blit process. Windows' active daemon was restored to
the prior SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`;
the tested daemon remains retained at SHA-256
`ae414e649cf64f042f9d2a61639371c57d2fc3107cc426fe5a27a057b6630322`.

The session is void: `runs.csv` contains 25 provisional data rows plus its
header. Arm 26 was not appended, and no row may be analyzed or graded.

## Predicted observable failure

Intermittently after a successful Windows-responder transfer, stopping the
exact daemon lets its synchronous `cmd.exe /d /c start.cmd` launcher finish in
the check-to-stop window. Teardown mistakes the already-achieved launcher
postcondition for a cleanup failure and voids the session.

## What

Treat an already-absent, previously identity-verified Windows launcher as a
successful teardown state while continuing to fail closed if the launcher,
any child, the daemon, or port 9031 survives.

## Approach

- Preserve exact launcher and daemon identity checks before stopping anything.
- Make the redundant launcher stop tolerant of the launcher disappearing after
  its exact daemon is stopped.
- Keep the existing post-stop launcher, child, daemon, and listener absence
  checks authoritative.
- Extend the Bash 3.2 structural self-test to forbid the live-failing strict
  stop form and mutation-prove it red.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — tolerate the exact launcher's normal
  check-to-stop disappearance and structurally forbid the strict live-failing
  form.

## Guard proof

- Focused restored green: Bash syntax; 96-arm Bash 3.2 self-test with no SSH;
  all 77 analyzer tests.
- Production mutation: changing only the launcher's explicit stop action from
  `SilentlyContinue` back to `Stop` made the static Bash self-test fail at
  `Windows launcher teardown does not tolerate exact launcher self-exit`.
  Restoring it returned the focused checks green.
- Full repository gates pass: rustfmt check, strict workspace clippy, and the
  complete workspace test suite. The first suite run hit an unrelated
  temporary-daemon connection refusal in
  `delegated_pull_to_read_only_destination_is_rejected`; that exact test passed
  alone and the complete workspace rerun passed unchanged.

## Coder dispute

None.

## Known gaps

Additive staging and a complete valid live run remain.

## Reviewer comments

Tactical Grok 4.5/high review returned clean with no findings for exact range
`0c4c7f4..ef9ef0b`. It independently changed only the production launcher stop
back to strict, proved the exact guard red, restored focused green, and left a
clean exact worktree. Record:
`.review/results/ldt-4-live-f9-r1.grok-verdict.md`.
