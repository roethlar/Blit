# cr-ecm1-3: An include also named a file spelled like the pattern, in the typed case only

**Severity**: MEDIUM — a mirror's deletion scope is what the includes name, so a destination-only file merely spelled like an include pattern was in scope and deleted; under `--ignore-case` the two cases of one such name were treated differently
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm1-3: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, review `exclude-slice1-r1` over `3bc3ee47..66d1338a`. That run FAILED before a verdict (provider 503); this finding is read from its event stream (`.review/results/exclude-slice1-r1.codex.events.jsonl`, event 136, second probe).

## Evidence
The reviewer's probe against the built head: source `a.txt`; destination-only `[a].txt`, `[A].TXT`, `B.txt`; `mirror --yes --ignore-case --include '[a].txt'`. Afterwards the destination held `B.txt`, `[A].TXT` and `a.txt`: the file named `[a].txt` was deleted and `[A].TXT` beside it was kept.

Cause, `crates/blit-core/src/fs_enum.rs` `FileFilter::allows_entry`: an include matched as a glob OR by `glob_match`, a comparison of the pattern's TEXT with the name. As a glob `[a].txt` is the class `a` and names `a.txt`; as text it equals the name `[a].txt`. So that file was "included", hence in the mirror's deletion scope, hence deleted as absent from the source — and only in the pattern's typed case, because the text comparison knows nothing of `--ignore-case`.

## Predicted observable failure
A file whose name happens to be spelled like an include pattern holding glob syntax is treated as included: copied from a source, and deleted from a mirror's destination when the source lacks it. With `--ignore-case` the same name in another case is not.

## Intake
Admitted. The text comparison is older than slice 1 — an exclude had the same one until cr-g3fix3-2 removed it — but slice 1 made the two readings disagree under the new option, and it is a deletion.

## What
An include is matched as a glob and nothing else: the include set is a `CompiledPatterns`, as the exclude set has been since cr-g3fix3-2 — widened under `--ignore-case`, and comparing as text only a pattern the glob compiler refuses (unreachable past validation). The literal name is written with its brackets escaped (`[[]a[]].txt`) and follows the mode like any other pattern.

## Guard proof
- `fs_enum::tests::an_include_is_a_glob_and_never_its_own_spelling`: in both modes the class names `a.txt` (and `A.TXT` only when case is ignored) and never `[a].txt`, `[A].TXT` or `sub/[a].txt`; the escaped literal names `[a].txt` (and `[A].TXT` only when case is ignored) and never `a.txt`; five ordinary include forms still match.
- `ignore_case::an_include_class_does_not_name_a_file_spelled_like_the_pattern`: the reviewer's scenario through the binary, in both modes — `a.txt`, `[a].txt`, `[A].TXT` and `B.txt` are all at the destination afterwards.
- Mutation (the text comparison restored): both guards red; restored byte for byte, both green. The workspace suite passed before the guards were added, so nothing relied on the old reading.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `compiled_includes` is a `CompiledPatterns`; `allows_entry` matches an include as a glob only; the unit guard.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- Run on Linux only.
- `exclude_dirs` (set by no command-line option) still compares its patterns as text against each path component.
