# cr-ecm8-1: An exclude reported as unmatchable still went into the filter and to the peer

**Severity**: HIGH — a pattern the run had just said "cannot match anything" was sent to a daemon that read the same text as a pattern; a mirror then deleted a destination file the source had
**Status**: Verified — `exclude-case-r9` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..ab7331c3`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `9c244efe`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r8` over `3bc3ee47..f4203c31` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r8.codex.md`.

## Evidence
The reviewer exercised the Windows resolver on Linux with a remote source and `--ignore-case`. It classified `\keep` as unmatchable but sent it as an active glob. A real Linux source daemon interpreted it as `keep`. With `--delete-scope all`, the mirror deleted destination `KEEP` even though source `KEEP` existed. Asked for: keep unmatchable patterns for diagnostics and omit them from executable filters on both ends.

Cause: audit-19 reports an absolute `--exclude` that names nothing under the source — outside it, the source itself, or any absolute pattern when the source is not local — and kept it among the patterns "as typed, where it matches nothing". That holds for the text on the host that typed it, where it is an absolute path and no walked path begins that way. It does not hold for the same text read by another kind of host: under the Windows rule `\keep` is a path at a drive's root; to a Linux peer it is the glob `\k` `eep`, the name `keep`. The dialect every other pattern is rewritten into before it travels (cr-g3fix4-3) was never applied to these — they were "as typed" by design.

## Predicted observable failure
From Windows, any `--exclude` written as a rooted path (`\name`, `\dir\name`) with a source on a Linux or macOS daemon: the daemon excludes `name` (every case of it with `--ignore-case`), so the entry is not transferred, and on a mirror with `--delete-scope all` the destination's copy is deleted. The run meanwhile says the pattern cannot match anything.

## Intake
Admitted.

## What
A pattern reported as unable to match anything is in NO filter: `ResolvedExcludes.unmatchable` holds it for the warning, `ResolvedExcludes.patterns` does not hold it, and so neither the filter built on this host nor the `FilterSpec` a peer is sent does. For each exclude as typed, `became` is now an empty range for such a pattern.

It is still held to what every pattern is: text that reads as a glob. One that does not (`/nowhere/[z-a]`) refuses the command as before — in `validate`, `build` and `build_spec` alike (`refuse_unreadable`) — so nothing that was refused is now let through with a warning.

## Guard proof
- `transfers::filter::tests::an_exclude_that_cannot_match_is_in_no_filter`: the reviewer's case under the Windows rule on every OS — `\keep`, `*.tmp` and `C:\data\Cache` against a source that is not local resolve to one pattern and two reported; the filter and the spec a command builds from that resolution hold `*.tmp` alone; the filter a peer builds from that spec excludes neither `keep`, `KEEP` nor `data/Cache`; and (off Windows) the text that used to be sent, read where it arrived with the case mode, does exclude `KEEP` — the fault itself. Under this host's own rule, a pattern outside the source and the source's own path are in neither `build` nor `build_spec`. And an unmatchable pattern that is no glob is refused by all three entry points.
- The older guards now pin the same thing where they pinned the opposite: `an_absolute_exclude_outside_the_source_root_is_unmatchable`, `relative_excludes_take_forward_slashes_under_the_windows_rule`, `the_case_note_is_derived_from_the_effective_filter`, `a_filter_is_built_from_the_resolution_it_is_handed`.
- Mutations, each restored byte for byte: what cannot match kept among the patterns as typed → the new guard and four older ones red; what cannot match no longer read as a glob → the new guard red.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — `resolve_excludes_on`, `ResolvedExcludes`' account of itself, `resolved_patterns`, `refuse_unreadable`, `build`, `build_spec`, `validate`; the guards.
- `CHANGELOG.md` — the fix.

## Known gaps
- **The reviewer's route was not run here**: a Windows client against a Linux daemon. The Windows resolver's answer is guarded on every OS (its string rule takes the platform as an argument); what the daemon did with the old text is shown at the filter a peer builds, on Linux. No Windows host ran anything.
- Between two Linux hosts nothing observable changes: an absolute pattern as typed begins with `/`, which no walked path does.
- No contract number changes: a pattern that is not sent is refused by no peer, and an older peer handed the shorter list applies exactly what it is given.
- Run on Linux only.
