# cr-g3fix5-4: Bracket-aware pattern parsing is incorrectly applied to literal Windows roots

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `80c6072e`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:390 and :415 — absolute_components applies the bracket-aware normalizer to the source root as well as the pattern. A parameterized Windows-rule probe on macOS used root 'C:\data[1\test]\src' and pattern 'C:/data[1/test]/src/cache'; relative_to_root returned None instead of ['cache']. The backslash between the root's two literal directory names was treated as a bracket-class member. through_resolved_root also stops at the bracket-bearing ancestor at line 335.

## Predicted observable failure
On Windows, an absolute exclude using forward slashes fails to relativize against an equivalent backslash-spelled source root whose directory components contain brackets spanning a separator. The command reports the exclude as unmatchable and proceeds without excluding the requested subtree.

## Reviewer's suggested approach
Tokenize filesystem roots and resolved prefixes as literal paths, normalizing every native separator. Reserve bracket-aware tokenization for user pattern text, and test mixed separators with bracket-bearing root components.

## Intake
Admitted on analysis (the reviewer's parameterized Windows-rule probe on macOS: root `C:\data[1\test]\src`, pattern `C:/data[1/test]/src/cache`, `relative_to_root` → `None`): cr-g3fix4-4 applied the bracket-aware `normalize_separators` to every input of `absolute_components`, including the source root and the resolved root, which are literal paths and hold no classes. cr-g3fix4-4's Known gaps had recorded exactly this root shape.

## What
Under the Windows rule, `absolute_components` reads a literal path (the source root, the resolved root, a verbatim or UNC prefix) with every `\` a separator, and only the typed pattern bracket-aware, so `C:/data[1/test]/src/cache` relativizes against the root `C:\data[1\test]\src` to `./cache`.

Fix shape, admitted: `absolute_components` splits literal paths — the source root, the resolved root, `\\?\` and UNC prefixes — with the plain separator replace, and only the typed pattern bracket-aware (`normalize_separators`); `through_resolved_root` the same. Guard: root `C:\data[1\test]\src`, pattern `C:/data[1/test]/src/cache` → `./cache`, and the cr-g3fix4-4 class case still `./back[s\]lash`.

## Approach
- **Literal paths are split as literal paths.** `literal_components_at` (new, `transfers/filter.rs`) splits a path at every separator the platform has — on Windows `\` and `/` alike, the plain replace cr-g3fix4-4 had swapped out — drops a verbatim `\\?\` prefix and keeps a UNC marker `//`, exactly as `absolute_components` did before cr-g3fix4-4. It also returns where each component ends in the input. `literal_components` is the same without the offsets. Every literal path goes through it: the source-root forms (`std::path::absolute` and `canonicalize`), the resolved root, and the resolved prefix `through_resolved_root` returns. A root `C:\data[1\test]\src` therefore names the directories `data[1` and `test]`.
- **Only typed pattern text is bracket-aware.** `typed_components` (new) splits typed text with `normalize_separators` (cr-g3fix3-3, cr-g3fix4-4), so a class keeps its `\`.
- **`relative_to_root` compares the pattern's leading part with the root as a literal path.** It reads the pattern with `literal_components_at`, matches the root's components against the first ones (case-insensitively on Windows, `same_component`), and splits only the text after the last matched component with `typed_components`. So `C:/data[1/test]/src/cache` (the reviewer's probe) and `C:\data[1\test]\src\cache` (typed like the root) both relativize to `./cache`, and `C:\src\back[s\]lash` is still `./back[s\]lash`.
- **Recorded deviation from the admitted shape.** The shape says "only the typed pattern bracket-aware". Read literally, the whole typed pattern would be split bracket-aware and compared with a literally-split root. Then a pattern typed with `\` exactly like the root (`C:\data[1\test]\src\cache`) would read `data[1\test]` as one class and fail to relativize. That is a regression: it works today and before cr-g3fix4-4, and cr-g3fix4-4's Known gaps recorded it as working. So the part of the pattern that is compared with the root is read as the literal path it is compared with, and only the text below the root is pattern text. A class can never equal a literal root component (no Windows name holds `\`), so this changes nothing a class could have matched. It only lets the root's own names through in either slash.
- **The resolved form is kept apart, not re-split.** Before, the resolved prefix and the typed rest were joined into one string and split again (`relative_parts`, removed). Now `through_resolved_root` returns the resolved prefix and the typed rest's text, and `resolved_parts` (new) makes the prefix's components below the root `Part::Literal` (split as a literal path) and the rest `Part::Typed` (split as pattern text). `resolve_excludes_on` tries the typed form against each root first, then the resolved form, in the same order as before.
- **`through_resolved_root` walks the pattern's literal reading.** Its ancestors are now the pattern's leading text up to each component of `literal_components_at` (the platform resolves them as literal paths), in place of `Path::ancestors`. A bare drive is resolved with its separator (`C:\`), never as that drive's current directory (`is_drive`). The reviewer also noted the walk "stops at the bracket-bearing ancestor". A component now stops it only when it holds glob syntax on its own (`holds_glob_syntax`, new): a wildcard, an alternation, a class that closes inside the component, or, off Windows, `\`. A `[` that never closes inside its component (`data[1`, with `test]` next) opens no class there and is the name's own character. An ancestor is used only if it exists and lands at or below the root, and the walk returns at the shortest one that does, so a bracket after the landing point is still typed text. `has_glob_meta` is unchanged for the stored-spelling walk below the root.
- `bracket_class_len` (`fs_enum.rs`) is now `pub(crate)` so `holds_glob_syntax` can use it.

## Files changed
- `crates/blit-core/src/fs_enum.rs:327` — `bracket_class_len` made `pub(crate)`.
- `crates/blit-core/src/transfers/filter.rs:12-15` — imports `bracket_class_len`.
- `crates/blit-core/src/transfers/filter.rs:271-284` — `resolve_excludes_on`: the typed form, else the resolved form through `resolved_parts`.
- `crates/blit-core/src/transfers/filter.rs:310-386` — `through_resolved_root` (doc and walk over `literal_components_at`; returns the typed rest's text), `is_drive` and `holds_glob_syntax` (new).
- `crates/blit-core/src/transfers/filter.rs:408-505` — `literal_components_at`, `literal_components`, `typed_components`, `same_component` (new; `absolute_components` removed), `relative_to_root` (literal leading part, typed rest).
- `crates/blit-core/src/transfers/filter.rs:516-543` — `resolved_parts` (new; `relative_parts` removed).
- `crates/blit-core/src/transfers/filter.rs:1915-2020` — guards `a_windows_literal_root_is_split_at_every_separator` and `a_pattern_through_an_alias_named_with_an_unclosed_bracket_is_resolved` (new).

## Guard proof
- Guards (ran on macOS aarch64; the Windows rule is a parameter, so it ran here as string handling, not as a Windows runtime):
  - W1 `transfers::filter::tests::a_windows_literal_root_is_split_at_every_separator` (blit-core, every OS, pure; can go red on every OS):
    - Under the Windows rule and the root `C:\data[1\test]\src`, three patterns give `./cache`: `C:/data[1/test]/src/cache` (the reviewer's probe), `C:\data[1\test]\src\cache` (typed like the root), and the mixed-slash, other-case `c:\DATA[1/Test]\src/cache`. Each gives `./cache` under the verbatim root `\\?\C:\data[1\test]\src` too.
    - Below that root a class keeps its `\`: `C:/data[1/test]/src/a/back[s\]lash` → `a/back[s\]lash`.
    - The UNC form `\\srv\share\d[1\x]\y\z` under `\\?\UNC\srv\share\d[1\x]` → `y/z`. A sibling path, and the root itself, → `None`.
    - `resolved_parts`: the prefix `\\?\C:\data[1\test]\src\sub` with the rest `\back[s\]lash` gives `[Literal("sub"), Typed("back[s\]lash")]`; the root with the rest `/cache` gives `[Typed("cache")]`; an empty rest at the root gives `None`.
    - `holds_glob_syntax`: `data[1` and `test]` are not glob syntax under either rule; `data[1]`, `a*`, `{a,b}`, and `a\b` under the Unix rule are.
  - W2 `transfers::filter::tests::a_pattern_through_an_alias_named_with_an_unclosed_bracket_is_resolved` (blit-core, Unix — making a symlink on Windows needs privilege; can go red on macOS and Linux): `<tmp>/a[1/b]/link/cache`, with `link` a symlink to the source root `<tmp>/src`, resolves to `["./cache"]` with nothing unmatchable.
  - Unchanged and green: `windows_absolute_excludes_compare_case_insensitively_with_either_slash` (verbatim and UNC prefixes, either slash, case), cr-g3fix4-4's `a_windows_absolute_exclude_keeps_a_bracket_class_backslash` (the class case still `./back[s\]lash`), and the symlinked-root and absolute-glob guards of cr-g3fix-2 and cr-g3fix2-2.
- M1 — `literal_components_at`: the literal paths split bracket-aware again (`normalize_separators(path, true)` in place of the plain replace; the pre-fix reading of every root). Red: W1 FAILED at `filter.rs:1937`, `left: None`, `right: Some("./cache")` (the reviewer's probe).
- M2 — `through_resolved_root`: the walk stops at any component holding `[` again (`has_glob_meta` in place of `holds_glob_syntax`; the pre-fix criterion). Red: W2 FAILED at `filter.rs:2017`, `left: ["/var/folders/…/a[1/b]/link/cache"]`, `right: ["./cache"]` (the pattern left as typed: unmatchable).
- M3 — `resolved_parts`: the typed rest split as a literal path. Red: W1 FAILED at `filter.rs:1962`, `left: Some([Literal("sub"), Typed("back[s"), Typed("]lash")])`, `right: Some([Literal("sub"), Typed("back[s\\]lash")])`.
- M4 — `relative_to_root`: the text below the root split as a literal path. Red: cr-g3fix4-4's guard FAILED at `filter.rs:1881`, `left: Some("back[s/]lash")`; W1 FAILED at `filter.rs:1948`, `left: Some("a/back[s/]lash")`, `right: Some("a/back[s\\]lash")`.
- Restore after each: the fixed file copied back with `/bin/cp -f`; md5 `da9092ff3c316f75fadd196aa8f29535` matched the saved fixed file every time. Then `cargo test -p blit-core --lib -- transfers::filter fs_enum` passed 40. `cargo clippy -p blit-core --all-targets -- -D warnings` is clean.

## Known gaps
- The part of a pattern compared with the root is read as a literal path in both slashes; only the text below the root is pattern text. A pattern that means a class spanning the root's last separator (`C:\src\a[b\]c` against a root `C:\src\a[b`) is read as the path `]c` below that root. Such a root is unusual; that reading is the literal one the root's own name gives.
- The stop rule of `through_resolved_root` treats a `[` that never closes inside its component as a name's character, under the Unix rule as well. A typed Unix pattern whose alias part spans a class across `/` (`/x/a[1/b]/…`, a class of `1`, `/`, `b`) is resolved through it only if a literal path of that text exists and lands in the source, which is the literal reading.
- W1 is the parameterized Windows rule on macOS, not a Windows runtime. W2 runs on Unix only; CI's Windows leg runs W1 and the build/build_spec parts of cr-g3fix4-4's guard under the host rule.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
