# cr-ecm34-5: a failure's identity and disposition were lost across the summary's conversions

**Severity**: HIGH — a saved retry can address a different file. Two losses:
- `FileFailure`'s wire form had no left-in-place flag, so a round trip turned `true` into `false`.
- The run's totals (`RunTotals`) dropped the summary's `failed_paths_raw`. A failure past the summary's 64-reason cap, in a log that heard no name live, was logged with neither its raw name nor its leftover flag.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "With 65 failures and no live name event, the saved record claimed a complete list while losing the 65th filename's raw bytes. In a separate probe, `dup\xff` and `dup\xfe` produced only one log entry when the reason cap fell between them. Join reasons to exact identities and use that result for both writers." "A summary-only failure missing from a truncated leftover list was logged as `Some(false)`, which can make a recovered `--ignore-existing` retry skip its own partial. Conversely, an old unknown event kept the entire list marked incomplete even after that file landed. Preserve unknown for the affected identity and clear it when a later copy settles that identity." The remaining work is cr-ecm35-3 and -5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `183408ae`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-5 (`33808972`) in part.

## Evidence
"A wire round trip changed a reported failure's flag from `true` to `false`. The separate leftover lists cover many cases, but `RunTotals` drops `failed_paths_raw`. In a summary-only log with 65 failures, the 65th file had a non-UTF-8 name and was left in place. Its log entry had neither its raw name nor its leftover flag. A saved retry can consequently address a different filename. Carry the raw identities past the 64-reason cap and preserve the disposition of each reported failure."

## Intake
Admitted as the reviewer put it.

## What
- **The wire carries the flag.** `FileFailure` gains `bool left_in_place = 4`, so the contract goes to v30. Peers must speak the same contract, so a peer of v29 is refused at session open, as for any other bump. `to_wire` writes the flag and `from_wire` reads it. Source-side wire failures (scan failures, skips) carry `false`: nothing of theirs is at the destination.
- **The totals carry each path's bytes.**
  - `RunTotals.failed_paths_raw` holds one entry for every failed path: the exact bytes of a name that is not text, at its place, and nothing at the place of one that is.
  - It is filled from the wire summary (`raw_at_their_places`), the local summary and the delegated summary.
  - `merge` keeps the bytes at their places, so a part that carried none does not misplace another's.
- **The run log's end pass.** A failure the summary named without a reason is logged by the bytes the summary carried for its path when there are any, else by the name heard live. Its left-in-place flag is then read from the raw set by those bytes.

## Guard proof
- `sink::tests::a_failure_keeps_its_disposition_across_the_wire`: a left-in-place failure keeps its flag and its bytes through `to_wire`/`from_wire`, and another keeps `false`.
- `run_log::tests::a_summary_only_failure_is_named_by_its_own_bytes`:
  - a summary-only log names the raw-named failure by its bytes with `left_in_place: Some(true)`, and the text-named one with `Some(false)`;
  - merged after a part without bytes, the bytes stay at their place.
- Mutations, each restored byte for byte:
  - the wire drops the flag → red;
  - the summary's bytes not carried → red;
  - the log ignores the summary's bytes → red;
  - a merge misplaces the bytes → red.
- Checks:
  - native clippy is clean;
  - the workspace passes on Linux: 2,079 tests over 57 binaries.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FileFailure.left_in_place`.
- `crates/blit-core/src/transfer_session/mod.rs` — contract v30; the source-side literals.
- `crates/blit-core/src/remote/transfer/sink.rs` — `to_wire`/`from_wire`; guard.
- `crates/blit-core/src/run_log.rs` — `RunTotals.failed_paths_raw`, `raw_at_their_places`, `merge`, the end pass; guard.
- `crates/blit-cli/src/run_log.rs` — the local and delegated totals.
- `crates/blit-core/src/transfers/failures.rs`, `crates/blit-core/tests/source_side_containment.rs` — test literals.

## Known gaps
- `CHANGELOG.md`'s 1.0.0 entry still names contract 18, though the contract has moved since. Bringing it up to date is not this finding's.
