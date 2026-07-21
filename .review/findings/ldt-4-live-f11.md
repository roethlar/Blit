# ldt-4-live-f11 — reconcile an authenticated Windows evidence reservation after ambiguous SSH exit

**Severity**: MEDIUM — a Windows evidence directory can be created successfully
while the SSH call reports failure, voiding an otherwise healthy registered
session before the transfer starts.
**Status**: Closed — fixed, mutation-proved, tactically reviewed clean, and validated by the complete 96-arm rig-W run.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `96a4e3b03caf43ee368efadc779e3324248067f6`

## Evidence

Exact reviewed and staged harness `c621e33` completed 21 byte-identical arms in
retained session `ldt4-20260721T221543Z-c621e33fd9df`. Before arm `ldt4-022`
started, `prepare_arm_dirs` created the two local evidence directories, then its
Windows SSH reservation returned nonzero without a PowerShell error. The
harness failed closed at `ldt4-022 cannot reserve Windows arm evidence`.

A read-only audit found the exact remote arm path had nevertheless been created
at the failure time. It is a plain, empty Windows directory with no link type:
`D:/blit-test/ldt4-sessions/ldt4-20260721T221543Z-c621e33fd9df/logs/ldt4-022`.
The remote session has exactly 22 arm directories, matching the 21 completed
arms plus this committed reservation effect. The two local `ldt4-022` evidence
directories are also empty.

`runs.csv` contains its header plus 21 provisional rows and
`runtime-gates.csv` contains its header plus 11 accepted gate samples. No arm
22 transfer started, no arm 22 row was appended, no analyzer ran, and the
session has no analysis output. Every provisional row is invalid and ungraded.

Cleanup succeeded. q and Windows have no port-9031 listener, Blit process, or
session harness process. Windows restored the prior active daemon at SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`
and retained the tested daemon at SHA-256
`ae414e649cf64f042f9d2a61639371c57d2fc3107cc426fe5a27a057b6630322`.

## Predicted observable failure

If authenticated SSH loses the final command status after Windows commits the
directory creation, the one-shot reservation treats the transport result as
authoritative and voids even though its exact safe postcondition exists.
Blindly retrying the create is not safe because the existing-directory refusal
cannot distinguish this invocation's committed effect from a stale collision.

## What

Make the remote evidence reservation safely reconcilable without weakening its
pre-existing-path refusal. Bind a marker inside the new directory to the exact
session and arm. If the primary SSH call is ambiguous, accept only a separate
authenticated read-back proving the exact plain directory, exact plain marker,
exact marker contents, and no unexpected directory contents.

## Approach

- Preserve the first command's refusal when the target already exists.
- After creating the directory, write and verify an exact session-and-arm
  reservation marker before reporting success.
- On nonzero SSH return, reconnect once outside timed transfer measurement and
  reconcile only the complete marker-bound postcondition. Missing, mismatched,
  linked, non-plain, or extra contents remain terminal and void the session.
- Extend the Bash 3.2 structural self-test to require the marker and the strict
  ambiguous-result reconciliation; mutation-prove the new guard red.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — nonce-bound durable reservation marker,
  authenticated ambiguous-result reconciliation, and structural guard.

## Guard proof

- Focused restored green: Bash syntax; 96-arm Bash 3.2 self-test with no SSH;
  all 77 analyzer tests.
- Production mutation: inverting the exact marker-content comparison from
  `-cne` to `-ceq` made the static self-test fail at
  `Windows arm reservation postcondition is not exact and marker-bound`.
  Restoring the strict mismatch refusal returned the focused checks green.
- Full repository gates pass: rustfmt check, strict workspace clippy, and the
  complete workspace test suite.

## Coder dispute

None.

## Known gaps

None for this finding. Exact harness `96a4e3b03caf43ee368efadc779e3324248067f6`
includes the fix and completed all 96 valid arms. Retained evidence is recorded
at `docs/bench/ldt4-rigw-2026-07-21/`.

## Reviewer comments

Tactical Grok 4.5/high review returned clean with no findings for exact range
`6d3a0bc..96a4e3b`. It independently proved production `-cne`→`-ceq` red,
restored focused green, and left a clean exact worktree. Record:
`.review/results/ldt-4-live-f11-r1.grok-verdict.md`.
