# cr-g3fix5-2: A glob component prevents protection of differently spelled destination descendants

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:810 — respelled_below stops at the first glob component; crates/blit-core/src/fs_enum.rs:142 keeps path rules case-sensitive. Reproduced on macOS with src/x/cache and dst/x/Cache/saved.txt: both --exclude '**/cache' and the absolute --exclude '<src>/*/cache' omitted the source subtree but deleted saved.txt, exiting 0. The literal './x/cache' and bare 'cache' controls preserved it.

## Predicted observable failure
Valid relative and absolute glob exclusions still allow mirror to delete their excluded destination counterparts when stored spelling differs. This leaves cr-g3fix4-2 and the broader cr-g3fix-1 filesystem-case contract incomplete.

## Reviewer's suggested approach
Preserve destination filesystem equivalence throughout path-rule matching, including components after globs, without expanding or rewriting wildcard syntax. Add survival coverage for '**/cache' and absolute wildcard paths.

## Intake
Reproduced 2026-10-17 with `blit 1.0.0+cf3405f65366` (the release build of the pushed candidate; script `scratchpad/g3fix5/repro.sh`, output `repro.out`): `mirror -y --exclude '**/cache'` over `src/x/cache/` with a pre-existing `dst/x/Cache/saved.txt` printed `Deleted: 1 file(s), 2 dir(s)` and the file was gone; the absolute `--exclude <src>/*/cache` did the same. Admitted as HIGH on the same rule; cr-g3fix4-2's Known gaps had recorded the glob-component case as remaining.

## What
Under `mirror`, an exclude with a glob component (`**/cache`, `*/cache`, an absolute `<src>/*/cache`) protects the destination's own spelling of the literal components after the glob (`dst/x/Cache` for `**/cache`), on every route the deletion pass serves.

Fix shape, admitted: with cr-g3fix5-1 (see its shape) — literal components after a glob component are resolved at the destination per visited directory through `stored_name`, glob components are fold-matched when the destination ignores case, and `respelled_below` no longer stops at the first glob component. Survival guards for `**/cache`, `*/cache` and an absolute `<src>/*/cache` against `dst/x/Cache/saved.txt`, local carrier and destination daemon.

## Approach
- **One mechanism with cr-g3fix5-1.** `DestinationNames` (cr-g3fix5-1), which the deletion walk asks for each entry it visits, now reads path rules too. This replaces cr-g3fix4-2's static respelling from the root (`respelled_at`, `respelled_below`, removed), which stopped at the first glob component. A path rule's `respelled_below` walk is now this per-directory reading, and it no longer stops there.
- **Components.** A path rule's body (as `exclude_rule` reads it) is split at each `/` that separates components (`path_components`, new). A rule whose `/` lies inside a class or an alternation, or is escaped, or that has an empty component, is read whole instead (below). Each component becomes a `Step` (`path_steps`, new):
  - a literal name (`plain_name`) → `Name`: its own spelling, or a name the destination pairs with it in that directory, looked up with `stored_name` as cr-g3fix5-1 does. The lookups are memoized per directory, so there is one lookup per literal per visited directory where the rule has got that far, never per entry; no lookup is made for an ASCII literal where the destination compares case.
  - `**` before another component → `AnyDepth`: any number of components, none included. `**` last → `Rest`: one or more, as globset reads a trailing `/**` (`./cache/**` names what is inside `cache`, not `cache` itself).
  - any other glob → `Glob`: globset text matched against one component, or, when it holds `*` or `?`, against several joined with `/`. In this filter (no `literal_separator`) `*` and `?` match `/`, so `*/cache` names `a/b/cache` as globset reads it. Where the destination ignores case it is matched under cr-g3fix5-1's `fold`, the pattern text and the joined component text alike.
