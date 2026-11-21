# cr-ecm15-3: a record that did not note where it wrote was refused a retry it could have had

**Severity**: MEDIUM — a retry of an older record is refused though both readings of its arguments write to one place
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm15-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-ecm14-4 (`62b12719`).

## Evidence
"For `copy source/child/.. output`, I verified that both an absent and an existing `output` received the file directly. The legacy retry still refused because the nesting bit was missing. This source has no basename to append, so the resolver can establish the destination under either interpretation."

Cause. cr-ecm14-4 refuses to retry a record written before records noted whether the copy went under the destination, wherever that answer turned on what the disk held when the copy ran (`nesting_asks_the_disk`). It asked only whether the ANSWER could differ, not whether the PLACE written to differs with it. Going under the destination means joining the source's last name to it, and `source/child/..` has no last name: under either answer the destination is used as it stands.

## Intake
Admitted as the reviewer put it.

## What
`transfers::resolution::nesting_changes_the_destination(raw_source, raw_dest, src, dst)`: the answer turns on the disk AND the source has a name to put under the destination (`source_basename`, the same function the resolver joins by). `blit jobs retry` refuses a record without the note only where that holds.

## Guard proof
- `transfers::resolution::tests::nesting_asks_the_disk_only_where_its_answer_can_change` (extended): for `src`, `/a/b/src`, `source/child/..` and `..` the answer turns on the disk; the place changes with it for the first two only — and the guard shows that by resolving the destination under both answers and comparing the two places, not by the new function alone. A source that is its contents (`src/`, `/`) asks nothing and changes nothing.
- `crates/blit-cli/tests/job_retry.rs::a_record_that_did_not_note_where_it_wrote_is_not_guessed_at` (extended, end to end): a fifth case, `copy src/child/.. backup` with `backup` there, its record stripped of the note as the four others are — the retry runs and lands the file in `backup` itself. The first case (`src` to `backup`, no slash on either) is still refused.
- Mutations, each restored byte for byte: the name not asked for, as it was (both guards red); the name asked for the wrong way round (both red).

## Files changed
- `crates/blit-core/src/transfers/resolution.rs` — `nesting_changes_the_destination`; guard.
- `crates/blit-cli/src/jobs.rs` — the refusal asks it.
- `crates/blit-cli/tests/job_retry.rs` — guard.

## Known gaps
- A record that is refused is still refused whole; nothing is recovered from what the disk holds now (cr-ecm14-4's decision, unchanged).
- A source on a daemon with no last name (a module's root) is covered by the same function and has no end-to-end guard of its own.
- Linux only; nothing here is conditional on the platform.
