# cr-ecm6-1: An include was read differently here and at a peer

**Severity**: HIGH — the same mirror put a different destination-only file in its deletion scope depending on whether the destination was local or behind a daemon
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm6-1: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r6` over `3bc3ee47..a13238b6` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r6.codex.md`.

## Evidence
With the Unix pattern `--include '{a\,**/x,b}'`, the local filter admitted `a/x` and rejected `a,/x`. The decoded wire filter did the reverse in both case modes. A remote mirror can therefore delete a destination-only `a,/x` that a local mirror keeps.

Reproduced before any change (a probe over nine patterns and seventeen paths; only this one differed). Cause: cr-ecm3-3 made `build_spec` send an include in the dialect every platform reads alike (an escape `\c` written as the class `[c]`), and left `build` — the filter this machine uses for itself — holding the include as typed, on the reasoning that this machine reads its own text. The glob library reads typed text by its raw characters: inside braces, a `**` that follows a `,` or `{` is taken for the start of an alternative, escaped or not, and the token before it is replaced. In `{a\,**/x,b}` that drops the comma. Handed `{a[,]**/x,b}`, in which the comma is plainly part of the name, the peer read what was typed.

## Predicted observable failure
`blit mirror --include '{a\,**/x,b}' SRC DST`: between two local folders the scope is `a/…/x` and `b`; through a daemon it is `a,…/x` and `b`. Each deletes a destination-only file the other keeps. Unix only — under the Windows rule a `\` is a separator.

## Intake
Admitted. The peer's reading is the one typed; the local one was the library's quirk.

## What
One representation: a filter built for this machine's own use (`filter::build`) and the early check (`filter::validate`) hold an include in the same form `build_spec` sends, so the two ends are built from one text and cannot disagree. Excludes have been so since cr-g3fix4-3 (both go through `resolve_excludes`).

## Guard proof
- `transfers::filter::tests::an_include_means_the_same_here_and_at_a_peer` (every OS): for twenty-one patterns in both case modes, the early check and the built filter agree on whether the pattern is usable; the local filter and the wire spec hold the same text; and the local filter and the filter a peer builds from the spec give the same answer for each of twenty-five paths. Under the rule a `\` escapes by, `{a\,**/x,b}` admits `a,/x` and `b` and refuses `a/x`.
- `ignore_case::an_include_reads_the_same_locally_and_through_a_daemon` (Unix), through the binary: `mirror --include '{a\,**/x,b}'` between two local folders, pushed to a real daemon and pulled from one — on each route `b` is copied, the destination-only `a,/x` deleted, and `a/x` and an unrelated file kept.
- `an_include_goes_to_a_peer_in_the_dialect_every_platform_reads` (cr-ecm3-3) asserted that a local filter keeps the include as typed; it now asserts the one form.
- Mutation (a local filter keeping the include as typed): all three red; restored byte for byte.
- The early check's half has no guard that can go red: every pattern of up to five characters over eleven symbols (`a \ [ ] { } , * / ! -`, about 177,000) was judged usable or not alike in the two forms. It reads the same form so that what is checked is what is built.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — `build`, `validate`; the guards.
- `crates/blit-cli/tests/ignore_case.rs` — the end-to-end guard.

## Known gaps
- What a pattern means is still the glob library's reading of the dialect form; the guard is that both ends share it, and one case where it is the reading typed.
- The line about capital letters is built from the include as typed; it shows the same letters either way.
- The end-to-end guard cannot run under the Windows rule; run on Linux only.
