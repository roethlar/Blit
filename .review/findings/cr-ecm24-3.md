# cr-ecm24-3: a destination that is one file was refused its own replacement

**Severity**: HIGH — a resumed pull to an exact file path (a `0600` source over an existing `0644` target) exited 2; it succeeded before cr-ecm23-3, which brought this in
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm24-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm23-3 (`6bf9c2f9`).

## Evidence
"A resumed loopback pull from a `0600` source to an existing `0644` target succeeded at base and exited 2 at head. The canonical root is the file itself, so testing whether its parent starts with that root rejects the requested target."

## Intake
Admitted. "Represent an exact-file destination by its verified parent and leaf": the chain's end is its resolved directory and its name, and the end lies within the destination when its directory does, or when directory and name together are the destination itself — the case of a destination that is one file.

## What
`fresh_partial_ready`: a chain's end whose directory lies outside the destination's root is refused unless the end — that directory joined with its name — is the root itself.

## Guard proof
- `resume::tests::a_destination_that_is_one_file_takes_a_fresh_partial` (Unix): with the root the file itself, a fresh partial takes the file's place; a name beside it is refused and left as it was.
- `sink::tests::a_resume_into_a_destination_that_is_one_file_completes` (Unix): a sink whose destination is one file holds it whole and completes it.
- Mutation, restored byte for byte: the end judged by its directory alone (red, both).
- The resume, sink and session tests (305) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial_ready`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — guard.

## Known gaps
- The reviewer's loopback pull is not repeated end to end; the sink test stands for it.
