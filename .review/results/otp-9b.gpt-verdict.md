# otp-9b — codex review adjudication

reviewer: gpt-5.6-sol (codex exec, read-only)
reviewed commit: `b2fd876`
raw output: `.review/results/otp-9b.codex.md`
verdict line: NEEDS FIXES (4 findings; test-count accounting confirmed
exact: 9 retired + 3 new, 1558 → 1552)
fix commit: `1ce73b5` (4/4; suite 1552 → 1555)

## F1 (High) — `require_complete_scan` forwarded but never enforced by the session

**Claim** (delegated_pull.rs:319): only mirror checks `scan_complete`;
a remote→remote MOVE (which sets the flag) can omit unreadable source
entries, report success, and the CLI then deletes the source — silent
data loss.

**Adjudication: ACCEPTED.** Verified: `destination_session`'s
`ManifestComplete` arm gated the refusal on `mirror_enabled` alone
(mod.rs). This is a SESSION gap, not a delegated one — the old paths'
R49-F2 enforcement had no session equivalent, and otp-10's verb cutover
would have inherited it too. Fixed at the same abort point as the
mirror guard: `open.require_complete_scan && !scan_complete` now
refuses with `SCAN_INCOMPLETE` before any transfer. Pinned by
`incomplete_scan_refused_when_completeness_required` (scripted source
peer, bounded wait); guard proof: disabling the check makes the test
fail at its timeout (the destination proceeds instead of refusing).

## F2 (High) — mirror pass in one `spawn_blocking` outlives cancellation

**Claim** (mod.rs:2809): dropping the session future (client
disconnect, `CancelJob`) cannot stop a started blocking task, so
deletions can continue after the job is recorded cancelled; the retired
async delete loop stopped at its next await.

**Adjudication: ACCEPTED.** Verified — `spawn_blocking` detaches; the
`.await`'s cancellation abandons, not aborts, the task. Pre-existing
since otp-6b (any served mirror session), but the delegated reroute
makes CancelJob-on-a-mirroring-row a first-class production path, so it
lands here rather than as residue. Fixed with a drop-guard
(`AbortFlagOnDrop`) whose `Drop` flips an `AtomicBool` the pass checks
before every filesystem op — a dropped future stops the deletions at
the next entry. Pinned by
`mirror_delete_pass_aborts_on_the_cancellation_flag` (pre-set flag ⇒
zero deletions + error; un-aborted control deletes); guard proof by
disabling the check → test fails.

## F3 (Medium) — non-special open failures lose the NEGOTIATE phase

**Claim** (session_client.rs:294): open failures other than
Unimplemented/PermissionDenied map to `Internal` → phase TRANSFER,
while the old typed boundary classified every pre-response RPC failure
as NEGOTIATE; open-phase identity must stay structural.

**Adjudication: ACCEPTED.** Fixed with a marker type:
`TransferOpenRefusal(SessionFault)` wraps every open-time failure (the
inner code still names the closest session meaning), and the handler's
new pure `session_error_phase` classifies: any `TransferOpenRefusal` →
NEGOTIATE; mid-session faults by code; bare transport errors →
TRANSFER. `CONNECT_SOURCE` stays the separate structural connect step
(codex confirmed it correct). Pinned by
`session_error_phase_classifies_structurally` (incl. the
non-special-code open failure); guard proof by removing the marker arm
→ test fails.

## F4 (Medium) — updated fakes keep legacy PullSync behavior, leaving the reroute unguarded

**Claim** (remote_remote.rs:507, jobs_lifecycle.rs:351): both fakes
model equivalent behavior on BOTH RPC surfaces, so reverting delegation
to `pull_sync_with_spec` would still pass every affected test.

**Adjudication: ACCEPTED.** Exactly the vacuous-guard shape the loop
exists to catch. Fixed: both fakes' `pull_sync` now returns
`Unimplemented("delegation no longer uses PullSync (otp-9b)")` — a
reverted (pre-session) delegated path fails the refusal-wording pin
and the cancel-of-active-job pin instead of passing silently. Both CLI
suites re-run green on the current code.

## Also from the raw output

Codex confirmed the spec→options and summary mappings, the
`CONNECT_SOURCE` phasing, cancellation through core.rs's three-way
select, the no-payload-bytes contract, and the exact test-count
accounting.
