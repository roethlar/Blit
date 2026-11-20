# cr-ecm2-2: A name that is not valid UTF-8 was matched through a lossy copy

**Severity**: HIGH — a mirror deleted a destination-only file from under an exclude that names it
**Status**: Verified — `exclude-case-r3` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..467773b4`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ca96e5cb`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-slice1-r2` over `3bc3ee47..a1fd6bae` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-slice1-r2.codex.md`.

## Evidence
The reviewer ran `mirror --ignore-case --exclude '?.txt'` over a destination holding `X.TXT` and a file named by the single byte `0xFF` followed by `.TXT`, neither in the source. `X.TXT` was kept; the other was deleted. The reviewer confirmed the same at the base in exact mode: the defect is older than this plan.

Cause: every place a pattern met a name first turned the name into text with `to_string_lossy`, which writes each byte that is not valid UTF-8 as the three bytes of U+FFFD. `?` and a class match one byte (Q10), so the one-byte name was three bytes to the matcher: `???.txt` named it and `?.txt` did not. Four places: `FileFilter::allows_entry` and the directory pruning beside it, `ExcludeRules::excludes` (`crates/blit-core/src/fs_enum.rs`), the deletion scope's own steps (`DestinationNames::protects`, `crates/blit-core/src/transfers/filter.rs`), and the filter over scanned headers (`filter_headers`, `crates/blit-core/src/remote/transfer/source.rs`), which read the header's lossy text though the scan carries the exact bytes beside it.

## Predicted observable failure
On Linux and the BSDs, a file whose name holds a byte that is not valid UTF-8 is not named by a pattern that counts its bytes with `?` or a class, and is named by one that counts three per bad byte. An exclude meant to protect it does not; under `mirror` it is deleted.

## Intake
Admitted, though it predates the change under review: the plan's rule — `?` matches one byte, as rsync and git do — was not true of such names, and the consequence is a deletion.

## What
A pattern is matched against a name's own bytes. `match_bytes` (`fs_enum.rs`) gives them: on Unix the name's bytes exactly, valid UTF-8 or not; on Windows, where a name is UTF-16 and has no bytes of its own, its text with ill-formed UTF-16 replaced — what the glob engine itself does with a Windows path, so nothing changes there. Every matcher takes those bytes: the whole-path glob and the includes, the any-depth name rule and the subtree rule, directory pruning, the deletion scope's literal comparison, glob steps, crossing globs and bounded automaton, and the header filter, which now builds the path from the header's raw bytes when the scan sent them.

## Guard proof
- `fs_enum::tests::a_name_that_is_not_utf8_is_matched_by_its_own_bytes` (Unix): eighteen rows — `?`, a negated class, `*`, two bad bytes, the name rule at depth, the path rule, a directory only the name rule or the subtree rule can name; `é` still two bytes; the reviewer's case ignoring case; an include; directory pruning.
- `transfers::filter::tests::the_deletion_scope_matches_a_name_that_is_not_utf8_by_its_bytes` (Unix): the automaton (`one/?.txt`), a glob step without one (`one/[!a].txt`), a crossing glob (`one/**/?.txt`), a directory component.
- `remote::transfer::source::raw_name_filter_tests::a_header_is_filtered_by_its_names_own_bytes` (Unix): three excludes over four headers, two of them carrying raw bytes.
- `exclude_contract::mirror_exclude_matches_a_name_that_is_not_utf8_by_its_bytes` (Linux and the BSDs): the reviewer's scenario through the binary in both modes — the destination's file kept, the source's left out — and `???.TXT` naming neither.
- Mutations, each restored byte for byte: every match reads the lossy copy again → all four guards red; only the name and subtree rules do → the unit guard red (and the header and end-to-end guards); only the deletion scope's steps do → its guard red; a header filtered by its text → the header guard red (and the end-to-end one: a local mirror goes through it). The end-to-end guard going red here shows it ran and was not skipped.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `match_bytes`; `CompiledPatterns::is_match`, `ExcludeRules::excludes`, `excluded`, `allows_entry`, `should_include_dir` take bytes; the guard.
- `crates/blit-core/src/transfers/filter.rs` — `protects`, `step`, `advance` take bytes; the guard.
- `crates/blit-core/src/remote/transfer/source.rs` — `filter_headers` reads the raw path; the guard.
- `crates/blit-cli/tests/exclude_contract.rs` — the end-to-end guard.

## Known gaps
- Every guard is Unix-only by nature (a Windows name has no bytes of its own), and the end-to-end one needs a filesystem that stores such a name: it cannot run on macOS or Windows. On Windows the matched text is unchanged by construction, shown only by the cross-target build and the existing suite — not run there.
- `exclude_dirs` (set by no command-line option) still compares text and skips a component that is not UTF-8.
- A name that is valid UTF-8 is matched exactly as before; no measurement of speed was taken for this change (cr-ecm2-3 measures the filter's cost as a whole).
