# cr-g3fix4-2: Destination spelling differences still expose excluded subtrees to mirror deletion

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:127 — subtree matching compares the destination's spelling against the source-derived case-sensitive pattern. transfer_session/mod.rs:4177 reuses that filter for destination enumeration. Reproduced on case-insensitive macOS with src/cache pointing outside, absolute exclude src/CACHE, and existing dst/Cache/saved.txt: mirror deleted the saved file and directory. The identical run with destination spelling dst/cache preserved them.

## Predicted observable failure
Even after source-name resolution succeeds, mirror deletes an excluded destination counterpart whose stored spelling differs from the source's. This leaves the earlier filesystem-case contract materially open.

## Reviewer's suggested approach
Resolve literal exclusions against the destination filesystem when constructing deletion scope, preserving glob semantics and protecting equivalent destination spellings. Add a survival test where source, typed exclude, and destination use different spellings.

## Intake
Admitted; reproduced by the orchestrator with the release binary `blit 1.0.0+548625f5c7cf` on macOS APFS (`scratchpad/g3fix4/repro.out`), and wider than the reviewer's case: a plain directory `src/cache` excluded as `<src>/cache`, typed exactly as stored, with a pre-existing `dst/Cache/saved.txt` — `blit mirror -y` reported "Deleted: 1 file(s), 1 dir(s)". The symlink form reproduced the same. Root cause: the mirror deletion scope (`transfer_session/mod.rs` ~4171-4190, `mirror_filter` — the user's `FileFilter` on the local carrier, `filter_from_spec` on a daemon) applies the source-spelled patterns to the destination walk as typed, case-sensitively; the destination's own stored spelling is never consulted. Recorded as an observation since cr-g3fix2-1; it is a data-loss path under `mirror`, so it cannot stay one. Fix shape (the reviewer's): where the deletion-scope filter is built — that one place, which runs wherever the destination is local, the local carrier and a destination daemon alike, since the destination walk is the session's own — respell each exclude pattern's literal path components against the destination root with the same `stored_name` machinery the source side uses. A pattern with a `/` (anchored `./a/b` or relative `a/b`) is walked component by component from the destination root: a component is plain when it holds no glob syntax other than the single-character classes `globset::escape`/`literal_glob` emit (`[*]` `[?]` `[[]` `[]]` `[{]` `[}]` `[\]`); it is unescaped to its name, probed with `stored_name(dst_root/so-far/name)`, and re-escaped; the walk stops at the first component with real glob syntax or that does not exist on the destination. The respelled pattern is ADDED beside the original: the original still protects a destination spelled like the source, the respelled one protects the destination's own spelling, and adding patterns can only shrink the deletion scope. A tie (cr-g3fix4-1) adds every name. The any-depth name rule (a pattern with no `/`, `--exclude cache`) cannot be respelled per directory; for it the deletion-scope filter's name globs are compiled `case_insensitive(true)` when the destination filesystem is case-insensitive, detected once at the root without writing anything: `symlink_metadata` of the root path with the case of its last alphabetic component swapped resolves to the root's own identity (no letter to swap → assume case-sensitive, and say so in the doc). Normalization-insensitivity for the any-depth rule (an NFC name against an NFD destination) is out of reach without a per-entry probe and is recorded as the remaining gap (APFS preserves the spelling blit wrote, so it needs a destination created by another tool in the other form). FAST: both run once when the deletion-scope filter is built, never per entry; a case-insensitive globset matches at the same cost. Guards: CLI `mirror` survival with `src/cache` (a directory) excluded as `<src>/cache` against a pre-existing `dst/Cache/saved.txt`, and the symlink form, and a nested `a/b` form, each skipped where the destination filesystem is case-sensitive; the any-depth form `--exclude cache` with `dst/Cache` (case-insensitive fs); unit tests of the respelling helper (pure, a stand-in `stored` closure; the escape-aware component parser; a glob component stops the walk; a missing destination component stops it); the daemon route through the existing loopback-daemon mirror test in `exclude_contract` if the harness allows a destination-spelling case; mutation-red.

