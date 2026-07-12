# otp-12 design review — adjudication

**Reviewed commit**: `045da4a` (adds `docs/plan/OTP12_ACCEPTANCE_RUN.md`).
**Raw review**: `.review/results/otp-12-design.codex.md` (gpt-5.6-sol,
125,286 tokens). **Verdict**: CHANGES REQUIRED — 7 findings (3 High,
4 Medium); "D4 timing, D6 staging, D7 arithmetic, and the requested
endpoint/--force-grpc/delegation/handshake facts otherwise check out."
reviewer: gpt-5.6-sol

Note on timing: between review dispatch and adjudication the owner ruled on
the doc's Q1 in session (recorded as **D-2026-07-12-1**, commit `bfb9670`,
with propagation to `ONE_TRANSFER_PATH.md` criterion 2 and the doc's Q1
section). F1's adjudication reflects that.

## F1 (High) — rig W is APFS↔NTFS, not "same filesystem class"; the residue rule weakens the parent's unconditional bar

**OVERTAKEN BY OWNER DECISION (accepted in spirit; residual fix applied).**
The finding's substance — an agent must not weaken an acceptance bar — was
honored by process: the exact question was surfaced to the owner in plain
English (the doc's Q1) and the owner ruled "yes" before this adjudication
(D-2026-07-12-1). The fleet contains no same-fs-class 10 GbE pair; the
owner designated Mac↔Windows as the cross-direction rig on 2026-07-10
(`docs/bench/otp2w-baseline-2026-07-10/README.md` §Status). Criterion 2's
evaluation rule is now annotated in the parent (bfb9670). Residual fix in
this commit: parent criterion 1 gains the same instantiation note (the
designated pair + why invariance A/B is valid there: both arms of a pair
share endpoints, so endpoint asymmetry cancels within the pair).

## F2 (High) — same-session old rerun as THE hard reference; a slower old rerun could loosen the fixed bar

**ACCEPTED.** D2 rewritten: a clean converge-up PASS now requires the new
arm ≤ ×1.10 against **BOTH** references — the same-session interleaved old
arm AND the committed 2026-07-10 baseline median. A cell that passes
same-session but fails the committed reference is recorded
`FAIL-REFERENCE-DRIFT` and triggers one pre-registered fresh session re-run
for that cell; if the drift persists it stands as a recorded failure for
the otp-13 walk — never silently excused by rig-state drift.

## F3 (High) — tolerance compounding: arm B could reach 1.21× the old bar

**ACCEPTED.** D2 rewritten: EVERY unified arm median of a data direction
(both initiators on rig W, both blocks) must independently satisfy the
converge-up bars for that direction. The invariance ratio is an additional
constraint, not a substitute ceiling.

## F4 (Medium) — otp-12d scheduled acceptance-checkbox edits

**ACCEPTED.** Contradicted the doc's own "declares nothing" and Earned
Practices (checkpoints are owner-only). 12d now assembles the matrix only;
checkbox flips happen at otp-13 with the owner.

## F5 (Medium) — fixed ABAB confounds arm with within-pair order

**ACCEPTED.** D1 rewritten: deterministic counterbalanced order
`A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs) so each arm leads half the
pairs; pre-registered, no randomness (scripts stay reproducible).

## F6 (Medium) — Mac→Windows arms read different physical source trees

**ACCEPTED.** D3/D5 rewritten: `MAC_MODULE_ROOT` defaults to `$MAC_WORK`
itself — the Mac daemon exports the exact fixture trees arm A pushes, so
arm B's pull reads the same inodes (`$MAC_HOST:9031:/bench/src_<w>/`). No
copy/move of fixtures on the Mac.

## F7 (Medium) — undrained runs flagged but kept; failed runs excluded without replacement

**ACCEPTED.** D2/D5 rewritten: a run with nonzero exit OR an undrained
window voids its whole interleave PAIR (both arms at that position); the
pair is re-run (appended, same counterbalance position) until RUNS valid
pairs exist, capped at 2×RUNS pair attempts; at the cap the cell is
recorded `INCOMPLETE` with the drain log — surfaced, never a silent pass
or a short median.

## Fix commit

fix sha: `92e1d51` (docs-only; check-docs gate green). Related same-day
records: decision `bfb9670` (D-2026-07-12-1, landed between review
dispatch and adjudication — see F1).
