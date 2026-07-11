# otp-10b-2 — codex verdict adjudication

reviewer: gpt-5.6-sol (codex exec, read-only; raw output
`.review/results/otp-10b-2.codex.md`)
slice commit: `2014782`
verdict: NEEDS FIXES — 3 High, 2 Med, 1 Low
adjudication: **5 accepted (1 in part) + fixed, 1 deferred**
fix sha: (appended below after the fix commit)

## F1 (High) — peer fault unread during the mirror delete pass

**Accepted.** Verified: the DESTINATION's SourceDone arm awaited the
purge's `spawn_blocking` bare — a `CancelJob` on the serving source
arrives as a framed `SessionError{CANCELLED}` that sat unread while
deletions ran to completion behind a cancelled session (the otp-9b F2
drop-guard only covers THIS end's future being dropped). Fixed: the
purge races ONE control-lane read (biased frame-first, so an
already-queued cancel aborts before the first delete); any lane event
flips the abort flag, the pass winds down at its next filesystem op,
and the peer's fault owns the outcome. Pinned by
`cancel_frame_during_mirror_purge_aborts_the_deletions` (scripted
peer queues CANCELLED behind SourceDone over a 2000-file extraneous
tree: fault surfaces as CANCELLED, survivors remain). Guard proof
(mutation X): gating the recv arm off reverts to the bare await — the
pin fails (clean summary, all files deleted).

## F2 (High) — delegated remote→remote move kept the SizeMtime skip

**Accepted.** Verified: `run_remote_to_remote_direct_inner` built its
wire options from the raw flags, so a delegated MOVE rode the
SizeMtime default — the dst daemon skips a same-size changed file and
the CLI then deletes the remote source: the exact otp-10a F1 loss on
the one route the slice missed. Fixed: options extraction
`delegated_pull_options(args, filter, mirror, move_verb)` forces
`ignore_times` for a move unless `--checksum` (the spec builder maps
ignore_times with top precedence, reproducing `move_comparison_mode`
through the old wire spec) and owns `require_complete_scan`; the TUI's
`f3_pull_options` gets the same defense (its delegated move is
rejected upstream today). Pinned at the wire level:
`delegated_move_transfers_unconditionally_on_the_wire` (spec
compare = IGNORE_TIMES / CHECKSUM) + the copy-passthrough control.
Guard proof (mutation Y): dropping the forced ignore_times fails the
pin. (No twin-daemon R2R move binary e2e exists yet — the fix is
argument mapping only; the delegated session itself is otp-9's
covered surface. Noted as a 10c candidate.)

## F3 (High) — gate texts pointed at a lossy remediation

**Accepted in part.** The real half: the new `--size-only` rejection
recommended "plain `blit move` (which transfers every file
unconditionally)" while the LOCAL move route still configured a
SizeMtime compare — and the rewritten `--force`/`--ignore-times`
texts made route-dependent claims. Fixed: local move (CLI
`build_local_options` move branch; TUI `perform_local_move`) now maps
through the move rule explicitly (IgnoreTimes / Checksum), making the
texts' "every route transfers unconditionally" claim true by
construction; texts rewritten to the uniform truth (pinned prefixes
kept; cli_arg_safety_gates green incl. the R55 negative asserts).

The claimed LIVE data loss did **not** reproduce: probed the actual
binary — a plain local copy OVERWRITES a same-size same-mtime changed
file (the non-mirror local path copies unconditionally; the compare
mode is consulted by the mirror/diff machinery, not the plain copy
walk). So the pre-fix local move was not losing data today; the codex
scenario becomes real exactly when otp-11 puts local transfers on the
session, whose diff DOES skip that cell under SizeMtime. The explicit
mapping + two pins
(`local_move_lands_source_bytes_over_same_size_same_mtime_destination`,
`perform_local_move_lands_source_bytes_over_matching_metadata`) are
therefore REGRESSION pins for otp-11, documented as such in place —
they pass pre-fix (behavior already held) and exist to survive the
local cutover; the probe result is recorded in the code comments.

## F4 (Med) — every served session counted/exposed as Push, empty endpoint

**Accepted.** Verified: `core.rs::transfer` registered
`ActiveJobKind::Push` + `inc_push()` + a started event with empty
module/path for BOTH roles — post-cutover that misreports every pull
(kind, metrics, jobs rows, events); the code's own comment deferred
the taxonomy "until cutover", which is now. Fixed: an `on_open` hook
fires at the open's successful resolve — the first point the daemon
knows the role and endpoint: it sets the row's kind + module/path
(new `ActiveJobUpdater`, the `'static` sibling of
`ActiveJobGuard::set_endpoint`), counts `inc_pull`/`inc_push` by
role (a handshake-refused session now counts nothing), and emits
`TransferStarted` with real values (strictly better than the old
streaming RPCs' empty-endpoint events). A session that dies pre-open
emits the placeholder started right before its finished event, so
subscribers keep the paired sequence. Kind taxonomy: daemon-as-SOURCE
= `PullSync` (the old pull verbs' kind — CancelJob-capable, wire
`TransferKind::PullSync`), daemon-as-DESTINATION = `Push`. Pinned by
`served_sessions_record_their_kind_and_endpoint` (both roles, module
asserted from the recents ring). Guard proof (mutation W2): mapping
the source-resolver hook to Push fails the pin.

## F5 (Med) — progress monitor lives through the in-session purge

**Deferred.** Verified real as a display artifact: the old pull tore
the monitor down before its client-side purge (the a0-pull-execution
round-2 lifecycle), while the session runs deletions inside the one
call — so a long mirror purge emits zero-delta `[progress]` ticks and
dilutes the final avg-throughput line. It is not a correctness or
data risk, and no w6-1 event exists to mark the purge phase — tearing
the monitor down mid-session is not possible from the verb layer.
The structural fix is the already-named M-C `AppProgressEvent`
reshape (a phase-bearing event family); filed to the STATE post-REV4
residue list. (The push verb has had the identical shape since
otp-10a — the purge runs daemon-side there, same monitor lifetime.)

## F6 (Low) — TUI F1 builder bypassed the one mapping

**Accepted.** Fixed: `build_f1_push_execution` maps through
`comparison_mode`/`move_comparison_mode` (flags default — the TUI has
no compare toggles); `build_f3_pull_execution` already did. Values
unchanged, pinned by the existing builder tests + the compare-unit
precedence table; no separate mutation run (the pins predate the
refactor and still pass).

## Fix-round validation

fmt + clippy clean; full workspace suite re-run green (count in the
fix commit); mutations X/Y/W2 each run to a FAILING pin and restored.