- **The reading.** Each path rule is a small automaton read down the depth-first chain the walk visits: each directory of the chain keeps every path rule's states (`Level::states`; a state is the next step, plus where a spanning glob began). A component is matched against its parent's states (`advance`, with `closure` passing over a `**` that matches nothing). A rule that reaches its end names that entry, and a directory named so marks everything below it. Path rules are read from the root, as the subtree rule reads them (anchored or not).
- **Rules with no literal component** (`*/*.tmp`), and the rules read whole, have nothing for the destination to resolve. Where the destination ignores case they join cr-g3fix5-1's folded set and are matched whole under the fold, so globset's own reading of them is kept.
- **What this covers.** `**/cache` names `x/Cache` and a top-level `Cache` (zero components before), `*/cache` names `x/Cache` and `a/b/CACHE` but not a top-level `Cache`, and the absolute `<src>/*/cache` is relativized to `*/cache` first (cr-g3fix-2), so the same. A tie after a glob (`*/ς` with a hard-linked entry named `alias` there) names every name, which no text fold could.
- **cr-g3fix5-1's FAST guard counts one more lookup.** Its second stand-in case now also counts the path rule `./a/b`'s `a`, looked up once, at the root, the one directory that rule reaches. The expectation went from 4 to 5. Its last assertion ("a path rule alone is not read here") was true of cr-g3fix5-1 only and is now "no exclude, nothing read".
- **cr-g3fix4-2's guards adapted.** `a_deletion_scope_path_rule_takes_the_destinations_spelling` (V1) tested `respelled_at` and is replaced by `a_deletion_scope_path_rule_is_read_component_by_component`. That test keeps V1's stand-in cases (`./cache`, `./cache/`, `cache/sub/*.tmp`, a glob component and a missing component, the one-member class `./data[[]1[]]`, the tie `./ς`, the Windows `.\cache`) and every `plain_name` assertion. Where V1 asserted that the walk stops at a glob, the new test asserts that the reading continues (`cache/s*b/x` names `Cache/s1b/X`). `the_deletion_scope_names_the_destinations_own_spelling` (V2) asserted that `destination_scope` added `./Cache` and `A/B` to the patterns. The patterns now stay as given and are read at the walk, so that assertion is `["./cache", "a/b", "name"]`; its out-of-scope and in-scope assertions are unchanged and green.
- Unchanged: the source-side copy filter, the wire form, the CLI (SIMPLE).

