# cr-ecm4-3: The line about capital letters was not said of the filter the run uses

**Severity**: HIGH — the line showed, as something `--ignore-case` "would also match", a spelling the option does not match; an operator who acted on it had a mirror delete the file
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm4-3: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r4` over `3bc3ee47..0669279f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r4.codex.md`.

## Evidence
For `photo[!x].jpg` the prompt said `--ignore-case` would also match `PHOTO[!x].JPG`. Read as the glob it is, that admits `PHOTOX.JPG` — which the mirror deleted — and `photoX.jpg` lost the protection it had as typed. The reviewer also saw one outside-root pattern described as unable to match anything and, in the next line, as matching.

Cause, three parts of one thing — the line was built from the patterns as typed, not from what the matcher is given:
1. `--ignore-case` gives a negated class the other case of its letters too (`[!x]` is read `[!xX]`), so there the option matches LESS. `Spellings::of` (`crates/blit-core/src/case_fold.rs`) copied such a class into a spelling beside letters in their other case; with no other letter in the pattern it already said "matches exactly as typed … reads it otherwise" (cr-ecm3-5), which is how this case was missed.
2. The CLI passed every typed exclude to the line, the ones `resolve_excludes` had just reported unmatchable included.
3. Found while fixing, same cause: an absolute exclude under the source was shown with the SOURCE's letters in another case (`/srv/Data` → "would also match /srv/data, /SRV/DATA"). The option never reads the source's own path that way; under `/srv` there is no `/SRV/DATA`.

## Predicted observable failure
- `mirror --exclude 'photo[!x].jpg'` (any pattern with letters and a negated class of one case): the question shows a spelling that admits names `--ignore-case` deletes.
- An absolute exclude outside the source: "cannot match anything", then "matches … only".

## Intake
Admitted, all of it.

## What
- The line is derived from the effective patterns: `filter::case_note(exclude, include, resolved)` takes what `resolve_excludes` made of the same excludes (`ResolvedExcludes.became` now says, per typed exclude, which patterns it became). An unmatchable exclude is neither shown nor counted. An absolute exclude is echoed as typed and described as the pattern below the source: `"/srv/Data" matches Data only … (--ignore-case would also match data, DATA)`.
- A pattern in which the option would make a negated class take less gets no other spelling at all, and the line says what the class would stop matching: `"photo[!x].jpg" matches exactly as typed; capital letters matter (--ignore-case changes what it matches: [!x] would stop matching X).` The letters named are only those the class does not refuse already (`[!a-cA]` → `B-C`).
- The two older entry points (`filter::case_note(&[String], &[String])`, `filter::case_note_from`) are gone: neither could know what the run had resolved.

The matching itself is unchanged: `[!x]` with `--ignore-case` refuses both cases, as case-insensitive matching does everywhere (slice 1).

## Guard proof
- `case_fold::tests::the_case_note_says_what_a_negated_class_would_stop_matching`: the reviewer's pattern to the letter, with the matcher's own answers beside it; five patterns with the class and letters named; negated classes the option leaves alone still get a true spelling; and for nine patterns every spelling shown, READ AS A GLOB over a pool of characters, admits only names the option matches.
- `case_fold::tests::the_case_note_describes_the_pattern_as_the_filter_reads_it`; `transfers::filter::tests::the_case_note_is_derived_from_the_effective_filter` (an unmatchable exclude passed over, alone and beside others; an absolute one described as what it became; a source that is not local; `became` per typed exclude).
- `case_note::the_line_is_said_of_the_filter_the_run_uses`, through the binary: the line for `photo[!x].jpg`, then both mirrors — as typed `photoX.jpg` is kept and `PHOTOY.JPG` deleted, with the option the reverse, as the line says; an outside exclude reported once and not spoken of, alone and beside `*Cache`; an absolute exclude under the source.
- Mutations, each restored byte for byte: a narrowed class shown in a spelling again → three guards red; letters already refused named as lost → red; an unmatchable exclude spoken of → two red; an absolute exclude described as typed → two red.
- `the_case_note_follows_what_the_matcher_does` (cr-ecm3-5) asserted the old words for `*[!a]`; it now asserts the new ones.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Noted`, `case_note_of`, `Spellings.narrowed`, `narrowed_class`, `other_cases`; guards.
- `crates/blit-core/src/transfers/filter.rs` — `case_note`, `ResolvedExcludes.became`; guard.
- `crates/blit-cli/src/transfers/mod.rs` — the excludes resolved once, for the warnings and the line.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- Only the FIRST narrowed class of the pattern shown is named; a second one in the same pattern, and any in the patterns that are only counted, are covered by "changes what it matches" and not spelled out.
- A class across both cases that the option still widens (`[A-c]1`) keeps the older words, "reads it otherwise": nothing is lost there, and no spelling shows what is gained.
- An absolute exclude naming an entry stored under several names is described by the first of the patterns it became.
- The Windows path rule for an absolute exclude is exercised here only through this host's rule; the end-to-end guard runs on every OS but was run on Linux only.
