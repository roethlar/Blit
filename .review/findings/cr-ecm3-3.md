# cr-ecm3-3: An `--include` went onto the wire as typed

**Severity**: HIGH — across platforms the peer read another pattern: the file meant fell out of the transfer, and under `mirror` an unrelated destination-only file was in the deletion scope
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm3-3: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
A Windows include `docs\report.txt` names a file in a folder there. The wire kept the backslash. The reviewer verified that a Linux receiver, with `--ignore-case`, rejected `DOCS/REPORT.TXT` and accepted `DOCSREPORT.TXT`: off Windows a `\` is an escape, so the pattern read `docsreport.txt`.

Cause: `build_spec` (`crates/blit-core/src/transfers/filter.rs`) sent `--include` patterns exactly as typed. `--exclude` patterns have been written in a dialect every platform reads alike since cr-g3fix4-3, which recorded the include half as a known gap on the reasoning that "`--include` takes no part in the deletion scope". That reasoning was wrong: an include defines the deletion scope of a filtered mirror (what it names is what may be deleted), as this plan's own guards show.

## Predicted observable failure
Between a Windows machine and a Unix one, in either direction, an include holding a `\` is read differently at the two ends: files are left out of a copy in silence, and a `mirror` may delete a destination-only file the pattern was never meant to name. Older than this plan; `--ignore-case` widens what the misread pattern names.

## Intake
Admitted, though the pattern handling predates this plan: the reviewer shows a deletion, and the recorded reason for leaving it was false.

## What
An include goes to a peer in the same dialect as an exclude (`wire_includes`: under the Windows rule `\` outside a class is written `/`; under the Unix rule an escape is written as a one-character class, or bare). A filter built for this machine's own use keeps the include as typed — this machine reads it by its own rule.

## Guard proof
- `transfers::filter::tests::an_include_goes_to_a_peer_in_the_dialect_every_platform_reads`: the dialect under each platform's rule (six rows); what a peer builds from the Windows-typed include names `docs/report.txt` and never `docsreport.txt`, and under `ignore_case` also `DOCS/REPORT.TXT` and never `DOCSREPORT.TXT`; `build_spec` applies it under this host's rule and `build` does not.
- `transfers::filter::tests::a_typed_unix_escape_names_the_same_path_under_either_rule` (cr-g3fix4-3's guard) asserted the old behaviour ("--include is passed as typed"); it now asserts the include is written for the peer.
- Mutations, each restored byte for byte: the dialect not applied → the guard red; `build_spec` sending includes as typed → the guard red.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — `wire_includes`, `build_spec`, the two guards.

## Known gaps
- No Windows machine took part: the Windows rule is exercised as a parameter on Linux, as in cr-g3fix4-3, and the receiving end here is this host's.
- Run on Linux only.
