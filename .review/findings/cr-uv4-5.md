# cr-uv4-5: the bound on the accounts a daemon keeps for moves did not hold

**Severity**: MEDIUM — one move's account could take the memory the bound was written to deny it, and put out every other move's
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv4-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-uv3-1 (`57e8ee24`).

## Evidence
"A single account larger than the advertised 1 GiB limit evicts every older account and is then inserted in full. Bound the incoming account and the number of retained tickets."

Cause. `moved::keep` made room for a new account by dropping the oldest kept ones until the files held, with the new account's, were within the bound — or until nothing was left to drop. An account larger than the bound by itself emptied the shelf and was then kept all the same. And nothing counted the accounts: a session that sent no file had a ticket that cost a map entry and held no file, so any number of them fitted.

## Intake
Admitted as the reviewer put it.

## What
- **An account that by itself holds more files than may be kept is not kept**, and nothing is put out for it (`keep_within`, asked first). Its session is given an empty ticket; the move that asked for it copies, is told by its own side that "the daemon gave no account of what it sent", removes nothing and exits non-zero, with the source whole. The daemon logs it.
- **The number of accounts kept is bounded** as well as the files they hold: 4,096 accounts, 64 million files between them (`BOUNDS`). The oldest make room for either.
- Oldest is the order they were kept in, by a count, not by the clock.
- The bounds are a value handed to the keeping, so that a guard can keep within small ones on a shelf of its own.

## Guard proof
- `moved::tests::what_is_kept_of_moves_stays_within_its_bounds`, on a shelf of its own with bounds of ten files and three accounts: an account of eleven gets no ticket and both older accounts are still there; one that fits only without the oldest puts the oldest out and no other; one of exactly ten is kept alone; and of five accounts holding nothing, the last three are kept.
- Mutations, each restored byte for byte: the account over the bound kept (and pushing the others out); no bound on the number of accounts; the newest making room in place of the oldest; no bound on the files — four, each red.

## Files changed
- `crates/blit-core/src/moved.rs` — `Bounds`, `BOUNDS`, `keep_within`; `Kept::order`; guard.

## Known gaps
- **A move of more than 64 million files from a daemon removes nothing** in one go: it copies, and says why it removed nothing. It is not split into several accounts.
- The files bound counts files, not bytes: an account whose session is still open holds 32 bytes a file, one that has ended 16 — two gibibytes at the bound if every session were open at once.
- That the session sends the empty ticket on, and the asker refuses the removal for it, is the path cr-uv3-1 guards (`a_removal_for_a_move_is_refused_without_the_account_of_what_was_sent`, and the asker's own refusal of an empty ticket); no test here drives a 64-million-file account through a session.
- The bounds are constants. Nothing reads them from the daemon's configuration.
