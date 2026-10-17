# cr-g3fix4-4: Windows absolute patterns still rewrite backslashes inside bracket classes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `e7be4970`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:352 — absolute_components still replaces every backslash, bypassing the new bracket-aware normalizer. A probe of resolve_excludes_on with the Windows rule converted <root>/back[s\]lash into back[s/]lash without warning. The original glob matches backslash but not back/lash; the resulting filter excluded both backslash/saved.txt and back/lash/saved.txt. On Windows, C:\src\back[s\]lash follows this same absolute-pattern branch.

## Predicted observable failure
A valid Windows absolute exclude silently omits an unrelated subtree from copy or backup and incorrectly removes it from mirror deletion scope. build_spec also sends the corrupted pattern to peers.

## Reviewer's suggested approach
Make absolute-pattern tokenization preserve bracket-class contents as well as relative-pattern normalization. Test absolute Windows patterns containing classes with both a backslash and an ordinary member.

## Intake
Admitted on analysis (the Windows rule as a parameter; `absolute_components` at `filter.rs:350-375` reads `path.replace('\\', "/")` unconditionally under the Windows rule), the same footing as cr-g3fix-3 and cr-g3fix3-3: cr-g3fix3-3 made `normalize_separators` bracket-aware for relative patterns and `ExcludeRules::compile`, and missed the third place a typed Windows pattern is split. Fix shape (the reviewer's): `absolute_components` normalizes with the bracket-aware `normalize_separators` instead of `replace`, and the verbatim/UNC prefix handling reads the normalized text as before (a bracket class cannot be part of a prefix). Guards: `resolve_excludes_on` under the Windows rule turns `<root>/back[s\]lash` and `C:\src\back[s\]lash` into `back[s\]lash`, the compiled filter excludes `backslash/saved.txt` and leaves `back/lash/saved.txt` in scope, `build_spec` carries the same; the earlier UNC and verbatim-prefix cases stay green; mutation-red.

## What
Under the Windows rule an absolute `--exclude` keeps a bracket class's `\` as a class member, as globset reads it there. `C:\src\back[s\]lash` (or `<root>/back[s\]lash`) becomes `./back[s\]lash`: it leaves out `backslash/saved.txt` and leaves the unrelated `back/lash/saved.txt` in scope, in the copy and in a mirror's deletion scope, and the wire form `build_spec` sends is the same. Before, `absolute_components` rewrote every `\` of an absolute pattern as `/`, so the pattern became `back[s/]lash`, whose class matches a real separator, and `back/lash/` was silently excluded too.

## Approach
- `absolute_components` (`transfers/filter.rs`) splits its input with the bracket-aware `normalize_separators` (cr-g3fix3-3) instead of `path.replace('\\', "/")`. Under the Windows rule a bare `\` is still a separator and a class is copied verbatim. Under the Unix rule the input is returned as typed, as before.
- That one function splits all three things an absolute pattern is compared through: the typed pattern, the source-root forms, and the form through the resolved root. Each of them now reads a class the same way, so a pattern typed with `\` separators still relativizes against a root written either way.
- The verbatim (`\\?\`) and UNC (`\\?\UNC\`, `\\server`) prefix handling reads the normalized text exactly as before: a prefix holds no `[`, so its `\` are all bare and all become `/`.
- `through_resolved_root` needed no change. On Windows `Path` splits `back[s\]lash` at the `\`, but its ancestor walk stops at the first component holding a glob metacharacter (`back[s`), so the resolved prefix never reaches a class, and the typed rest is re-appended unchanged.
- No deviation from the admitted fix shape.

## Files changed
- `crates/blit-core/src/transfers/filter.rs:346-353` — `absolute_components` doc and body: bracket-aware `normalize_separators` in place of the unconditional replace.
- `crates/blit-core/src/transfers/filter.rs:1378-1424` — guard `a_windows_absolute_exclude_keeps_a_bracket_class_backslash` (new).

## Guard proof
- Guard (ran on macOS aarch64; the Windows rule is a parameter, so it ran here as string handling and a filter over the output, not as a Windows runtime):
  - E1 `transfers::filter::tests::a_windows_absolute_exclude_keeps_a_bracket_class_backslash` (blit-core, every OS):
    - `relative_to_root` under the Windows rule gives `./back[s\]lash` for `C:\src\back[s\]lash` under `C:\src`, `a/back[s\]lash/x` for `C:\src\a\back[s\]lash\x` under the verbatim `\\?\C:\src`, and `./[\]x` for `\\srv\share\d\[\]x` under `\\?\UNC\srv\share\d`.
    - `resolve_excludes_on([<root>/back[s\]lash], Some(root), true)` gives `["./back[s\]lash"]` with nothing unmatchable. A filter over that output leaves out `backslash/saved.txt` and allows `back/lash/saved.txt`.
    - `build` and `build_spec` give `["./back[s\]lash"]` under this host's rule. On macOS that is the Unix rule, which the bug never touched, so this part can go red only on CI's Windows leg.
  - Unchanged and green: `windows_absolute_excludes_compare_case_insensitively_with_either_slash` (the verbatim and UNC prefixes, either slash, case), `relative_excludes_take_forward_slashes_under_the_windows_rule`, `a_bracket_class_keeps_its_backslash_under_either_rule`.
- D1 — `absolute_components`: the pre-fix `if windows { path.replace('\\', "/") } else { path.to_string() }` restored. Red: E1 FAILED at `filter.rs:1392`, `left: Some("back[s/]lash")`, `right: Some("./back[s\\]lash")` (the raw report says `filter.rs:1396`: the mutation adds four lines in front of the test). `cargo test -p blit-core --lib -- transfers::filter fs_enum`: 31 passed, 1 failed.
- D1b — the same mutation with E1's three `relative_to_root` assertions and its `resolved.patterns` assertion disabled for the run, to show that the filter half bites on its own. Red: E1 FAILED at the `allows("back/lash/saved.txt")` assertion (`filter.rs:1415` in the fixed file), "a separator is no member of the class". The resolved pattern `back[s/]lash` left the unrelated `back/lash/saved.txt` out.
- Restore after each: the fixed `filter.rs` copied back with `/bin/cp -f`; its md5 `24868585fc5f5a69d771fec7c573fa68` matched the saved fixed file. Then `cargo test -p blit-core --lib -- transfers::filter fs_enum` passed 32, E1 `ok`.

## Known gaps
- A source root whose own path holds a `[` in one directory name and a `]` in a later one with a `\` between them (`C:\a[1\b]\src`) is now split there as a class, and a pattern naming that root with `/` separators would not relativize against it (it would be reported as unable to match). Both the root and a pattern typed with `\` are split the same way, so a pattern typed like the root still relativizes. Such a root is unusual; not handled.
- E1's `build`/`build_spec` assertions run under the host's rule, so on macOS they cannot go red for this Windows-only bug; they can on CI's Windows leg. E1's parameterized part is what bit here.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the final gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
