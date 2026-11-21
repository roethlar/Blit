# cr-ecm14-1: a name a retry did not find was answered for by another of its text

**Severity**: HIGH — a failed file that has gone from the source is not reported, and the retry exits 0 without it
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm14-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-ecm13-3 (`6f0479dc`, `b2b77b78`).

## Evidence
"A job recorded two failures, one named `dup` plus byte `FF`, the other `dup` plus U+FFFD. After the first disappeared, `jobs retry` copied the second and exited 0. The missing-path check discarded the raw bytes when consulting the sent manifest."

Cause. cr-ecm13-3 carried a name's bytes into what a retry ASKS for and into what it REPORTS missing. Between the two, `requested_but_unscanned` decided what had been found by looking each requested path up in the source's map of the manifest — which is keyed by TEXT, one entry a text, the first to arrive kept. A name that is not text was looked up by the text it is shown as; `dup` and U+FFFD, a different file, was in the map under exactly that text; so the name that had gone was taken as found.

## Intake
Admitted as the reviewer put it. It holds the other way round too — the name that IS text gone, the one that is not found — which the same lookup got wrong the same way.

## What
Under a list (a retry's, a `--files-from`), the source keeps every name its walk puts on the manifest BY ITS OWN BYTES (`own_name`: the exact bytes of a name that is not text, its text's otherwise) in a set beside the map of texts, and `requested_but_unscanned` looks what was asked for up there, by its own bytes. Two names of one text are two entries. The map of texts is as it was: it is what the end that writes is answered from, one entry a text. No list in force, nothing is kept.

## Guard proof
- `transfer_session::…::a_name_is_not_found_because_another_of_its_text_was`: both names asked for; found the one that is text — the other is reported missing with its bytes; found the one that is not — the text one is reported; both found, neither; none found, both. And a manifest entry's own name.
- `crates/blit-cli/tests/retry_pass.rs::a_name_that_has_gone_is_missing_though_another_of_its_text_was_found` (end to end, the reviewer's case as an automatic retry): both names fail in the first pass — a file stands where their directory is to be — and during the wait the place is freed and the name that is not text removed from the source. The retry lands the one that is there, says the other was missing at retry, and exits 2.
- The four guards that hand `requested_but_unscanned` a manifest hold unchanged, given the set of names in its place.
- Mutations, each restored byte for byte: what was asked for looked up by its text (the unit guard and the end-to-end one red); a name the walk found kept by its text (`two_names_of_one_text_both_arrive` and `a_failed_name_that_is_not_text_is_retried_as_itself` red — with both names there, one then answers for the other again); a manifest entry's own name taken as its text (the unit guard and those two red).

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `own_name`; the set of names found under a list; `requested_but_unscanned` asks it; guards.
- `crates/blit-cli/tests/retry_pass.rs` — guard.

## Known gaps
- The set is one more copy of each listed name the walk finds, for the length of the scan, on the source end. A list is a retry's or a `--files-from`: as many names as were asked for, not a tree's.
- Unix: a name that is not text exists only there. On Windows a name that is not valid Unicode is carried by another road (WTF-8 on the same machine), and nothing ran there.
- The end-to-end guard returns early on a filesystem that refuses a name that is not text (APFS).
