# otp-2w — codex review adjudication

reviewer: gpt-5.6-sol (codex exec, read-only)
reviewed commits: `0c43d2a` + `ceea6ed`
raw output: `.review/results/otp-2w.codex.md`
verdict line: NEEDS FIXES (4 Medium, 3 Low)
fix commit: (appended after landing)

## F1 (Med) — drain probe fails open (`$null -lt N` is true)

**ACCEPTED.** Non-terminating PowerShell errors could leave the
counter sample `$null`, which compares less-than and counted as
quiet. Fixed: `$ErrorActionPreference = 'Stop'` in the probe, an
explicit `$null` check, and the local side now warns on ANY non-
"drained" outcome (timeout, failed probe, empty output alike).

## F2 (Med) — WMI PID discarded; kill-by-name; stale daemon masks failures

**ACCEPTED.** The launch now REFUSES if any blit-daemon is already
running (a stale one would mask a bind failure and get benchmarked in
place of the new build), records the fresh daemon's PID, and teardown
kills exactly that PID. The refusal guard proved useful immediately —
it is what protects reruns after the mid-run abort this same round.

## F3 (Med) — durability costs asymmetric across directions, undisclosed

**ACCEPTED — and measurement showed it was worse than the finding.**
Quantified: an in-window `ssh host <flush>` costs ~1.2 s to Windows
(connection + pwsh spawn + module load) and ~1.2 s to zoey
(slow-core key exchange) — landing only on push windows, on BOTH
rigs, inflating every published push median and ratio. Fix is
structural, not disclosure: durability steps now time THEMSELVES on
the destination (Stopwatch around `Write-VolumeCache`; `/proc/uptime`
around zoey's `sync`; the fsync walk reports its own elapsed) and
only that duration joins the window. **Both matrices were re-run**;
the overhead-biased sessions are kept as labeled probes
(`probe1-sshoverhead-*`, `probe5-sshoverhead-*`) and the READMEs
carry a Timing-overhead correction section. Corrected ratios:
Windows ×1.46–×2.38, zoey ×1.25–×1.75 (was reported as ×1.8–×2.7 /
×1.23–×2.19 from biased data).

## F4 (Med) — 7/12 not 8/12 cells ≤2%

**ACCEPTED.** Miscount, repeated in STATE/DEVLOG/finding doc. All
stability claims now recomputed from runs.csv per dataset (the
re-run data supersedes the original counts: Windows 4 cells ≤2% /
9 ≤9%, worst 14.5%; zoey worst 48.6% with median as the statistic);
the finding doc records the correction.

## F5 (Low) — purge-standby.ps1 unchecked API calls, leaked handle

**ACCEPTED.** Every step now checked with the causal Win32 error
surfaced (including AdjustTokenPrivileges' success-with-
ERROR_NOT_ALL_ASSIGNED case), token handle closed in a finally.

## F6 (Low) — "NEAR-SYMMETRIC" overstates the owner's designation

**ACCEPTED.** Header and README now say CLOSER-SPEC (the owner's
words) and explicitly disclaim platform symmetry.

## F7 (Low) — finding doc referenced nonexistent drain.log

**ACCEPTED.** Corrected to `drain-outcomes.txt` (the `*.log` ignore
rule had silently dropped the original from the evidence commit).

## Process note

The F3 fix introduced two of its own bugs, both caught by running:
cross-process `time.monotonic()` (0/negative windows; reverted to
wall clock with rationale) and PowerShell CRLF output breaking bash
arithmetic (stripped at the boundary). Both are documented in the
harness comments.
