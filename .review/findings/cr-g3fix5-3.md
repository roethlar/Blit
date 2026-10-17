# cr-g3fix5-3: Rewriting an escaped separator changes recursive-glob semantics

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:300 and :326 — escaped '/' is emitted as a bare '/', turning '**\/cache' into '**/cache'. Globset parses the latter as a recursive prefix that also matches top-level cache; the original requires a preceding separator. A direct FileFilter comparison confirmed that cache/needed.txt changes from allowed to excluded. The built CLI's copy --exclude '**\/cache' omitted that file and exited 0.

## Predicted observable failure
Previously valid Unix exclusions acquire broader matching behavior, silently omitting additional files from copies or backups and incorrectly protecting additional paths from mirror deletion.

## Reviewer's suggested approach
Translate escapes while preserving parsed glob semantics and rule anchoring, including the context-sensitive interpretation of '**'. Add equivalence tests for escaped separators adjacent to recursive wildcards.

## Intake
Reproduced 2026-10-17 with `blit 1.0.0+cf3405f65366` (the release build of the pushed candidate; script `scratchpad/g3fix5/repro.sh`, output `repro.out`): `copy --exclude '**\/cache'` over `src/cache/needed.txt` and `src/a/cache/x` left out `cache/needed.txt`, which the typed pattern (a preceding separator required) allows; `a/cache/x` left out as expected. Admitted as MEDIUM: a valid Unix exclusion silently broadened by cr-g3fix4-3's rewrite.

## What
A typed Unix escape of `/` keeps its meaning after cr-g3fix4-3's rewrite: `**\/cache` still requires a preceding separator (it leaves out `a/cache`, not a top-level `cache`), locally and on the wire.

Fix shape, admitted: `typed_exclude_dialect` writes an escaped `/` in a form with the same meaning as globset reads `\/`: a literal separator that does not take part in `**`'s recursive-prefix reading. Candidates to prove by globset equivalence tests (typed vs written, over names with and without a leading component): `**\/x` → `*/**/x` (at least one component before `x`; `*` matches across `/` in this filter, so prove it), or leaving `\/` unrewritten where no equal spelling exists under the Unix rule and recording the Windows-peer reading as a gap. The tests cover `**\/x`, `a\/b`, `\/x`, and an escaped `/` inside and outside alternations.

