# cr-uv11-1: `find -type … -delete` removed whatever replaced the entry found

**Severity**: HIGH — `find -type l -delete` removes a regular file that took the link's name after the walk tested it
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv11-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-uv10-2 (`55f43417`).

## Evidence
"For `-type l -delete`, the walker normally supplies the parent identity and omits the entry identity. If a regular file replaces the selected link in the same directory, the remover unlinks that regular file. Carry the listed inode and kind where reliable, obtain missing facts for a destructive match, and report a changed entry as a per-entry failure. The read-only walk can keep its current fast path."

## Intake
Admitted, by what each test needs: a test of an entry's kind holds for an entry of that kind, so the kind found is carried — the listing's own word, for nothing — and asked again at the removal. A test of size or age carries the entry's identity already (cr-uv10-2). A name test holds for whatever bears the name, so nothing more is looked up for it; the read-only walk is unchanged.

## What
- `find::Seen::kind`: where a `-type` test held, the kind the walk found the entry to be.
- `Remover::remove_found`: where the walk said, the entry there must still be that kind — a link found, a file there now, and it stays with "it is not what was found: it was replaced since" (a per-entry failure, as any removal's).
- The daemon maps `find`'s kinds onto the removal's.

## Guard proof
- `deletion::…::a_found_entry_is_removed_only_where_and_as_it_was_found` (a case added): a link found by its kind, a file renamed over its name — it stays, the file whole.
- `find::tests::each_entry_found_says_where_it_was_when_asked` (a case added): with `-type f`, the file found says it was a file.
- Mutations, each restored byte for byte: the kind found not asked (red); an entry saying nothing of its kind (red).
- The daemon's tests (220) and the CLI's `find` tests (14) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/find.rs` — `Seen::kind`; guard.
- `crates/blit-core/src/deletion.rs` — `Remover::remove_found`; guard.
- `crates/blit-daemon/src/service/admin.rs` — the kinds handed on.

## Known gaps
- Speed: `-type … -delete` asks each entry removed once more (`fstatat`); not measured. Other removals are unchanged.
- An entry of the same kind and name put in the found one's place, with no size or age tested, is removed: every test it was found by holds for it.
- Windows: removal by path, as before.
