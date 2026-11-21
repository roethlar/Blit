# cr-ecm10-3: A single-file source's one name did not reach the in-run retry

**Severity**: HIGH — a file that could be copied on the retry was reported `missing at retry`, and the command exited 2 with nothing copied
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm10-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r10` over `df682eea..41b4bfdd` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r10.codex.md`. Completes cr-ecm9-3, which that review found incomplete.

## Evidence
The reviewer blocked `out/Report.txt` with a directory, ran a listed single-file copy, and removed the directory during the retry wait. The head exited 2 with `source: missing at retry` and left the file uncopied. The retry passed the empty wire path into list validation, which classified it as unnamed. Translate that internal root entry back to the source basename before building the retry filter.

Reproduced here before the fix, and WITHOUT `--files-from` as well: a retry pass limits its session to the failed paths through the same list a `--files-from` gives.

```
Up to date: 1 files examined, 0 changed (in 4.81s)
1 file(s) did not land at the destination:
  <the transfer root file>: source: missing at retry (not found by the retry scan) (retried)
```

Cause. A source that is one file has one entry, known to the transfer by the empty relative path; that is what its failure is recorded under, and so what the next pass is asked to retry. Since cr-ecm7-1 a list holds NAMES, and a line that is no name — the empty path among them — is kept apart as one that names nothing; since then the retry of a single-file source has sent nothing. (cr-ecm9-3 made the walk and the account know such a file by its name; the retry was the third place.)

## Predicted observable failure
Any single-file `copy` or `mirror` whose one file fails the main pass — a destination briefly blocked, a file briefly locked — is never retried: every pass reports it missing, and the command ends with exit 2 although the file and the destination are both fine by then. `blit jobs retry` of such a run likewise.

## Intake
Admitted as the reviewer put it.

## What
`transfers::retry::named_for_a_list`: where a pass's paths are handed to the list — the three places in the retry loop, and `jobs retry` — the empty path is replaced by the source's last name as typed (a local path's, or a remote operand's). Every other path goes as it is; with no last name to go by the path stands.

## Guard proof
- `retry_pass.rs::a_single_file_freed_during_the_wait_lands_on_the_retry_pass`, through the binary, with the file named in `--files-from` and without: `out/Report.txt` blocked by a directory, freed during the wait — exit 0, the file landed with its content, no failure reported, one retry pass announced and counted. Red before the fix (the output above).
- `transfers::retry::tests::the_one_file_of_a_single_file_source_is_retried_by_its_name`: five sources (absolute, relative, `./`, a module path, the default root) with the name each gives; a tree's paths untouched; a source with no last name, and a bare word (a daemon to blit), left as they are.
- Mutations, each restored byte for byte: the empty path handed to the list as it stands → the end-to-end guard red; the translation leaving the empty path → both.

## Files changed
- `crates/blit-cli/src/transfers/retry.rs` — `named_for_a_list`, `source_name`; the unit guard.
- `crates/blit-cli/src/jobs.rs` — `jobs retry` uses it.
- `crates/blit-cli/tests/retry_pass.rs` — the end-to-end guard.
- `CHANGELOG.md`.

## Known gaps
- `jobs retry` of a single-file run is covered by the function it shares with the in-run retry, not by an end-to-end guard of its own.
- A single-file source on a daemon (a pull) takes the name from the operand as typed; the end-to-end guard is local to local.
- A retry of a single-file source whose file really is gone reports it under its name, where the main pass's failure was recorded under the empty path. Not examined further here.
- Run on Linux only.
