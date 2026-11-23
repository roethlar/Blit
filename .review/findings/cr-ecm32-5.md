# cr-ecm32-5: a partial left behind was named in the error but not in the summary's left-in-place set

**Severity**: MEDIUM — a fresh partial left behind (cr-ecm31-3: its first look refused, or its removal failed) was named in the failure's message, but not in the words the summary reads for "this run left its own copy here". So it was missing from the summary's left-in-place set, and from what job records and retries read from it.
**Status**: Reviewed, found incomplete — `combined-r34` (Codex openreview, 2026-11-23) reviewed `70efef8a..0f912b06` — the `combined-r33` fixes — with `guard_confirmed: true`: "Checking before truncation, opening FIFOs without waiting, and recording the resolved parent and name for a Unix resume are sound changes." "I observed errors in both directions. Failed validation at a previously absent destination left an unreported zero-byte file. A source-only failure whose pathname contained the diagnostic phrase put an untouched destination in `left_in_place`. Track whether acquisition created a file, settle it on failure, and carry the outcome into the summary independently of the error text." The remaining work is cr-ecm33-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b91abdc1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r33.codex.md`. It completes cr-ecm31-3 (`89ca4eaa`).

## Evidence
"In a focused eight-byte resume, a failed first metadata read left a correctly marked nine-byte partial. The failure message named it, but the summary's `left_in_place` list was empty. Propagate the retained-file status into the path set consumed by job records and retries."

## Intake
Admitted. The set is read from the failure's reason (`INCOMPLETE_LEFT_IN_PLACE`, cr-ecm13-4), so the partial left behind is said in those words.

## What
`Resolved::fresh_as`'s message for a partial left behind begins with `INCOMPLETE_LEFT_IN_PLACE` ("an incomplete copy was left at the destination"). It does so whether or not the partial could be marked unfinished. Every record of a failure reads that phrase: the summary's left-in-place set, a job's record and the retry's selection.

## Guard proof
- `sink::tests::a_fresh_partial_left_behind_is_reported_left_in_place` (Unix): a local `--resume` with a 0600 source of 8 bytes and a 0644 destination, whose fresh partial's first look is refused (`FRESH_LOOK_FAULTS`, now visible to the sink's tests). The file fails, its reason carries `INCOMPLETE_LEFT_IN_PLACE`, and the partial is 9 bytes, marked unfinished.
- Mutation, restored byte for byte: the message without those words → red.
- All 1,158 `blit-core` tests pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as`'s message for a partial left.
- `crates/blit-core/src/copy/mod.rs` — `FRESH_LOOK_FAULTS` for tests.
- `crates/blit-core/src/remote/transfer/sink.rs` — guard.

## Known gaps
- None known.
