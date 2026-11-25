# ci-windows-case-note: three case-note guards assumed a host that reads paths as Unix does

**Severity**: MEDIUM — three guards fail on Windows; the code does there what its design says (admitted HIGH while its cause was unknown).
**Status**: Fixed (the guards); awaiting the next CI run on Windows
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-windows-case-note`)
**Reviewer**: none — found by GitHub CI run 45508559853 at `b6e6fab9`, 2026-11-24 — the first CI since 2026-10-24, after the owner pushed to GitHub; for the next openreview.

## Evidence
Three tests failed on Windows.

`case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed`:
- gave `["1*A*2"]`;
- expected `["1xAx2"]`.

`the_case_note_shows_only_spellings_ignore_case_matches`:
- gave `["[a-c_]x", "[A-C_]X"]`;
- expected `["ax", "[A-C_]X"]`.

`the_case_note_says_what_the_first_lettered_pattern_matches`:
- gave "matches Cache/** only … cache/**, CACHE/**";
- expected "matches exactly as typed … cache/x, CACHE/x".

Each asks for the Unix reading (`windows = false`) and expects what checking it with the filter gives. `Spellings::of_role` puts a reading to this machine's filter only where the reading is this machine's own. Under the other platform's reading nothing can be asked, and its rules stand unchecked. That is the documented design, and on Windows the rule's spellings are what was given.

## Intake
Admitted. The fault is the guards' assumption of a host, not the code under them.

## What
The three tests are marked `#[cfg_attr(windows, ignore = "…")]`, with the reason in the attribute and a comment. They are of the Unix reading as a host that reads paths so checks it, and of the Windows reading by its rules. On Windows both are the other way about. On Linux and macOS they run as before.

## Guard proof
- Checks:
  - Windows clippy is clean;
  - all 21 `case_fold::` tests pass on Linux, these three among them.
- Proof is the next CI run on Windows: this host is not Windows.

## Files changed
- `crates/blit-core/src/case_fold.rs` — the three guards' attributes.

## Known gaps
- On Windows no test asserts these patterns' spellings to the letter under Windows' own reading, checked. `each_claim_is_checked_against_the_filter` checks there that every claim the note makes is true of the filter.
