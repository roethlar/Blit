# cr-ecm1-2: A pattern that no longer compiles once widened drops every exclude to its text

**Severity**: HIGH — under `mirror --ignore-case`, destination entries the excludes name are deleted; the option names LESS than not giving it
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm1-2: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, review `exclude-slice1-r1` over `3bc3ee47..66d1338a`. That run FAILED before a verdict (provider 503); this finding is read from its event stream (`.review/results/exclude-slice1-r1.codex.events.jsonl`, event 147: the reviewer's probe and its output).

## Evidence
The reviewer's probe, run against the built head: a destination holding `s`, `SAVED.CFG` and `saved.cfg`, none of them in the source, and `mirror --yes <mode> --exclude '*.cfg' --exclude '{{{…{s}…}}}'` (one `s` inside 247 alternations).
- `--match-case`: `s` and `saved.cfg` remain; `SAVED.CFG` is deleted. Correct: both patterns work, exactly.
- `--ignore-case`: only `saved.cfg` remains. `s` and `SAVED.CFG` are both deleted.

Cause: slice 1 validated a widened pattern by PARSING it (`globset::Glob::new`). Widening writes the letter `s` as `{S,s,ſ}`, one alternation deeper; the pattern still parses, but the set can no longer be BUILT. `CompiledPatterns::compile` (`crates/blit-core/src/fs_enum.rs`) answers a set that cannot be built by comparing every pattern as text exactly as written — so `*.cfg` stopped ignoring case, and the nested pattern named nothing.

## Predicted observable failure
With `--ignore-case`, an exclude set holding one pattern that compiles as typed and not once widened protects nothing in another case and nothing the failing pattern named. Under `mirror` those entries are deleted, exit 0.

## Intake
Admitted. The fallback to text is older than slice 1 (it also answers a set that cannot be built as typed), but slice 1 made it reachable from a pattern that validation had accepted, and under an option whose whole purpose is to name MORE.

## What
A filter is validated by compiling it as it will be used, not by parsing it. `FileFilter::validate_globs` now builds each pattern alone into a matcher — widened when the filter ignores case — so the refusal names the pattern, and then builds the sets exactly as the filter will (`CompiledPatterns::build` for the whole-path globs and the includes, `ExcludeRules::build_error` for the name and subtree rules). Anything that does not build refuses the run: `--exclude pattern "…" cannot be compiled with --ignore-case: …`. Every caller that validates — `filter::build`, `filter::build_spec`, and so the CLI and `check` — refuses before a walk starts. The text fallback stays for a filter an embedder builds without validating; no validated filter reaches it.

## Guard proof
- `fs_enum::tests::a_pattern_that_cannot_be_compiled_as_it_will_be_read_is_refused`: finds the deepest nesting that compiles as typed, shows it validates and excludes as typed, and is refused widened, naming the pattern at fault and the option; the same for an include; one level deeper is refused as typed with no mode named.
- `ignore_case::a_pattern_that_cannot_be_compiled_widened_is_refused_before_anything_is_deleted`: the reviewer's scenario through the binary. The depth is found by asking the binary. `--match-case` leaves `s` and `saved.cfg`; `--ignore-case` exits non-zero with the refusal, and `s`, `SAVED.CFG` and `saved.cfg` are all still there and nothing was copied.
- Mutation (validation that only parses: the per-pattern build returns nothing and the set builds are skipped): both guards red; restored byte for byte, both green.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `read_pattern`, `compile_error`; `CompiledPatterns::build` beside `compile`; `ExcludeRules::bodies` and `build_error`; `FileFilter::validate_globs` compiles; the unit guard.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- Run on Linux only.
- The daemon's own check of a wire spec (`filter_from_spec`) still only parses; it carries no case mode until slice 3, which must give it this same validation.
