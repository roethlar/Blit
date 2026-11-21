# cr-uv5-4: a failed name struck two files from a move's account

**Severity**: MEDIUM — a move of two names of one text lands both, leaves one at the source and exits 1
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv5-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-uv4-1 (`c4241da0`).

## Evidence
"When valid UTF-8 `dup�` landed first and the name ending in byte `FF` landed on retry, both arrived, but cleanup left `dup�` behind. The receipt subtracts both the raw name and its display spelling. Use the raw bytes at that position when present, falling back to text only for that entry."

Cause. cr-uv4-1 strikes from a move's account what the destination's summary says did not land. The summary gives each failed path as text (`failed_paths`) and, at the same place in a second list, the exact bytes of a name that is not text (`failed_paths_raw`). `not_landed` struck every text AND every list of bytes. The text a name that is not text is shown as is the NAME of another file; where that file was in the same move and had landed, it was struck with the one that failed, and stayed at the source for good — the retry that landed the other added only that other to its own account.

## Intake
Admitted as the reviewer put it, with one case added that the reviewer's remedy would have made unsafe: two names of one text that BOTH fail are one entry in the summary's list, carrying one of their identities (`SinkOutcome::failed_raw` holds one for a text). Striking that one identity alone would leave the other failed file in the account, to be removed though it never landed. Such a summary counts more failures than it names paths, and is now taken as one that cannot say what landed.

## What
`not_landed` gives ONE identity for each failed path: the bytes at its place in `failed_paths_raw` when there are any — a name that is not text, or a name refused for sharing its text, which is recorded by its own bytes — and its text otherwise. And it gives nothing at all, so that the account is dropped and no removal can use it, when the summary's list is cut short (as before) or shorter than its count of failures.

## Guard proof
- `transfer_session::…::what_did_not_land_is_one_identity_a_failed_path_or_nothing`: the reviewer's case — the name that is not text failed, and the file whose name is its text is not struck; the other way round; a summary of text names with no list of bytes; none failed; a list cut short; one path named for two failures counted; failures counted and none named.
- `crates/blit-cli/tests/remote_move.rs::a_move_of_two_names_of_one_text_removes_both` (end to end, the reviewer's case): a module holding `dup` and the byte `FF`, `dup` and U+FFFD and a third file, moved from a daemon with one retry pass — exit 0, both names at the destination with their own bytes, nothing left at the source.
- Mutations, each restored byte for byte: every text and every list of bytes struck, as it was (both guards red); a summary that names fewer paths than it counts failures taken at its word (the unit guard red).

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `not_landed`; guard.
- `crates/blit-cli/tests/remote_move.rs` — guard.

## Known gaps
- Two names of one text that both fail in one session cannot be told apart in the summary at all: the wire gives one entry for a text. Nothing is removed for such a session; a later one that lands them both has its own account.
- The count and the list are compared by number only.
- The end-to-end guard returns early on a filesystem that refuses a name that is not text.
