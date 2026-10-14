# cr-g3fix-1: Absolute excludes lose filesystem case semantics after rewriting

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/transfers/filter.rs:235 — the first lexical match wins before the canonical spelling is considered. relative_to_root preserves the suffix's typed case at line 315, while fs_enum.rs:213 builds case-sensitive globs. Reproduced on case-insensitive macOS: src/CACHE and src/cache identify the same directory, but excluding the absolute src/CACHE still copied cache/private.txt without warning. Windows has the same suffix-matching problem despite its case-insensitive root comparison.

## Predicted observable failure
A differently cased absolute exclude silently transfers the intended excluded subtree and leaves it within a mirror's deletion scope.

## Reviewer's suggested approach
Preserve filesystem case semantics in the rewritten matcher, including the wire representation, or resolve literal suffixes to their filesystem spelling. Test case differences below the source root.

## Intake
Admitted. On a case-insensitive filesystem (macOS by default, Windows) `--exclude /src/CACHE` for an on-disk `src/cache` is rewritten to the anchored literal `./CACHE`, and the anchored rule compares case-sensitively, so nothing matches and no warning fires: the subtree the user named copies, and under mirror its counterpart is deletable. The reviewer reproduced it on macOS. Fix shape: when the absolute pattern's literal part names an existing path, take the filesystem's own spelling of each component below the root (canonicalize the literal path, then relativize), so the rewritten rule — and the wire form a push sends — name the entry as the disk does; a path that does not exist keeps its typed case. A guard with a differently cased absolute exclude on a case-insensitive filesystem (skipped where the filesystem is case-sensitive).

## What
An absolute `--exclude` typed in another case than the disk stores — `--exclude /src/CACHE` for an on-disk `src/cache` on macOS or Windows — now leaves that entry out. Once the pattern is made source-relative, each component the filesystem holds is spelled the way the filesystem stores it, so the rewritten rule (`./cache`) and the wire form a push sends name the entry exactly as the walk sees it. A path that does not exist keeps its typed case.

## Approach
- `relative_to_root` now returns the components below the root instead of the finished string; `written` (new) joins them, a single name still written `./name`.
- `stored_spelling` (new): walks those components from the source root and replaces each with the name `stored` reports for it, stopping at the first component that holds a glob metacharacter or that `stored` cannot name; that one and the rest stay as typed.
- `stored_name` (new, the production `stored`): the last component of `std::fs::canonicalize(path)` — the stored case (and, on Windows, the long name) — unless the path is missing or is a symlink.
- `resolve_excludes` passes every relativized pattern through `stored_spelling(root, rest, windows, stored_name)` before `written`, so `build` (the local rule) and `build_spec` (the wire form) both carry the stored spelling. Unmatchable patterns are untouched.
- Decision: the spelling is taken one component at a time from the source root, not by canonicalizing the whole literal path and relativizing it. Canonicalizing the whole path also resolves a symlink inside the source to its target, which names a different entry from the link the walk sees (the walk does not follow links); per-component lookup stops at a symlink instead. It also leaves the spelling of the source root itself out of it — the root is matched as before (typed, then resolved).
- The `stored` lookup is a parameter so the walk is unit-tested on every OS against a stand-in case-insensitive disk.

## Files changed
- `crates/blit-core/src/transfers/filter.rs:240-249` — `resolve_excludes`: a relativized pattern is spelled by `stored_spelling` and written by `written`.
- `crates/blit-core/src/transfers/filter.rs:349-373` — `relative_to_root` returns the components below the root.
- `crates/blit-core/src/transfers/filter.rs:375-442` — `written`, `stored_spelling`, `stored_name` (new).
- `crates/blit-core/src/transfers/filter.rs:679-765` — test helper `case_insensitive`; guards `an_absolute_exclude_in_another_case_takes_the_stored_spelling`, `the_stored_spelling_runs_until_a_component_cannot_be_named`, `the_stored_name_is_the_entrys_own_never_a_symlink_targets`.
- `crates/blit-core/src/transfers/filter.rs:771-773` — the Windows-forms test formats `relative_to_root`'s components with `written` (assertions unchanged).
- `crates/blit-cli/tests/exclude_contract.rs:184-230` — `case_insensitive` and `mirror_exclude_absolute_path_in_another_case_leaves_the_entry_out` (guard, CLI end to end).

## Guard proof
- Guards:
  - `transfers::filter::tests::an_absolute_exclude_in_another_case_takes_the_stored_spelling` (blit-core; skipped with a message where the filesystem is case-sensitive, detected by writing `case-probe` and opening `CASE-PROBE`; ran here on macOS APFS): `<root>/CACHE`, `<root>/CACHE/SUB/*.tmp`, `<root>/Missing/X` over an on-disk `cache/Sub` must give `./cache`, `cache/Sub/*.tmp`, `Missing/X` from both `build` and `build_spec`, none unmatchable.
  - `transfers::filter::tests::the_stored_spelling_runs_until_a_component_cannot_be_named` (blit-core, every OS): the walk against a stand-in disk.
  - `transfers::filter::tests::the_stored_name_is_the_entrys_own_never_a_symlink_targets` (blit-core, Unix): a directory gives its name; a symlink and a missing path give none.
  - `exclude_contract::mirror_exclude_absolute_path_in_another_case_leaves_the_entry_out` (blit-transfer; same skip): `blit mirror src/ dst/ --exclude <src>/CACHE` leaves `cache/` out, copies `keep.txt`, no warning.
- M1: in `resolve_excludes`, `stored_spelling(root, rest, windows, stored_name)` → `stored_spelling(root, rest, windows, |_| None)` (no lookup: the typed spelling, as before the fix). Red: case guard FAILED, "assertion `left == right` failed: build", `left: ["./CACHE", "CACHE/SUB/*.tmp", "Missing/X"]`, `right: ["./cache", "cache/Sub/*.tmp", "Missing/X"]`; CLI guard FAILED at `exclude_contract.rs:221`, "the entry named in another case must be left out".
- M2: in `stored_spelling`, `spelled.push(name)` → `spelled.push(component)`. Red: the spelling guard FAILED, `left: ["CACHE", "SUB", "X.TXT"]`, `right: ["cache", "Sub", "x.txt"]`.
- M3: in `stored_name`, the symlink check made always false. Red: the stored-name guard FAILED, `left: Some("real")`, `right: None` — the link took its target's name.
- Restore after each: the fixed `filter.rs` copied back, md5 `d872ed4a12bd403656bd107aa47d111d` matched; `cargo test -p blit-core --lib transfers::filter` 15 passed and `cargo test -p blit-transfer --test exclude_contract` 8 passed, all four guards `ok`.

## Known gaps
- A component that is a symlink keeps its typed spelling, and so does everything after it: its stored name cannot be read through `canonicalize` without resolving it. On a case-insensitive filesystem a differently cased absolute exclude that names a symlink inside the source therefore still misses it — one link entry, never the data behind it, since the walk does not follow links.
- The case guards run only where the filesystem is case-insensitive: macOS here (ran, not skipped), Windows on CI's Windows leg (not run here); on Linux they print a skip line and pass. The stored-spelling unit test runs everywhere. Windows behaviour of `stored_name` (`canonicalize`'s `\\?\` form and long names) is by reading only.
- Verified on macOS (aarch64) only, plus Linux and Windows cross-clippy; Linux and Windows runtime tests were not run.

## Reviewer comments
(filled by the re-review)
