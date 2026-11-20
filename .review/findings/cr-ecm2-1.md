# cr-ecm2-1: Under `--ignore-case` a bracket class matched letters of two and three bytes

**Severity**: HIGH — `mirror --ignore-case --include '[sk].txt'` deleted destination-only `ſ.txt` and the Kelvin-sign `K.txt`; the same command as typed kept both
**Status**: Verified — `exclude-case-r3` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..467773b4`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `bbc50303`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-slice1-r2` over `3bc3ee47..a1fd6bae` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-slice1-r2.codex.md`.

## Evidence
The reviewer ran, against the built head: `mirror --yes --ignore-case --include '[sk].txt'` over a destination holding `ſ.txt` and `K.txt` (U+212A) that the source lacks. Both were deleted. Without the option both were kept. `ſ` is two bytes and the Kelvin sign three.

Cause, two places:
- `crates/blit-core/src/case_fold.rs` `widen_class` wrote a positive class as itself, its ASCII partners, AND each letter's fold-mates beyond ASCII as alternatives: `[sk]` became `{[sk],[SK],ſ,K}`. The plan says otherwise in two places — "`?` matches one byte and a class matches one byte, in both modes" (Q10) and "an ASCII class gains its letters' other case" (Design §Matching). Slice 1 went past its plan.
- `crates/blit-core/src/transfers/filter.rs` `DestinationNames` reads a one-member class as its character (`plain_name`: that is how escaped syntax such as `[*]` is written) and then widened the resulting NAME as bare letters, so the destination read `[s]aved` as `{S,s,ſ}aved` whatever the class widening did.

## Predicted observable failure
With `--ignore-case`, a class holding `s` or `k` (alone, among others, or in a range such as `[a-z]`) names entries whose letter is `ſ` or the Kelvin sign. For an include that puts them in a mirror's deletion scope; for an exclude the destination keeps what the source walk copies, the two ends reading one pattern two ways.

## Intake
Admitted. Slice 1's own record described the wider form as intended; it contradicted the plan it was landing.

## What
A class gains the other case of its ASCII letters and nothing else, with the option as without it. `widen_class` no longer adds anything beyond ASCII: `[sk]` is `{[sk],[SK]}`, one byte either way. At the destination, a component that puts an ASCII letter in a class is a glob under `--ignore-case` (`plain_name_in`), matched as written by the same class-aware widening the source walk uses, and no longer a name widened as bare letters. A bare letter is unchanged: `s` still names `ſ` when case is ignored (AC-2). Escaped syntax in a one-member class (`[*]`) is still a plain name.

## Guard proof
- `case_fold::tests::a_class_stays_one_byte_wide_when_case_is_ignored`: eight classes — single, several, ranges, upper case — name neither `ſ.txt` nor the Kelvin-sign `.txt` in either mode, and the widened text holds no letter beyond ASCII; the ASCII partners are gained; a negated class matches no letter of several bytes; the bare letters do name their fold-mates.
- `case_fold::tests::a_class_gains_the_other_case_of_its_letters` (slice 1's guard, corrected): `[st]x` and `[j-l]x` no longer name `ſx` and the Kelvin `x`; `[A-z]` gains nothing.
- `transfers::filter::tests::the_deletion_scope_reads_a_letter_class_as_one_byte`: for a name rule and two path rules holding `[s]`, the destination scope and the source filter agree entry for entry — `saved`, `Saved`, `SAVED` named, `ſaved` not; the bare `saved` names `ſaved` at both ends.
- `ignore_case::mirror_ignore_case_keeps_a_class_one_byte_wide`: the reviewer's scenario through the binary, for the include (both entries kept in both modes; the ASCII partner `sub/S.txt` deleted only when case is ignored) and for an exclude `[s]aved` (a source `ſaved` is copied and a destination-only one deleted in both modes; `Saved` kept only when case is ignored).
- Mutations, each restored byte for byte: the class gains `ſ` and the Kelvin sign again → all four guards red; the destination reads a letter in a class as a bare letter again → the deletion-scope guard and the end-to-end guard red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `widen_class`; two guards.
- `crates/blit-core/src/transfers/filter.rs` — `plain_name_in`, `class_holds_a_letter`, four call sites; one guard.
- `crates/blit-core/src/fs_enum.rs` — a guard's comment.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- Run on Linux only. The end-to-end guard keeps `ſ` and `S` names in separate folders so a filesystem that pairs them holds both; it has not run on one.
- Under `--ignore-case`, an exclude component with a letter in a class is no longer resolved to the destination's stored spelling (it is a glob, read as written like any other glob).
