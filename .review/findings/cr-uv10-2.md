# cr-uv10-2: `find -delete` removed by a path, not in the directory its tests were held in

**Severity**: HIGH — a directory replaced between the walk's test and the removal gives up a same-named entry of another size: one the test never held for is removed
**Status**: Reviewed, found incomplete — `combined-r22` (Codex openreview, 2026-11-22) reviewed `b92c9a17..5d2f8def` — the eight `combined-r21` fixes — with `guard_confirmed: true`. "For `-type l -delete`, the walker normally supplies the parent identity and omits the entry identity. If a regular file replaces the selected link in the same directory, the remover unlinks that regular file. Carry the listed inode and kind where reliable, obtain missing facts for a destructive match, and report a changed entry as a per-entry failure. The read-only walk can keep its current fast path." The remaining work is cr-uv11-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `55f43417`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-uv9-1 (`5a663780`).

## Evidence
"The walker evaluates `-size` through its held directory. The remover receives a pathname and opens the parent separately. If that directory is replaced before its first matching entry is removed, a same-named file of a different size can be deleted. Carry the observed parent and entry identity into removal."

## Intake
Admitted as the reviewer put it. Seen in the work: asking every entry which file it is costs a call each, and slowed a `find -delete` of 200,000 files from 294 to 477 ms — so the entry's identity is carried where the walk looked at it anyway (a test of its size or age, a directory it entered, the start), and the directory's is asked once per directory.

## What
- `find::Seen` — where an entry found was: the directory the walk held it in, and which file it is where the walk looked at it anyway. `FindOptions::seen` asks for it; `FindEvent::Found` carries it.
- `deletion::Remover::remove_found`: the directory the removal holds must be the directory the entry was found in — asked once when the removal opens it — and, where the walk said which file the entry was, the entry there must be that file; otherwise it stays, and says why. `Remover::remove` is it with nothing seen.
- The daemon's `find -delete` asks for it and hands it on.

## Guard proof
- `deletion::…::a_found_entry_is_removed_only_where_and_as_it_was_found`: the directory replaced by another holding another name for the same file — the entry stays; the entry replaced in its directory by another file — it stays, with the other file's bytes; as it was — it goes.
- `find::tests::each_entry_found_says_where_it_was_when_asked`: asked, each entry found says the directory it was held in, and which file it is where its size was asked, or it is a directory or the start; not asked, nothing.
- Mutations, each restored byte for byte, four: the directory found in not asked; the entry found not asked; a directory read saying nothing of itself; the start saying nothing of where it was — each red.
- The daemon's tests (220) and the CLI's `find` tests (14) pass. Windows cross-clippy clean.
- Speed, 200,000 empty files through a loopback daemon: `find -delete` median 293.4 → 295.6 ms; `-name 'f00*' -delete` 75.2 → 76.8.

## Files changed
- `crates/blit-core/src/find.rs` — `Seen`, `FindOptions::seen`, the walk; guard.
- `crates/blit-core/src/held.rs` — `Start::seen`.
- `crates/blit-core/src/deletion.rs` — `Remover::remove_found`, `Passage::version`; guard.
- `crates/blit-daemon/src/service/admin.rs`, `crates/blit-daemon/src/service/core.rs` — `-delete` asks for and hands on what was seen.

## Known gaps
- An entry whose size or age was not tested — `-name x -delete` — is removed by its name in the directory found, whichever file is there: its tests (name, kind) hold for any entry there by that name. Its directory is still the one found.
- Between the removal's look and its unlink, a look and then an act, as every check by name.
- Windows: removal by path, as before.
