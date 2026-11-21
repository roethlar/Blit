# cr-ecm14-4: a record from before `destination_nested` was replayed by reading its words again

**Severity**: HIGH — the retry of a run recorded by an earlier build writes the files that failed somewhere else, and exits 0
**Status**: Reviewed, found incomplete — `combined-r16` (Codex openreview, 2026-11-21) reviewed `35adeacb..13faafd2` — utilities slices 5 and 6 and the eleven `combined-r15` fixes — with `guard_confirmed: true`. It refuses where the two readings give one destination: "For `copy source/child/.. output`, I verified that both an absent and an existing `output` received the file directly. The legacy retry still refused." The remaining work is cr-ecm15-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `62b12719`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-ecm13-2 (`bf4987de`), whose design the review keeps ("persisted retry destination").

## Evidence
"I replayed an old-format record for `copy src backup`, where the first run created `backup`. The retry wrote `backup/src/big`, left `backup/big` missing, and exited 0. Recover the original destination from durable evidence or report the ambiguity before writing."

Cause. cr-ecm13-2 has a run keep, in its record, whether it put the source under the destination by its name (`RunRecord.destination_nested`), and has the retry use that. A record written before the field has none, and the retry fell back to what it had always done: read the two words again and ask the disk — which the first run had changed, by making `backup`.

## Intake
Admitted. Of the reviewer's two remedies, the second: **the ambiguity is reported, and nothing is written.** The first — recover the destination from durable evidence — was looked at and not taken. What a record of that age holds is its two words and its failures; what is on the disk is one of two trees, `backup/…` or `backup/src/…`, and neither a directory called `src` under `backup` (which a source holding a `src` of its own also makes) nor its absence (a run whose every file failed makes nothing) says which. Evidence that settles some cases and guesses the rest is what this finding is about.

## What
- `blit_core::transfers::resolution::nesting_asks_the_disk` says whether two words' answer turns on what stands at the destination: a source that is not its contents (no `/` after it), to a destination on this machine with no separator after it. Only then can the same words give two answers.
- `blit jobs retry` of a record that holds no `destination_nested`: where the words ask the disk, it is refused before anything is read or written, saying that the record is of an earlier build, what the two readings are, and how to spell the destination to say which. Where the words alone decide — a `/` after the source, or after the destination, or a destination on a daemon — they decide now as they did then, and the retry runs as before.
- A record that holds the field is untouched by this.

## Guard proof
- `transfers::resolution::tests::nesting_asks_the_disk_only_where_its_answer_can_change`: ten spellings, each shown BOTH ways — what the function says, and whether `destination_nests` in fact answers differently with a directory there and without; and a destination on a daemon, which is read by its spelling alone.
- `crates/blit-cli/tests/job_retry.rs::a_record_that_did_not_note_where_it_wrote_is_not_guessed_at` (end to end), the four spellings of `copy src backup` with `backup` there and one file blocked: each run's record has the field taken out, as an earlier build would have written it, and the block removed. With no `/` after either, `jobs retry` fails, says `did not note where it wrote` and `nothing is retried`, and the destination's tree is entry for entry what it was; with a `/` after the source, the destination, or both, the retry lands the file where it failed.
- `a_retry_writes_where_the_run_it_retries_wrote` (cr-ecm13-2) holds unchanged: a record with the field is retried by it.
- Mutations, each restored byte for byte: the record replayed by its words (the end-to-end guard red); no two words taken to ask the disk; a source that is its contents taken to ask; a destination typed as a container taken to ask — four, each red on the guards named.

## Files changed
- `crates/blit-core/src/transfers/resolution.rs` — `nesting_asks_the_disk`; guard.
- `crates/blit-cli/src/jobs.rs` — the refusal.
- `crates/blit-cli/tests/job_retry.rs` — guard.

## Known gaps
- **Such a job cannot be retried at all**; it has to be run again, with its destination spelled out. Its failures are still listed (`blit jobs show`). The records concerned are those written by builds of this development branch between the day job records began (2026-10) and cr-ecm13-2 (2026-11-21): no release has written one.
- A SAVED job (`blit jobs run`) is not a retry and reads its words afresh each time it runs, as a command typed again does; only `jobs retry` is refused.
- A retry of a retry inherits its parent's answer only if the parent had one: a chain begun by an old record is refused at every link.
- The refusal names the two readings and does not look at the disk to say which it would have guessed.
