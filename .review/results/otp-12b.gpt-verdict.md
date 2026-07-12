# otp-12b harness review — adjudication

**Reviewed commits**: `d30b1e3` (harness + doc grammar) + `772cfe6`
(quote-parity fix). **Raw review**: `.review/results/otp-12b.codex.md`
(gpt-5.6-sol, 109,042 tokens). **Verdict**: FAIL — 12 findings (4 High,
5 Medium, 3 Low); "macOS Bash 3.2 syntax and default-path quote parity
pass." All twelve verified against the script and ACCEPTED — zero
rejected.
reviewer: gpt-5.6-sol

## F1 (High) — manifest hashes inside `echo "$(…)"` again
Confirmed — a regression of the exact otp-12a F3 lesson (the zoey
script got var-captured hashes; this one didn't). Fixed: all seven
hashes captured into variables first; the Windows daemon hashes are
kept for F2's arm-swap verification.

## F2 (High) — arm swap could launch a stale exe as the wrong arm
Confirmed: `Copy-Item` failures are non-terminating and the old arm has
no handshake to catch a stale active exe. Fixed:
`$ErrorActionPreference = 'Stop'` in the launch payload and the landed
active exe's SHA256 must equal the requested arm's manifest hash before
WMI create.

## F3 (High) — both arms of a slot share the destination path
Confirmed, the sharpest catch: `rid` lacked the arm, destination sweeps
suppress errors, so the second arm could no-op onto the first arm's
leftover data and record a bogus valid time. Fixed: the arm is baked
into every rid and therefore every destination path (the zoey harness
always had this; the wrapper refactor here lost it).

## F4 (High) — derived verdicts bypassed `complete()`
Confirmed: block-2 converge rows could reference a partial block-1
median, and gap rows could mix incomplete cells. Fixed: the same-session
reference requires the block-1 counterpart complete; gap rows emit only
when all four contributing cells are complete.

## F5 (Medium) — invariance arms not doing identical work
Confirmed on both counts (nesting-shape divergence on `mw`; one arm's
container precreated outside the window while the other paid an
in-window create). Fixed: every block-2 arm gets its destination
container precreated OUTSIDE the timed window and every source is
no-trailing-slash — all four arms land the same `container/src_<w>`
tree. Block 1 keeps the otp-2w shapes verbatim.

## F6 (Medium) — `MAC_MODULE_ROOT` override could break F6-the-design-rule
Confirmed. Fixed: hardcoded to `$MAC_WORK`.

## F7 (Medium) — fail-open Windows timing
Confirmed: an errored flush read as 0 ms; pwsh noise could parse as a
plausible `ms,0`. Fixed: sentinel-framed outputs both places
(`F:<ms>:F`, `R:<ms>,<rc>:R`) with strict extraction; a flush `NA`
voids the run per the D2 rule; a client parse failure is `T_RC=99`.

## F8 (Medium) — optional references
Confirmed: block-2-only CELLS sessions silently dropped the
same-session bar; missing committed rows were silently omitted. Fixed:
committed references are mandatory (fail closed, both the F3 rows and
the F4 cross rows); an absent/incomplete block-1 counterpart emits the
registered `NO-SAME-SESSION-REF` row instead of silence.

## F9 (Medium) — untracked daemon window
Confirmed (and the WMI pid is cmd's, not the daemon's). Fixed: the cmd
pid is recorded in the launch payload itself; the verify step resolves
the daemon as the blit-daemon whose ParentProcessId is our cmd (a name
lookup tied to THIS launch); `win_daemon_stop` covers the
interrupted-between-payloads gap via the same parent-pid resolution.

## F10 (Low) — CELLS could match the CSV header
Confirmed (`CELLS=cell`). Fixed: validation greps `tail -n +2`.

## F11 (Low) — firewall rule trusted by display name
Confirmed. Fixed: an existing rule's program path, action, and enabled
state are verified; a mismatch refuses with guidance (the owner's
firewall is never silently mutated).

## F12 (Low) — vocabulary not closed; gap-row labels inexact
Confirmed. Fixed: doc vocabulary closed
(`cross-gap`/`RECORDED`/`NO-SAME-SESSION-REF` registered); gap rows
label their operands exactly (`old_push,old_pull` /
`new_mw_worst,new_wm_worst`).

## Fix commit

fix sha: `d3eae58` (`bash -n` exit 0 verified as its own step;
check-docs green; no crates/proto changes — suite stands at the
recorded 1484).
