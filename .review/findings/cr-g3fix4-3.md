# cr-g3fix4-3: Typed Unix escapes still change deletion scope on a Windows peer

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:204 — a bare backslash becomes '/', including in received patterns. transfers/filter.rs:228 leaves Unix relative escapes unchanged before serialization. A parameterized compiler probe confirmed that wire pattern ./a\b excludes ab/saved.txt under the Unix rule but allows it under the Windows rule; Windows instead excludes a/b/saved.txt. Both destination paths are valid Windows names. This was tested through the actual rule compiler on macOS, not a Windows runtime.

## Predicted observable failure
A Unix-to-Windows mirror using --exclude './a\b' omits source ab while leaving destination ab inside deletion scope, allowing existing excluded data to be deleted. It also incorrectly protects a/b.

## Reviewer's suggested approach
Translate typed Unix escapes into a portable wire representation before serialization and compile received patterns consistently. Preserve native Windows separator handling for locally supplied patterns, including .\cache.

## Intake
Admitted on the reviewer's parameterized-compiler analysis (the Windows rule as a parameter, run on macOS; no Windows runtime), the Unix half reproduced by the orchestrator with `blit 1.0.0+548625f5c7cf`: `blit copy src/ dst/ --exclude './a\b'` leaves `ab` out and copies `a/b`, so the source side excludes `ab` while, per the analysis, a Windows peer compiling the same wire text (`fs_enum.rs:204`, and globset's own Windows reading of a bare `\`) keeps `ab` inside its deletion scope and shields `a/b` instead. cr-g3fix3-3's Known gaps recorded exactly this and called it a wider dialect change; it is a mirror data-loss path, so it cannot stay a gap. Fix shape (the reviewer's): a typed Unix escape never leaves the typed entry. Under the Unix rule `resolve_excludes_on` rewrites every bare `\c` outside a bracket class into the class dialect — `globset::escape(c)`, i.e. `[c]` for a glob metacharacter and `c` itself otherwise, `\\` → `[\]` — bracket-aware through `bracket_class_len`, a trailing lone `\` left for validation to refuse as today. The rewrite applies to relative patterns and to the typed parts of a resolved absolute pattern (`written`), so `build` (the local filter) and `build_spec` (the wire form) both carry the canonical dialect, which holds no bare `\`. Under the Windows rule `\` is already a separator and nothing changes; a Windows user's `.\cache` still anchors. The receiving peer's `compile` is untouched: its reading of a bare `\` no longer matters because none arrives from a 1.0 peer. `--include` is not rewritten (it takes no part in the deletion scope; record it). Guards: unit — `resolve_excludes_on(["./a\\b", "x\\*y", "[\\]z", "p\\\\q", "./cache"], None, false)` gives `["./ab", "x[*]y", "[\\]z", "p[\\]q", "./cache"]`, and the output compiled under the Windows rule excludes `ab/saved.txt` and leaves `a/b/saved.txt` in scope (parameterized, every OS); `build_spec` carries the rewritten form; the CLI `copy --exclude './a\b'` still leaves `ab` out and copies `a/b` (Unix); cr-g3fix-3's and cr-g3fix3-3's guards stay green; mutation-red.

## What
Off Windows a typed `--exclude` escape no longer leaves the client as a bare `\`. `--exclude './a\b'` (the name `ab` under the Unix rule) is written `./ab` in the local filter and in the wire form `build_spec` sends, so a Windows peer's mirror deletion scope leaves out `ab`, exactly as the client's walk does, and leaves `a/b` in scope. Before, the pattern went out as typed. A Windows peer read the bare `\` as a separator (its globset, and `normalize_separators`), so its deletion scope kept the destination's `ab` (whose source counterpart the client had left out, so `mirror` could delete it) and shielded the unrelated `a/b` instead. On this host nothing changes in what a pattern matches: `copy --exclude './a\b'` still leaves out `ab` and copies `a/b`.

## Approach
- `typed_exclude_dialect(pattern, windows)` (new, `fs_enum.rs`) writes a typed pattern in the dialect every peer reads alike, `/`-separated with no bare `\`:
  - Under the Windows rule it is `normalize_separators`, unchanged.
  - Under the Unix rule each escape `\c` outside a bracket class is written by `escape_as_class(c)`. That gives `[c]` for `?*[]{}` (the characters `globset::escape` brackets), for `,` and for `\`, and `c` itself otherwise. So `./a\b` → `./ab`, `x\*y` → `x[*]y`, `p\\q` → `p[\]q`, `\[ab]` → `[[]ab]`.
  - A class is copied as it is, found with `bracket_class_len` (cr-g3fix3-3), because a class's `\` is a member on every platform: `[\]z` stays `[\]z`.
  - A pattern globset refuses under the Unix rule is returned exactly as typed, so validation still refuses it as it does today. That covers a trailing lone `\` (a dangling escape) and an unclosed class (`a[\*`, which a naive rewrite would turn into the valid `a[[*]`).
- Deviation, recorded: `\,` is written `[,]`, not `,` as `globset::escape(',')` would give. Inside an alternation (`{a\,b,c}`) a bare `,` separates branches, so `,` would have changed the meaning to `{a,b,c}`. A one-member class is a literal comma everywhere, inside an alternation or not.
- `resolve_excludes_on` (`transfers/filter.rs`) writes each pattern through it:
  - relative patterns, on both of its paths (where `normalize_separators` was called before);
  - a resolved absolute pattern's whole written text. Its literal parts are classes already (`literal_glob`) and are copied unchanged, so in effect only the typed parts are rewritten.
  - Rewriting the joined text rather than each typed component also covers an escaped `/` split across components: `/src/a\/b` → `a/b`, globset's own reading of `a\/b`.
- An unmatchable absolute pattern stays exactly as typed in `patterns` and in the warning, as the intake says. It matches nothing under either rule, so its bare `\` changes nothing on any peer.
- `build` (the local filter) and `build_spec` (the wire form) both take `resolve_excludes(...).patterns`, so both carry the class dialect. A receiving peer's `ExcludeRules::compile` is unchanged; no bare `\` reaches it from a 1.0 peer.
- `--include` is not rewritten: it takes no part in the deletion scope (see Known gaps).
- cr-g3fix-3's guard `relative_excludes_take_forward_slashes_under_the_windows_rule` asserted that under the Unix rule relative patterns pass through as typed, which this fix changes by design. Its Unix assertions now expect the class dialect with the same meaning: `.\cache` → `.cache`, `a\.java` → `a.java`, the dangling `cache\` as typed, and `C:\elsewhere\x` (relative under the Unix rule) → `C:elsewherex`. Its Windows-rule assertions are unchanged.
- No other deviation from the admitted fix shape.

## Files changed
- `crates/blit-core/src/fs_enum.rs:136-142` — `excluded_under_rule` (new, `cfg(test)`): `ExcludeRules` under either platform rule, so a test outside the module can read a pattern as a Windows peer does.
- `crates/blit-core/src/fs_enum.rs:222-279` — `typed_exclude_dialect` and `escape_as_class` (new).
- `crates/blit-core/src/fs_enum.rs:993-1074` — guard `a_typed_unix_escape_is_written_in_the_class_dialect` (new).
- `crates/blit-core/src/transfers/filter.rs:12-14` — imports `typed_exclude_dialect`.
- `crates/blit-core/src/transfers/filter.rs:192-197, 212-216` — `ResolvedExcludes::patterns` and `resolve_excludes` docs.
- `crates/blit-core/src/transfers/filter.rs:235, 251, 272-277` — `resolve_excludes_on` writes relative patterns and each resolved pattern through `typed_exclude_dialect`.
- `crates/blit-core/src/transfers/filter.rs:1439-1473` — cr-g3fix-3's `relative_excludes_take_forward_slashes_under_the_windows_rule`: doc, and its Unix-rule assertions redefined (see Approach).
- `crates/blit-core/src/transfers/filter.rs:1489-1540` — guard `a_typed_unix_escape_names_the_same_path_under_either_rule` (new).
- `crates/blit-cli/tests/exclude_contract.rs:476-507` — `copy_exclude_a_typed_unix_escape_names_the_escaped_character` (new, Unix).

## Guard proof
- Guards (ran on macOS aarch64; both rules are parameters, so the Windows rule ran here as string handling and rule compilation, not as a Windows runtime):
  - U1 `fs_enum::tests::a_typed_unix_escape_is_written_in_the_class_dialect` (blit-core, every OS, pure). Thirteen typed patterns are each written as expected: `./a\b` → `./ab`, `x\*y` → `x[*]y`, `[\]z` kept, `p\\q` → `p[\]q`, `\[ab]` → `[[]ab]`, `{a\,b,c}` → `{a[,]b,c}`, `{a\}b,c}` → `{a[}]b,c}`, `q\{x\}\?` → `q[{]x[}][?]`, `\!x\-` → `!x-`, `dir\/f` → `dir/f`, `[!\]]x\*` → `[!\]]x[*]`, `caf\é` → `café`, and `plain/*.tmp` unchanged. Each written form holds no bare `\` outside a class. Compiled with `backslash_escape(true)` (the Unix reading, forced on every OS), the typed and the written form agree on 23 names. The refused forms `a\`, `a[\*`, `x\*[y` come back as typed, and the Windows rule is the separator rewrite.
  - U2 `transfers::filter::tests::a_typed_unix_escape_names_the_same_path_under_either_rule` (blit-core, every OS; the absolute part on Unix). `resolve_excludes_on(["./a\b", "x\*y", "[\]z", "p\\q", "./cache"], None, false)` gives `["./ab", "x[*]y", "[\]z", "p[\]q", "./cache"]`. That output, read by `ExcludeRules` under the Unix and under the Windows rule, excludes `ab/saved.txt` and `x*y` and leaves `a/b/saved.txt` and `x/zy` in scope. `build` and `build_spec` carry the host rule's form (the class dialect here). `--include 'a\*b'` is passed as typed. On Unix, `<root>/a\b` and `<root>/sub/x\*y` resolve to `./ab` and `sub/x[*]y`.
  - U3 `exclude_contract::copy_exclude_a_typed_unix_escape_names_the_escaped_character` (blit-transfer, Unix): `blit copy src/ dst/ --exclude './a\b'` leaves out `ab`, copies `a/b/saved.txt` and `keep.txt`, and warns about nothing. This is a regression guard for this host's reading, which the fix keeps. It passed before the fix too, so it is not mutation-red; U1 and U2 are.
  - Unchanged and green: cr-g3fix3-3's `a_bracket_class_keeps_its_backslash_under_either_rule` and cr-g3fix-3's `backslash_forms_follow_the_platform_rule`. cr-g3fix-3's `relative_excludes_take_forward_slashes_under_the_windows_rule` is green with its Unix assertions redefined (Approach). `backslash_forms_match_on_windows` and `mirror_exclude_windows_backslash_forms` are `cfg(windows)`: type-checked by the Windows cross-clippy, run by CI's Windows leg.
- F1 — `typed_exclude_dialect`: the Unix rule returns every pattern as typed (the guard `if !pattern.contains('\\')` widened to `|| !windows`; the pre-fix behaviour). Red: U1 FAILED at `fs_enum.rs:1019`, `left: "./a\\b"`, `right: "./ab"`; U2 FAILED at `filter.rs:1502`, `left: ["./a\\b", "x\\*y", "[\\]z", "p\\\\q", "./cache"]`; the redefined cr-g3fix-3 guard FAILED at `filter.rs:1455`, `left: [".\\cache", "cache\\", "a\\.java", "*.tmp"]`.
- F1b — F1 with U2's `written` assertion disabled for the run, to show that the rule-reading half bites on its own. Red: U2 FAILED at `filter.rs:1505`, "the name typed as a\b (windows rule: true)". The Unix-rule pass of the loop went green first: the typed `./a\b`, read as a Windows peer reads it, does not exclude `ab`.
- F2 — `escape_as_class`: `,` dropped from the bracketed set. Red: U1 FAILED at `fs_enum.rs:1019`, `left: "{a,b,c}"`, `right: "{a[,]b,c}"`.
- F3 — `typed_exclude_dialect`: the bracket-class arm removed (a class's `\` rewritten as an escape). Red: U1 FAILED at `fs_enum.rs:1019`, `left: "[[]]z"`, `right: "[\\]z"`; U2 FAILED at `filter.rs:1502` (`"[[]]z"`); cr-g3fix4-4's E1 FAILED at `filter.rs:1434`, `left: ["./back[s[]]lash"]`; cr-g3fix2-2's `a_resolved_name_holding_a_backslash_matches_itself` FAILED at `filter.rs:1032`, `left: ["back[[]]slash/*.txt"]` (a literal part's class).
- F4 — `resolve_excludes_on`: a resolved absolute pattern pushed as `written` alone (no dialect). Red: U2 FAILED at `filter.rs:1536`, `left: ["./a\\b", "sub/x\\*y"]`, `right: ["./ab", "sub/x[*]y"]`.
- F5 — `typed_exclude_dialect`: an unclosed class taken as one ordinary `[` instead of returning the pattern as typed. Red: U1 FAILED at `fs_enum.rs:1070`, `left: "a[[*]"`, `right: "a[\\*"` (an invalid pattern made valid).
- Restore after each: the fixed file copied back with `/bin/cp -f`; md5 `f34fea2012dd2e12d5f9d852d1a4a8c7` (`fs_enum.rs`) and `9659899dabf1cfcd97b4c2a443b012ab` (`filter.rs`) matched the saved fixed files every time. Then `cargo test -p blit-core --lib -- transfers::filter fs_enum` passed 34, and `cargo test -p blit-transfer --test exclude_contract` passed 12.
- Line numbers are those of the fixed files. F3 removes seven lines and F5 two in front of U1, so its raw reports were shifted by that many.

## Known gaps
- `--include` is not rewritten. A Unix user's `--include 'a\*b'` still reaches a Windows peer with a bare `\`, which the peer reads as a separator. `--include` takes no part in the deletion scope: a file that fails it is not sent, and a mirror's deletion scope compiles only the excludes, so this cannot delete data. A source-side Windows daemon serving a pull would select different files than the Unix reading names.
- A received pattern still holding a bare `\` (from a pre-1.0 peer, or a filter an embedder builds) is still read as a separator by a Windows peer. 1.0 peers refuse other builds at session open (D-2026-07-05-2), so only an embedder can produce one.
- An unmatchable absolute pattern keeps its bare `\`; it matches nothing under either rule.
- U1's Windows-rule line and U2's Windows-rule pass are the string handling and the parameterized compiler, not a Windows runtime.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the final gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
