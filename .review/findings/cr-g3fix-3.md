# cr-g3fix-3: Windows relative anchors and trailing backslashes are not normalized

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `eddfab6c`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/fs_enum.rs:82 — compilation trims only '/', recognizes only './', and classifies separators using only '/'. resolve_excludes also passes relative patterns through unchanged. Consequently, Windows patterns such as '.\cache' and 'cache\' never acquire the corresponding anchored or subtree rules; the legacy whole-path checks do not match ordinary cache directory entries either. This finding is based on Windows-path code analysis, not a Windows runtime test.

## Predicted observable failure
On Windows, these ordinary directory-path exclusions silently copy cache and its contents instead of pruning them.

## Reviewer's suggested approach
Normalize Windows separators before anchor detection, trailing-separator removal, classification, and wire-spec construction. Preserve literal backslashes on Unix and add Windows coverage for both forms.

## Intake
Admitted. Rule compilation trims only `/`, recognizes only `./` as the anchor and classifies "has a separator" by `/` alone, and `resolve_excludes` passes relative patterns through unchanged, so on Windows `.\cache`, `cache\` and `a\.java` get neither the anchored nor the subtree rule, nor the trailing-separator trim, and the wire spec carries the backslash form; the legacy whole-path checks match nothing useful either. Analysis-only (no Windows runtime in the review), consistent with the agent's own "not done" note. Fix shape: on Windows, normalize `\` to `/` in relative patterns before anchor detection, trailing-separator removal, classification and `build_spec`; a backslash on Unix stays a literal character. Put the platform rule behind a parameter so the normalizer is unit-tested on every OS, and gate the end-to-end Windows forms on `cfg(windows)` for CI's Windows leg.

## What
On Windows a relative `--exclude` written with backslashes now means what the same pattern means with forward slashes: `.\cache` names the top-level `cache` only, `cache\` is the name `cache` at any depth, and `a\.java` (or `a\.java\`) is the `a/.java` subtree. The patterns reach the filter, and the wire form a push or pull sends, with `\` written `/`. On Unix nothing changes: a backslash is an ordinary character there (globset reads it as an escape), not a separator.

## Approach
- `normalize_separators(pattern, windows)` (new, `fs_enum`): `\` → `/` when `windows`, the pattern as typed otherwise. The platform rule is a parameter, so both rules are unit-tested on every OS.
- `ExcludeRules::compile` takes the rule (`compile(patterns, windows)`) and normalizes each pattern before the trailing-separator trim, `./` anchor detection and name/path classification; `FileFilter::subtree_rules` passes `cfg!(windows)`. This also covers a filter built directly — by an embedder, or from a spec a Windows daemon received.
- `resolve_excludes` is now `resolve_excludes_on(exclude, source_root, cfg!(windows))`; the inner function writes every relative pattern through `normalize_separators` on both of its paths (no absolute pattern in the list, and the per-pattern loop). `build` and `build_spec` both use it, so the wire form carries `/`. Absolute patterns were already split on either slash on Windows and are written with `/` once made relative; one that cannot match stays as typed.
- `ResolvedExcludes::patterns` and `ExcludeRules` doc comments say so.

## Files changed
- `crates/blit-core/src/fs_enum.rs:61-65` — `ExcludeRules` doc: `\` is read as `/` on Windows.
- `crates/blit-core/src/fs_enum.rs:79-83` — `ExcludeRules::compile` takes the rule and normalizes each pattern first.
- `crates/blit-core/src/fs_enum.rs:141-151` — `normalize_separators` (new).
- `crates/blit-core/src/fs_enum.rs:289` — `subtree_rules` compiles under `cfg!(windows)`.
- `crates/blit-core/src/fs_enum.rs:720-765` — `backslash_forms_follow_the_platform_rule` (every OS) and `backslash_forms_match_on_windows` (`cfg(windows)`).
- `crates/blit-core/src/transfers/filter.rs:12` — imports `normalize_separators`.
- `crates/blit-core/src/transfers/filter.rs:186-191, 198-243` — `ResolvedExcludes::patterns` doc; `resolve_excludes` delegates to `resolve_excludes_on`, which normalizes relative patterns on both paths.
- `crates/blit-core/src/transfers/filter.rs:816-843` — `relative_excludes_take_forward_slashes_under_the_windows_rule` (every OS).
- `crates/blit-cli/tests/exclude_contract.rs:267-302` — `mirror_exclude_windows_backslash_forms` (`cfg(windows)`, CLI end to end: `.\.java`, `a\.java\`, `.java\`).

## Guard proof
- Guards that run on every OS (run here, macOS aarch64):
  - `fs_enum::tests::backslash_forms_follow_the_platform_rule`: the normalizer under both rules (`.\cache` → `./cache`, `cache\` → `cache/`, `a\.java` → `a/.java`; Unix rule keeps `a\.java`), then `ExcludeRules::compile(.., true)`: `.\.java` excludes `.java/fonts/x` but not `a/.java/y`; `.java\` excludes `a/.java/y`; `a\.java\` excludes `a/.java/y` but not `.java/fonts/x`; under the Unix rule `a\.java` does not exclude `a/.java/y`.
  - `transfers::filter::tests::relative_excludes_take_forward_slashes_under_the_windows_rule`: `resolve_excludes_on` under the Windows rule gives `./cache`, `cache/`, `a/.java`, `*.tmp`, also with an unmatchable `C:\elsewhere\x` in the list (kept as typed, reported unmatchable); under the Unix rule the list is untouched and nothing is absolute.
- M1: `normalize_separators` made a no-op (`&& false` on its condition). Red: `backslash_forms_follow_the_platform_rule` FAILED at `fs_enum.rs:726`, `left: ".\\cache"`, `right: "./cache"`; `relative_excludes_take_forward_slashes_under_the_windows_rule` FAILED at `filter.rs:826`, `left: [".\\cache", "cache\\", "a\\.java", "*.tmp"]`, `right: ["./cache", "cache/", "a/.java", "*.tmp"]`.
- M2: the `normalize_separators` call removed from `ExcludeRules::compile`. Red: `backslash_forms_follow_the_platform_rule` FAILED at `fs_enum.rs:734`, "assertion failed: anchored.excludes(Path::new(\".java/fonts/x\"))".
- M3: both `resolve_excludes_on` paths pass relative patterns through unchanged. Red: the filter guard FAILED at `filter.rs:826` (the same left/right as M1).
- M4: only the per-pattern loop path unchanged. Red: the filter guard FAILED at `filter.rs:835`, `left: [".\\cache", "cache\\", "a\\.java", "*.tmp", "C:\\elsewhere\\x"]`, `right: ["./cache", "cache/", "a/.java", "*.tmp", "C:\\elsewhere\\x"]`.
- Restore after each: both files copied back, md5 `e697b5faee2bba9f8e47157e550bcf3f` (`fs_enum.rs`) and `c56a13082d550ee1439832f091885522` (`filter.rs`) matched; `cargo test -p blit-core --lib fs_enum` 7 passed and `cargo test -p blit-core --lib transfers::filter` 16 passed, both guards `ok`.
- `cfg(windows)` guards — `fs_enum::tests::backslash_forms_match_on_windows` and `exclude_contract::mirror_exclude_windows_backslash_forms` — were type-checked by the Windows cross-clippy here but not run; CI's Windows leg runs them.

## Known gaps
- Not run on Windows here. In particular, the only guard that can bite `subtree_rules` passing `cfg!(windows)` (rather than the parameter's tested behaviour) is `backslash_forms_match_on_windows`, which can go red only on Windows; the CLI test on Windows goes through `resolve_excludes`, which already normalizes. Both are unproven red until CI's Windows leg runs them against a mutation.
- A Unix client's pattern holding `\` is sent as typed; a Windows peer applying it (a daemon's mirror deletion scope) reads that `\` as a separator in `compile`. No Windows name can hold `\`, so the Unix reading could match nothing there either.
- `--include` is not normalized (outside this finding): it has no anchor or subtree rules, and globset already reads `\` in a glob as `/` on Windows.
- Verified on macOS (aarch64) only, plus Linux and Windows cross-clippy; Linux and Windows runtime tests were not run.

## Reviewer comments
(filled by the re-review)
