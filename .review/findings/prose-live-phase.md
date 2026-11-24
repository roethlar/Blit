# prose-live-phase: the live row's phase was read back from its own words

**Severity**: LOW — a breach of D-2026-11-24-1 with no visible effect. The local transfer's live row colours its leading word by the phase it is showing. That phase was chosen by reading the row's own text back (`head.starts_with("copying")`, `"retrying"`, `"deleting"`, `"comparing"`), after the same function had built the text.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix prose-live-phase`)
**Reviewer**: none yet. Found by the working agent on 2026-11-24, scanning for decisions read from prose under D-2026-11-24-1.

## Evidence
`crates/blit-cli/src/transfers/local.rs` `render_live_row_styled` built `head` in a `match` on the state, then decided `shown` from `head`'s first word. Every head begins with a fixed word, so nothing misfired. But a rewording of any row ("up to date", "retrying …") silently changed its colour, and the decision was read from text the function had just written.

## What
Each branch returns its phase beside its words: `(head, file, shown)`. The mapping is unchanged:
- deleting → Deleting;
- copying and retrying → Copying;
- enumerating → Enumerating;
- comparing → Comparing;
- up to date → Enumerating;
- the copy row before the first byte → Copying.

## Guard proof
- No behaviour changes, so no guard can tell the two versions apart by output. The guards pin each branch's phase instead:
  - `each_phase_colours_its_own_word` (existing) covers four of the six branches.
  - `diff_complete_with_nothing_needed_reports_up_to_date` and `diff_complete_with_work_pending_shows_the_copy_row` now also assert the colour of their row's leading word. Before, they checked only the plain text.
- Mutations, each restored byte for byte:
  - comparing shown as copying → red;
  - deleting shown as enumerating → red;
  - up to date shown as comparing → red, a branch no guard pinned before;
  - the early copy row shown as comparing → red, likewise.
- Checks: native clippy is clean, and the CLI's local transfer tests pass.

## Files changed
- `crates/blit-cli/src/transfers/local.rs` — `render_live_row_styled`; two guards extended.

## Known gaps
- None known.