## What
Under `mirror`, an excluded entry that the destination stores in another spelling than the source now stays outside the deletion scope. The reproduced cases on case-insensitive APFS all keep `dst/Cache/saved.txt`:
- a directory `src/cache` excluded as `<src>/cache`, with a pre-existing `dst/Cache/saved.txt`;
- the symlink form, `src/cache -> <outside>` excluded as `<src>/CACHE`;
- a nested `src/a/b` against `dst/A/B`;
- the any-depth `--exclude cache` against `dst/x/Cache`.

This holds for the local carrier and for a destination daemon. Before, the deletion scope applied the source-spelled patterns (`./cache`) to the destination walk case-sensitively. `dst/Cache` was named by no rule, counted as extraneous, and `blit mirror -y` reported "Deleted: 1 file(s), 1 dir(s)".

## Approach
- **Where.** `mirror_delete_pass` (`transfer_session/mod.rs`) now plans its deletions with `transfers::filter::destination_scope(filter, dst_root)` in place of the filter it is given. Every destination's deletions are planned there, from exactly one production call site: the `SourceDone` arm of `destination_session_inner`. Verified by reading the callers: all of the following reach it through `drive_destination` → `destination_session` → `destination_session_inner`:
  - the local carrier (`transfer_session/local.rs` → `run_destination`);
  - a pull's client (`remote/transfer/session_client.rs` → `run_destination`);
  - a destination daemon serving a push (`blit-daemon` `service/transfer.rs` → `run_responder` → `drive_destination`);
  - a delegated pull's destination daemon (`service/delegated_pull.rs` → `run_destination`).

  The deletion-scope filter is still built where it was (`mirror_filter`: the local carrier's `la.mirror_scope_filter`, or `filter_from_spec` on a daemon). The destination is read at the pass, not where `mirror_filter` is built, for two reasons. The pass runs on the blocking pool, while the filter is built in the async session body, where filesystem reads would block the runtime. And the pass runs after every payload has landed, so the destination is read once, as it stands. The source side's copy filter is untouched.
- **Path rules are respelled and added.** `destination_scope` reads each exclude as `ExcludeRules` does, through `fs_enum::exclude_rule` (new; `ExcludeRules::compile` now uses it too, so the two cannot disagree). For each path rule (anchored `./a/b`, or `a/b`), `respelled_at` walks its components from the destination root:
  - `plain_name` reads a component as a name when it holds no glob syntax other than one-member classes, the escapes `literal_glob` and the cr-g3fix4-3 rewrite write (`[*]`, `[[]`, `[\]`, …), each read as its character.
  - `stored_name` (cr-g3fix4-1; the entry itself, never a link's target) gives the destination's own names for it, escaped again with `literal_glob`.
  - The walk stops at the first component with real glob syntax, or one the destination does not hold, and the rest is kept as written. A tie forks the walk, one pattern per name. The anchor is kept.
  - A respelled pattern that differs from the original is added beside it. The original still names a destination spelled like the source, the respelled one names the destination's own spelling, and adding patterns can only shrink the deletion scope.
- **Name rules match case-insensitively at a case-insensitive destination.** A name rule (a pattern with no `/`) cannot be respelled per directory. When the filter has one and `case_insensitive_at(dst_root)` holds, the scoped filter's new `case_insensitive_names` flag makes `ExcludeRules` compile its name globs with `GlobBuilder::case_insensitive(true)`. Path rules and the whole-pattern arm stay case-sensitive.
- **Detection, read without writing.** `case_insensitive_at` takes an entry the root lists whose name holds an ASCII letter, swaps the case of every ASCII letter, and checks with `entry_identity` (cr-g3fix4-1) that the swapped path resolves to that same entry. Recorded deviation: the shape probed the root path's own last component; this probes an entry of the root first, among its first 64 entries, and falls back to the root's own name and then its ancestors only when none has an ASCII letter. A destination root is often a mount point (`/Volumes/Backup`), and the root's own name is looked up by the parent filesystem, which can differ from the destination's; an entry's name is looked up by the destination filesystem itself. ASCII letters only, so the swap is one character for one and every case-insensitive filesystem folds it. If no name anywhere has a letter, the destination is assumed case-sensitive.
- **FAST.** Nothing runs when no exclude is given or `files_from` is set (it bypasses the excludes), or for `MirrorMode::All`, whose filter is empty. Otherwise there is one directory read per leading literal component of each path rule (plus identity reads only where the spelling differs), and one listing probe of the root when a name rule exists. All of it is once per deletion pass, never per entry; a case-insensitive glob set matches at the same cost.
- **Embedders.** `FileFilter::case_insensitive_names` is `pub(crate)`: no new public knob, and no CLI option (SIMPLE).
- No other deviation from the admitted fix shape.

## Files changed
- `crates/blit-core/src/fs_enum.rs:55-61` — `FileFilter::case_insensitive_names` (new, `pub(crate)`).
- `crates/blit-core/src/fs_enum.rs:87-142` — `ExcludeRule` and `exclude_rule` (new); `ExcludeRules::compile` reads patterns through it and takes the name rules' case flag.
- `crates/blit-core/src/fs_enum.rs:180, 198-207` — `excluded_under_rule` passes `false`; `CompiledPatterns::compile` takes a case flag (`GlobBuilder`).
- `crates/blit-core/src/fs_enum.rs:356, 379, 469, 478-484, 631` — the flag in `clone_without_cache`, `pruning_rules` and `Default`; `file_globs` and `subtree_rules` pass it.
- `crates/blit-core/src/fs_enum.rs:921, 944, 965, 982, 1013` — test call sites of `ExcludeRules::compile` pass `false`.
- `crates/blit-core/src/transfers/filter.rs:13-14` — imports `exclude_rule`.
- `crates/blit-core/src/transfers/filter.rs:714-906` — `destination_scope`, `respelled_at`, `respelled_below`, `plain_name`, `case_insensitive_at` (new).
- `crates/blit-core/src/transfers/filter.rs:1947-2101` — guards `a_deletion_scope_path_rule_takes_the_destinations_spelling`, `the_deletion_scope_names_the_destinations_own_spelling`, `the_destinations_case_rule_is_read_without_writing` (new).
- `crates/blit-core/src/transfer_session/mod.rs:4027-4036, 4049, 4056` — `mirror_delete_pass` doc; it plans with `destination_scope`.
- `crates/blit-core/src/transfer_session/mod.rs:8966-9020` — guard `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` (new).
- `crates/blit-cli/tests/exclude_contract.rs:400-488` — guard `mirror_exclude_keeps_a_destination_entry_stored_in_another_case` (new; directory, nested, any-depth, and symlink on Unix).
- `crates/blit-cli/tests/exclude_contract.rs:778-828` — guard `push_mirror_keeps_a_destination_entry_stored_in_another_case` (new; the destination daemon).

## Guard proof
- Guards (ran on macOS aarch64, APFS case-insensitive; `--nocapture` printed no "skipping" line):
  - V1 `transfers::filter::tests::a_deletion_scope_path_rule_takes_the_destinations_spelling` (blit-core, every OS, stand-in destination). Expected respellings:
    - `./cache` → `./Cache`, and `./cache/` the same.
    - `cache/sub/*.tmp` → `Cache/SUB/*.tmp`.
    - A glob component ends the walk (`cache/s*b/x` → `Cache/s*b/x`), and so does a component the destination does not hold (`./cache/missing/x` → `./Cache/missing/x`).
    - `./data[[]1[]]` → `./DATA[[]1[]]` (a one-member class read as its character, the stored name escaped).
    - A tie, `./ς` → `./alias` and `./σ`.
    - Nothing is added for the name rule `cache`, for a same-spelled `./same`, for a missing `./missing`, or for `ca*/x`.
    - Under the Windows rule `.\cache` → `./Cache`. `plain_name` reads `[*]x[?]`, `a[]]b[[]`, `[\]` and `a,b-!^]` and refuses `[ab]`, `[!a]`, `[^]`, `a*`, `{a,b}`, `a\b`, `[x`, `.`, `..` and the empty name; under the Windows rule it refuses `[\]`.
  - V2 `transfers::filter::tests::the_deletion_scope_names_the_destinations_own_spelling` (blit-core; case-insensitive filesystems only). Over a real destination holding `Cache/saved.txt`, `A/B/saved.txt` and `x/NAME/saved.txt`:
    - the source-spelled filter `["./cache", "a/b", "name"]` allows `Cache/saved.txt` (the bug);
    - `destination_scope` gives `["./cache", "a/b", "name", "./Cache", "A/B"]` and leaves all three out of scope, while `other/x` and `A/keep.txt` stay in scope.
  - V3 `transfers::filter::tests::the_destinations_case_rule_is_read_without_writing` (blit-core, every OS). `case_insensitive_at` writes nothing, and it agrees with a probe that writes a file in three cases: an empty root (the root's own name), a root holding only `123`, and a root holding `Entry.txt`.
  - V4 `transfer_session::tests::mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` (blit-core; case-insensitive filesystems only). `mirror_delete_pass` with excludes `./cache`, `a/b`, `name` and an empty source set keeps `Cache/saved.txt`, `A/B/saved.txt` and `x/NAME/saved.txt` and deletes `stale.txt`.
  - V5 `exclude_contract::mirror_exclude_keeps_a_destination_entry_stored_in_another_case` (blit-transfer; case-insensitive filesystems only). The local carrier, end to end. Every case runs, and the ones that lost the excluded entry are named together:
    - directory: `src/cache` excluded as `<src>/cache`, against `dst/Cache/saved.txt`;
    - nested: `<src>/a/b` against `dst/A/B/saved.txt`;
    - any depth: `--exclude cache` against `dst/x/Cache/saved.txt`;
    - symlink (Unix): `src/cache -> <outside>` excluded as `<src>/CACHE`, against `dst/Cache/saved.txt`.
  - V6 `exclude_contract::push_mirror_keeps_a_destination_entry_stored_in_another_case` (blit-transfer, Unix; case-insensitive filesystems only). The destination daemon: a push mirror with `--exclude <src>/cache` keeps the module's `Cache/saved.txt` and deletes `extra.txt`.
  - Unchanged and green: every `fs_enum` rule test (now through `exclude_rule`), the `mirror_delete_pass` tests, `push_mirror_relativizes_an_absolute_exclude_and_prunes_it`, and the rest of `exclude_contract`.
- H1 — `mirror_delete_pass`: plans with the filter as given (`filter.clone_without_cache()` in place of `destination_scope`), i.e. the pre-fix pass. Red:
  - V4 FAILED at `transfer_session/mod.rs:9015`, "Cache/saved.txt is excluded and must survive".
  - V5 FAILED at `exclude_contract.rs:482`, naming all four cases, each with the mirror's "• Deleted: 1 file(s), 1 dir(s)" (directory, nested, any depth, symlink).
  - V6 FAILED at `exclude_contract.rs:819`, "the daemon's deletion scope must name the destination's own spelling", with the client's "Remote purge removed 3 entries." (the reported data loss on the daemon route).
- H2 — `destination_scope`: no respelled pattern added (`.take(0)`). Red: V4 FAILED at `mod.rs:9015` ("Cache/saved.txt …"); V2 FAILED at `filter.rs:2057`, `left: ["./cache", "a/b", "name"]`; V5 FAILED at `exclude_contract.rs:482`, naming directory, nested and symlink (any depth stayed green); V6 FAILED at `exclude_contract.rs:819` ("Remote purge removed 3 entries.").
- H3 — `destination_scope`: `case_insensitive_names` never set. Red: V4 FAILED at `mod.rs:9015`, "x/NAME/saved.txt is excluded and must survive"; V2 FAILED at `filter.rs:2062`, "x/NAME/saved.txt: out of the deletion scope"; V5 FAILED at `exclude_contract.rs:482`, naming any depth alone.
- H4 — `plain_name`: every class refused (a one-member class no longer read as its character). Red: V1 FAILED at `filter.rs:1984`, `left: []`, `right: ["./DATA[[]1[]]"]`.
- H5 — `CompiledPatterns::compile`: the case flag ignored (`case_insensitive && false`). Red: V4 FAILED at `mod.rs:9015` ("x/NAME/saved.txt …"); V2 FAILED at `filter.rs:2062` ("x/NAME/saved.txt: out of the deletion scope").
- H6 — `case_insensitive_at`: the identity comparison inverted. Red: V3 FAILED at `filter.rs:2097`, "an entry of the root", `left: false`, `right: true`; V4 FAILED at `mod.rs:9015`; V2 FAILED at `filter.rs:2062`; V5 FAILED at `exclude_contract.rs:482` ("any depth").
- H7 — `respelled_below`: a tie keeps its first name only. Red: V1 FAILED at `filter.rs:1989`, "a tie adds every name", `left: ["./alias"]`, `right: ["./alias", "./σ"]`.
- H8 — `respelled_at`: the anchor dropped. Red: V1 FAILED at `filter.rs:1967`, `left: ["Cache"]`, `right: ["./Cache"]`; V2 FAILED at `filter.rs:2057`, `left: [..., "Cache", "A/B"]`.
- Restore after each: the fixed file copied back with `/bin/cp -f`; md5 `1aebbd255a9cd0bb8ba1efd7a121f796` (`filter.rs`), `1e2495c312c981757cf57782668fa366` (`fs_enum.rs`) and `e9961dde9a412e50c36543e8ae12826d` (`transfer_session/mod.rs`) matched the saved fixed files every time. Then `cargo test -p blit-core --lib -- transfers::filter fs_enum mirror_delete_pass` passed 43 and `cargo test -p blit-transfer --test exclude_contract` passed 15. H1–H3 ran first against V5 stopping at its first failing case; V5 was then changed to run every case and report all of them (a test-only change), and H1–H3 were re-run against the CLI suite to record each case's red above. Native, Linux and Windows cross-clippy for blit-core and blit-transfer are clean.
- Line numbers are those of the fixed files: H4 adds three lines in front of V1, and the reported `filter.rs:1987` was shifted by that many. The other mutations keep the line count.

## Known gaps
- An any-depth name rule is matched case-insensitively (globset's Unicode simple case folding), not under the filesystem's own equivalence. A destination entry stored in another Unicode normalization than the source's name (NFD `café` at the destination for an NFC `--exclude café`) is still not named. APFS keeps the spelling blit wrote, so this needs a destination created by another tool in the other form. A per-entry probe would cost a lookup per destination entry, so it is out of reach (FAST).
- A path rule's components after its first glob component are not respelled, and a path rule that starts with a glob (`**/cache`, `*/cache`) is not respelled at all. Path rules are not compiled case-insensitively (the admitted shape scopes that to name rules). So `--exclude '**/cache'` against a destination `x/Cache` still leaves `x/Cache` in the deletion scope. The bare name `cache`, which means the same thing, is covered. Compiling path rules case-insensitively at a case-insensitive destination as well would close this. Not done; outside the admitted shape.
- The case rule is probed once at the root. A destination subtree with its own rule (a mount point below the root; Windows per-directory case sensitivity) is read with the root's rule.
- A respelling needs each leading component to exist at the destination; it stops at the first one that does not.
- The case-insensitive guards (the integration test, the delete-pass test, both CLI tests) run only where the filesystem ignores case: macOS here (none skipped). On Linux CI (case-sensitive ext4) they print a skip line and pass, so they can go red only on such a filesystem. On Windows CI, the delete-pass test, the integration test and the local CLI cases run; the daemon test is Unix-only like the existing push test. The pure respelling test and the case-probe test run everywhere.
- Verified on macOS aarch64 (APFS, case-insensitive) only, plus the Linux and Windows cross-clippy at the final gate; the Linux and Windows runtime tests were not run here.

## Reviewer comments
(filled by the re-review)
