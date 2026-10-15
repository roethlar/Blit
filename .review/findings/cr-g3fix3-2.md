# cr-g3fix3-2: Escaped literals also exclude entries named like their encoded pattern

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-15), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix3-r1-36b752b6`; codereview re-review g3fix3-r1 over `e9dc99c9..36b752b6` (record `.review/results/g3fix3-r1.codex.json` + `.events.jsonl`); 2026-10-15 20:31–20:40Z; capability_ok=true; guard_confirmed=true (reviewer replaced `globset::escape(name)` with `name.to_string()` in `literal_glob` → `a_name_read_from_the_disk_is_matched_literally` red at `filter.rs:871`, restored by `git checkout`, blob hash verified, green; 1517/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` for blake3 — the reviewer's environment, not the change); SHAs pin-verified; closed: cr-g3fix2-2 true, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:446 — escaped literal components reach ExcludeRules, whose fs_enum.rs:131 fallback also compares the encoded pattern as ordinary text. Reproduced with alias pointing to src/data[1], alongside src/data[[]1[]] and src/data1. Copying with --exclude <alias> omitted both data[1]/private.txt and the unrelated data[[]1[]]/private.txt, while copying data1/private.txt. The generated ./data[[]1[]] matches the intended name through globset and the unrelated encoded name through glob_match.

## Predicted observable failure
Copy or backup silently omits an unrelated subtree whose name equals an escaped pattern's textual representation, despite successful completion.

## Reviewer's suggested approach
Evaluate escaped patterns consistently as globs; do not additionally interpret their encoded text through the exact/prefix fallback. Add a negative control containing the escaped representation as an actual filename.

## Intake
Admitted; reproduced by the orchestrator with the release binary on macOS: `alias2 -> s2/data[1]`, siblings `s2/data[[]1[]]` and `s2/data1`, `blit copy s2/ d2/ --exclude <alias2>` copied only `data1/private.txt` — `data[[]1[]]/private.txt`, never named, was left out. cr-g3fix2-2's Known gaps called this pre-existing behaviour of the fallback; it is not: before the escaping no rule text ever equalled an encoded name, so the fallback could not misfire this way. Root cause: `ExcludeRules::excludes` (`fs_enum.rs:118-135`) evaluates every rule twice — through the compiled globset and through `glob_match` (`fs_enum.rs:532`, a legacy matcher: exact text compare, or `*`-prefix/suffix/contains) over the raw pattern text — so the escaped text `data[[]1[]]` equals a real name of that spelling. Fix shape: a pattern the globset compiled is evaluated as a glob only; the text fallback runs, if at all, only for a pattern the globset refused (establish what `glob_match` still supplies — read `FileFilter::build_globset` and the callers; if nothing, drop the fallback from `ExcludeRules` and say so). Guards: the alias-into-`data[1]` guard gains the sibling `data[[]1[]]` as a negative control (kept) in the unit test and the CLI test; a direct `ExcludeRules` test that an escaped literal rule matches its real name and not its encoded spelling; the existing rules (name at any depth, anchored, subtree, trailing `/`, `*.log`, `**/*.log`, `dir/**`) stay green; all mutation-red.

## What
(filled by the fix)

## Approach
(filled by the fix)

## Files changed
(filled by the fix)

## Guard proof
(filled by the fix)

## Known gaps
(filled by the fix)

## Reviewer comments
(filled by the re-review)
