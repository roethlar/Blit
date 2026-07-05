# ONE_TRANSFER_PATH plan — codex review adjudication

**Commit reviewed**: `06e5413` (plan draft + D-2026-07-05-1)
**Raw review**: `.review/results/one-transfer-path-plan.codex.md`
**Reviewer**: gpt-5.5 (codex exec, read-only sandbox)
**Verdict line**: NEEDS FIXES before Active flip (5 findings; no quiet
re-litigation of settled decisions found)
**Fix commit**: appended to this file after the fix lands.

## Findings

1. **High — REV4 mixed-version-compat conflict — ACCEPTED.** Verified:
   REV4 §Constraints (line 86) requires "mixed old/new peers must
   negotiate down"; the draft deleted `Push`/`PullSync` without naming
   that rule superseded — two live authorities in conflict. Fixed: the
   plan's Supersedes header + Non-goals name the supersession
   (effective only at the cutover slice); REV4's constraint text
   annotated in place; D-2026-07-05-1's Supersedes line extended.
2. **Medium — bounded-unilateral dial contract not carried — ACCEPTED.**
   D-2026-06-20-1/-2 and REV4 Design §4 make the sender-owned,
   receiver-profile-bounded dial a standing contract; the draft's "sf-2
   shape correction is the only stream policy" didn't restate it.
   Fixed: new Constraints bullet (profile travels DESTINATION→SOURCE
   regardless of initiator); otp-1 contract deliverables name it; the
   decision entry states the dial contract is NOT superseded.
3. **Medium — `DelegatedPull` RPC fate unstated — ACCEPTED.** Verified:
   it is a client↔daemon trigger + progress-relay stream (payload
   bytes flow over the daemon↔daemon session), but the draft's
   deletion proof was silent about it. Fixed: Design states its fate
   (trigger + relay only, handler shrinks to authorize/spawn/relay);
   the deletion-proof acceptance criterion now requires the
   no-payload-bytes assertion.
4. **Medium — converge-up not observable — ACCEPTED.** The constraint
   had no measurement: a symmetric-but-slower path would have passed
   acceptance. Fixed: new slice otp-2 (corrected symmetric-fs harness
   + OLD-path per-cell baseline recorded pre-cutover); new acceptance
   criterion (every unified cell ≤ the better old direction + noise);
   final rig run (otp-12) compares against that baseline. Slices
   renumbered 1..13.
5. **Medium — resume ordering underspecified — ACCEPTED.** Fixed:
   Design adds the resume-ordering RELIABLE exception (block-map
   exchange strictly precedes block send per file; stale partials
   fall back to full transfer; parallel to ue-r2-1g's single-stream
   exception); otp-1 pins the phase ordering in the wire contract,
   otp-7 pins stale-partial/mid-failure cases in tests.

Nothing rejected, nothing deferred.

reviewer: gpt-5.5