## Approach
- **Where globset tells `\/` from `/`.** Read from globset 0.4.20's parser (`parse_star`, `parse_backslash`, `bump`): under the Unix rule `\/` is a `Literal('/')` token, the same token a bare `/` makes. They differ in one place only. `parse_star` reads `**` as recursive (`**/`, `/**/`) only when it *peeks* a bare separator after it; when it peeks `\`, `**` is two `ZeroOrMore` (any text, `/` included). The separator *before* `**` is read from the previous character itself, so `a\/**` is `a/**` either way. Probed with the lockfile's globset (`scratchpad/g3fix5/globprobe`): `**\/cache` and `*/cache` both compile to "any text, then `/cache`" and match `a/cache`, `a/b/cache` and not `cache`, while `**/cache` also matches `cache`.
- **Where the rule reading tells them apart.** `exclude_rule` strips a leading bare `./` (the anchor) and trims trailing bare `/`s from the text. So `.\/x` (never anchored) would become anchored as `./x`, and a trailing `x\/` reads as the dangling `x\` there.
- **The rewrite** (`escaped_separator`, new, `fs_enum.rs`; `typed_exclude_dialect`'s Unix branch calls it for an escaped `/` in place of `escape_as_class('/')`, which wrote a bare `/`):
  - After a run of `*`: the run is written as one `*`, then a bare `/`. Before a `\` the whole run is `ZeroOrMore` tokens, and one `*` is the same any-text match, so `**\/x` → `*/x` and `a/**\/x` → `a/*/x`. Deviation, recorded: the shape's candidate was `*/**/x`. Both are the same language here (`*` crosses `/` in this filter); `*/x` is chosen because collapsing the run is one local rule that needs no reading of what follows.
  - After a leading `.` (or `./.`, `././.` …): that `.` is written `[.]` (a one-member class, the same character), so `.\/x` → `[.]/x` and `./.\/x` → `./[.]/x`. The pattern keeps the anchoring it had: none.
  - At the end, with only `/` after it: no spelling without `\` reads the same under the rule reading (it trims the bare `/`s and leaves a dangling escape), so the pattern is returned as typed. This is the shape's "leave `\/` unrewritten where no equal spelling exists"; the Windows-peer reading is a recorded gap.
  - Everywhere else a bare `/`, as before (`a\/b` → `a/b`, `{a\/b,c}` → `{a/b,c}`, `\/x` → `/x`). A class is still copied whole (`[\/]x` unchanged).
- The written form still holds no bare `\` (but for the trailing case), so a Windows peer reads it as this side does. The path/name split is unchanged: the typed and the written form both hold a `/`.
- No new option; the wire form changes only for these patterns.

## Files changed
- `crates/blit-core/src/fs_enum.rs:299-307` — `typed_exclude_dialect`: an escaped `/` goes through `escaped_separator`; a `false` return leaves the pattern as typed.
- `crates/blit-core/src/fs_enum.rs:318-356` — `escaped_separator` (new).
- `crates/blit-core/src/fs_enum.rs:1173-1284` — guard `an_escaped_separator_keeps_its_meaning_in_the_class_dialect` (new).
- `crates/blit-cli/tests/exclude_contract.rs:662-692` — guard `copy_exclude_an_escaped_separator_after_a_recursive_wildcard_keeps_its_meaning` (new, Unix).

## Guard proof
- Guards (ran on macOS aarch64):
  - S1 `fs_enum::tests::an_escaped_separator_keeps_its_meaning_in_the_class_dialect` (blit-core, every OS, pure; the Unix reading is forced with `backslash_escape(true)`, so it can go red on every OS). Nineteen typed patterns are each written as expected: `**\/cache` → `*/cache`, `a/**\/cache` → `a/*/cache`, `***\/cache` → `*/cache`, `\**\/x` → `[*]*/x`, `{**\/cache,z}` → `{*/cache,z}`, `{z,**\/cache}` → `{z,*/cache}`, `**\/**` → `*/**`, `a\/**` → `a/**`, `a\/b` → `a/b`, `{a\/b,c}` → `{a/b,c}`, `\/x` → `/x`, `.\/x` → `[.]/x`, `./.\/x` → `./[.]/x`, `\.\/x` → `[.]/x`, `{.\/x,c}` → `{./x,c}`, `[\/]x` kept, `a\/b\/c` → `a/b/c`, and `x\/` and `x\//` kept as typed. For each pair, over 23 names with and without a leading component (`cache`, `cache/needed.txt`, `a/cache`, `a/cache/x`, `a/b/cache`, `x`, `a/x`, `a/b/x`, `./x`, `/x`, …):
    - globset's Unix reading of the typed and the written form agrees;
    - the whole filter's reading (the any-depth and subtree rules plus the whole-pattern test against the name and the path) of the typed form under the Unix rule agrees with the written form's under the Unix rule, and, for every rewritten form, under the Windows rule (a Windows peer reading the wire form).
    - `*/cache` leaves `cache/needed.txt` in and `a/cache/x` out.
  - S2 `exclude_contract::copy_exclude_an_escaped_separator_after_a_recursive_wildcard_keeps_its_meaning` (blit-transfer, Unix — on Windows `\` is a separator; can go red on macOS and Linux): `blit copy src/ dst/ --exclude '**\/cache'` copies `cache/needed.txt` and `keep.txt` and leaves out `a/cache`. This is the reviewer's reproduction.
  - Unchanged and green: cr-g3fix4-3's `a_typed_unix_escape_is_written_in_the_class_dialect` (its `dir\/f` → `dir/f` still holds) and `a_typed_unix_escape_names_the_same_path_under_either_rule`, and the rest of `fs_enum`, `transfers::filter` and `exclude_contract`.
- N1 — `typed_exclude_dialect`: every escaped `/` written bare again (`if escaped == '/'` made `if false`, so `escape_as_class('/')` writes `/`; the pre-fix behaviour). Red: S1 FAILED at `fs_enum.rs:1253`, `left: "**/cache"`, `right: "*/cache"`; S2 FAILED at `exclude_contract.rs:686`, "a top-level cache is not named" (the reviewer's data omission).
- N1b — N1 with S1's written-form assertion disabled for the run, to show that the reading half bites on its own. Red: S1 FAILED at `fs_enum.rs:1256`, "globset: **\/cache and **/cache on "cache"", `left: false`, `right: true`.
- N3 — `escaped_separator`: the leading-dot case removed. Red: S1 FAILED at `fs_enum.rs:1253`, `left: "./x"`, `right: "[.]/x"`.
- N4 — `escaped_separator`: the trailing case removed. Red: S1 FAILED at `fs_enum.rs:1253`, `left: "x/"`, `right: "x\\/"`.
- Restore after each: the fixed file copied back with `/bin/cp -f`; md5 `b93b90d73dadbbae24a88badfe3a5376` matched the saved fixed file every time. Then `cargo test -p blit-core --lib -- fs_enum transfers::filter` passed 41, and S2 passed. `cargo clippy -p blit-core -p blit-transfer --all-targets -- -D warnings` is clean.
- Line numbers are those of the fixed file; N1b adds one line and N3, N4 remove one and two lines in front of S1, so their raw reports were shifted by that many.

## Known gaps
- A pattern ending in an escaped `/` (`x\/`, `x\//`) is left as typed. Its Unix reading is the dangling `x\` (the trailing bare `/`s are trimmed), which names only an entry literally called `x\`; a Windows peer reads the bare `\` as a separator and names `x`. Under `mirror` to a Windows destination daemon that peer's deletion scope keeps more than this side excluded (the safe direction).
- `--include` is still not rewritten (cr-g3fix4-3's gap).
- S1's Windows-rule half is the parameterized rule on macOS, not a Windows runtime; S2 is Unix only.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