## Files changed
- `crates/blit-core/src/transfers/filter.rs:785-821` — `destination_scope` doc and body: every exclude goes to `DestinationNames`; the respelled patterns are no longer added.
- `crates/blit-core/src/transfers/filter.rs:822-1151` — `DestinationNames` reads path rules: the doc, `literals`/`names`/`paths`, `Step` and `State` (new), `Level::states`, `new` (rules split into steps, the rest folded), `protects` and `step` (path rules advanced along the chain), `lookup`.
- `crates/blit-core/src/transfers/filter.rs:1152-1294` — `path_components`, `path_steps`, `closure`, `advance` (new); `respelled_at` and `respelled_below` removed.
- `crates/blit-core/src/transfers/filter.rs:2535-2658` — guard `a_deletion_scope_path_rule_is_read_component_by_component` (new; replaces cr-g3fix4-2's `a_deletion_scope_path_rule_takes_the_destinations_spelling`).
- `crates/blit-core/src/transfers/filter.rs:2666-2701` — cr-g3fix4-2's `the_deletion_scope_names_the_destinations_own_spelling`: doc, and the pattern-list assertion (see Approach).
- `crates/blit-core/src/transfers/filter.rs:2712-2771` — guard `the_deletion_scope_names_the_destinations_spelling_after_a_glob_component` (new).
- `crates/blit-core/src/transfers/filter.rs:2846-2858` — cr-g3fix5-1's `the_deletion_scope_reads_a_literal_name_once_per_visited_directory`: the count with the path rule read, and the empty-pattern assertion (see Approach).
- `crates/blit-core/src/transfer_session/mod.rs:9138-9194` — guard `mirror_delete_pass_keeps_a_destination_spelling_after_a_glob_component` (new).
- `crates/blit-cli/tests/exclude_contract.rs:605-735` — guards `mirror_exclude_keeps_a_destination_spelling_after_a_glob_component` (every OS) and `push_mirror_keeps_a_destination_spelling_after_a_glob_component` (Unix) (new).

## Guard proof
- Guards (ran on macOS aarch64, APFS case-insensitive; `--nocapture` printed no "skipping" line):
  - R1 `transfers::filter::tests::a_deletion_scope_path_rule_is_read_component_by_component` (blit-core, every OS; a stand-in destination that ignores case, so it can go red on every OS). Thirteen rules, each with entries out of and in the deletion scope:
    - cr-g3fix4-2's cases: `./cache` names `Cache` and `Cache/saved.txt`, not `x/Cache` or `Cachex`; `./cache/` the same; `cache/sub/*.tmp` names `Cache/SUB/a.tmp` and `Cache/SUB/A.TMP`, not `Cache/SUB/a.txt` or `Cache/a.tmp`; `./cache/missing/x` names `Cache/missing/x`, not `Cache/Missing/x` (a spelling the stand-in does not pair); `./data[[]1[]]` names `DATA[1]/f`; the tie `./ς` names `alias` and `σ/f`.
    - The continuation: `cache/s*b/x` names `Cache/s1b/X/saved.txt` (the destination's `X` after a glob), `Cache/sub/x` and `Cache/s/b/x` (`*` spans `/`), not `Cache/s1b/y`.
    - `**/cache` names `Cache/f`, `x/Cache/saved.txt` and `a/b/CACHE/saved.txt`, not `x/other` or `a/b/other/cache2`.
    - `*/cache` names `x/Cache/saved.txt` and `a/b/CACHE`, not `Cache/f` (a component before is required) or `x/other`.
    - `X*/cache` names `xa/Cache/saved.txt` (the glob folded), not `ya/Cache`.
    - `*/ς` names `q/alias/f` (a tie after a glob, which no fold gives), not `q/other` or `alias`.
    - `./cache/**` names `Cache/f` and `Cache/d/f`, not `Cache` itself.
    - `*/*.TMP` (no literal) names `x/a.tmp`, not `a.tmp`.
    - The Windows rule's `.\cache` names `Cache`. Every `plain_name` assertion of cr-g3fix4-2's V1 is kept. `path_components` splits `a/b*/c` and refuses `{a/b,c}/d`, `a[/]b/c`, `a\/b/c` and `a//b`.
  - R2 `transfers::filter::tests::the_deletion_scope_names_the_destinations_spelling_after_a_glob_component` (blit-core, real filesystem; case-insensitive filesystems only, so it can go red on APFS and NTFS). Over a destination holding `x/Cache/saved.txt`, `a/b/CACHE/saved.txt`, `Cache/top.txt` and `x/other.txt`:
    - `**/cache` leaves the first three out of the scope and keeps `x/other.txt` in;
    - `*/cache` leaves the first two out and keeps `Cache/top.txt` and `x/other.txt` in;
    - the source-spelled filter allows each of them (the bug).
  - R3 `transfer_session::tests::mirror_delete_pass_keeps_a_destination_spelling_after_a_glob_component` (blit-core; case-insensitive filesystems only). `mirror_delete_pass` keeps `x/Cache/saved.txt` under `**/cache` and under `*/cache`, and `a/b/CACHE/saved.txt` under `*/cache`; it deletes `stale.txt` each time, counts `(1, 0)`.
  - R4 `exclude_contract::mirror_exclude_keeps_a_destination_spelling_after_a_glob_component` (blit-transfer, every OS; case-insensitive filesystems only). The local carrier, the reported runs: `--exclude '**/cache'`, `--exclude '*/cache'` and the absolute `--exclude <src>/*/cache`, over `src/x/cache/` against `dst/x/Cache/saved.txt`. Each keeps `saved.txt` and copies `keep.txt`, with no unmatchable warning.
  - R5 `exclude_contract::push_mirror_keeps_a_destination_spelling_after_a_glob_component` (blit-transfer, Unix; case-insensitive filesystems only). The destination daemon: push mirrors with `--exclude '**/cache'`, then `--exclude <src>/*/cache`, each keep the module's `x/Cache/saved.txt` and delete `extra.txt`.
  - Unchanged and green: cr-g3fix5-1's guards (L1 with its count updated, see Approach), cr-g3fix4-2's V2 (its pattern-list assertion updated), V4, V5 and V6, and the rest of `transfers::filter`, `fs_enum`, `mirror_delete_pass` and `exclude_contract`.
- P1 — `DestinationNames::new`: a path rule holding a glob component is not read at all. Only all-literal path rules are read, so this is the shape of the pre-fix coverage, where the respelling stopped at the first glob. Red:
  - R1 FAILED at `filter.rs:2571`, "cache/sub/*.tmp is read at the destination" (nothing read for it).
  - R2 FAILED at `filter.rs:2761`, "**/cache: x/Cache/saved.txt: out of the deletion scope".
  - R3 FAILED at `transfer_session/mod.rs:9183`, "**/cache: x/Cache/saved.txt is excluded and must survive".
  - R4 FAILED at `exclude_contract.rs:662`, naming all three cases with the reported "• Deleted: 1 file(s), 2 dir(s)" (double star, star, absolute star).
  - R5 FAILED at `exclude_contract.rs:729`, naming both patterns with "Remote purge removed 4 entries." (the daemon route's data loss).
- P1b — such a rule read whole under the fold instead of component by component. Red: R1 FAILED at `filter.rs:2615`, "*/ς: q/alias/f is out of the deletion scope". R2–R5 stayed green under P1b: on APFS a case variant is also caught by the fold, so only a pairing the fold cannot give (a tie after a glob) tells the literal reading from it.
- P2 — a glob component never spans several components. Red: R1 FAILED at `filter.rs:2615`, "cache/s*b/x: Cache/s/b/x is out of the deletion scope"; R2 FAILED at `filter.rs:2761`, "*/cache: a/b/CACHE/saved.txt: out of the deletion scope"; R3 FAILED at `mod.rs:9183`, "*/cache: a/b/CACHE/saved.txt is excluded and must survive".
- P3 — `closure` removed (a `**` never matches no component). Red: R1 FAILED at `filter.rs:2615`, "**/cache: Cache/f is out of the deletion scope"; R2 FAILED at `filter.rs:2761`, "**/cache: x/Cache/saved.txt …"; R3 FAILED at `mod.rs:9183`, "**/cache: x/Cache/saved.txt is excluded and must survive".
- P4 — glob components not folded. Red: R1 FAILED at `filter.rs:2615`, "cache/sub/*.tmp: Cache/SUB/a.tmp is out of the deletion scope" (`*.tmp` against the folded `A.TMP`).
- P5 — a trailing `**` read as `AnyDepth` (none included). Red: R1 FAILED at `filter.rs:2621`, "./cache/**: Cache stays in the deletion scope".
- P6 — a literal component's own spelling not matched (lookups only). Red: R1 FAILED at `filter.rs:2615`, "cache/s*b/x: Cache/sub/x is out of the deletion scope".
- Restore after each: the fixed `filter.rs` copied back with `/bin/cp -f`; md5 `4f44cb5ac799311a9c204eaac57898ed` matched the saved fixed file every time. Then `cargo test -p blit-core --lib -- transfers::filter mirror_delete_pass fs_enum` passed 53, and R4 and R5 passed.
- After these runs, `protects`' loop over the chain was rewritten with an iterator for clippy (`needless_range_loop`), with no change of behaviour, and rustfmt joined one call, moving every later `filter.rs` line up by five. The line numbers above are those of the final file (the raw reports were five higher, and P1's six: it adds a line). On the final file (md5 `2c510076c9dfb7005a470146665976c2`) the guards were run green again and P1 re-run: red again at `filter.rs:2571` (R1, "cache/sub/*.tmp is read at the destination"), `filter.rs:2761` (R2, "**/cache: x/Cache/saved.txt: out of the deletion scope") and `transfer_session/mod.rs:9183` (R3); restored (md5 matched), then `transfers::filter mirror_delete_pass fs_enum` passed 53 and the whole `exclude_contract` 20. `cargo clippy -p blit-core -p blit-transfer --all-targets -- -D warnings` is clean on the final file.

## Known gaps
- A path rule whose `/` lies inside a class or an alternation (`{a/b,c}/d`), or is escaped, is not split. Where the destination ignores case it is matched whole under the fold, so a case variant is covered; a pairing beyond the fold (a tie, a filesystem-only equivalence) is not, for that rule.
- A glob component holding `*` or `?` may span several components (globset's reading here), and each place one could start is followed: the cost of such a rule after a `**` grows with the square of the depth, per entry. Rules of that shape are rare; plain `**/name`, `*/name` and literal path rules cost one step per entry.
- Glob components are folded only where the destination ignores case (cr-g3fix5-1's fold, with its gaps); on a case-sensitive but normalization-insensitive destination a glob component's NFC text does not pair with an NFD name. A literal component is looked up there and pairs.
- The case rule is probed once at the root (cr-g3fix4-2's and cr-g3fix5-1's gap).
- R2–R5 run only where the filesystem ignores case: macOS here (none skipped). On Linux CI (ext4) they print a skip line, so they can go red only on a case-insensitive filesystem. On Windows CI R2–R4 run; R5 is Unix-only like the other daemon tests. R1 runs and can go red everywhere.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
