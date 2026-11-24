# cr-ecm36-5: a resume record's close did not look at what stood at the name

**Severity**: MEDIUM — a run reports the opposite of what it left. cr-ecm35-4 made a resume record its source fails say "left" for a partial this run made, but it judged that from history, not from the destination:
- With a block's I/O error held, the held failure's "nothing left" won, though the partial this run made stayed at nine bytes for an eight-byte source.
- With the partial removed before the source's abort, the record still said it was left.
**Status**: Fixed on Unix; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm36-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-4 (`6da696f5`).

## Evidence
"With an injected block I/O error followed by a source abort, a nine-byte partial remained and the receiver reported `left_in_place = false`. Removing the partial before an otherwise identical abort produced `true` for an absent file. Use the held file's identity and the destination's state at close alongside the earlier error."

## Intake
Admitted as the reviewer put it.

## What
`FsTransferSink::fail_resume_record` looks at what stands at the name as the record closes. The name is resolved as every resume record's is, and a link there is followed.
- **The hold holds a partial.** It is left when the name still leads to that very file (by its identity) and either this run made it or the failure held for it says so. Gone from the name, nothing of it is there.
- **The hold holds nothing.** The held failure's own word stands, as before.

The reason keeps the source's words first, then the held failure's, then the account of what stayed.

## Guard proof
- `sink::tests::a_failed_resume_record_says_what_stands_as_it_closes` (Unix):
  - a whole-file hold's fresh partial with a block's failure held is flagged left at nine bytes;
  - one removed from the name before the record closes is not flagged.
- The other failed-record guards (cr-ecm22-5, cr-ecm34-4, cr-ecm35-4) still hold.
- Mutations, each restored byte for byte:
  - the held failure's word wins → red;
  - what stands not looked at → red.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `fail_resume_record`; guard.

## Known gaps
- A name that cannot be resolved as the record closes is taken as holding nothing of the run's.
