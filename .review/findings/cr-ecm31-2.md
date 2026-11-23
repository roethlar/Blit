# cr-ecm31-2: the comparison's look followed links, and a file simply gone was taken for a conflict

**Severity**: HIGH — the wire's comparison looked at a destination through its links. A link to nothing, put at a name found empty, passed the hold's check for nothing, and the partial was made where the link pointed: another name in the destination. And a file found, then simply gone, was taken for a conflict, so the copy of its replacement failed.
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm31-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It completes cr-ecm30-1 (`125231cb`).

## Evidence
"The wire comparison follows links. A dangling link introduced after an observation of absence can therefore pass the receiver's absence check and redirect creation to another in-root name. Conversely, I verified that removing the previously observed file and leaving the name vacant produces a conflict. Preserve the leaf observation across comparison and acquisition, and allow exclusive creation when the original file has simply gone."

## Intake
Admitted as the reviewer put it.

## What
- `ResumeBaseline::WholeOf { leaf, end }`.
  - `leaf` is the entry at the name itself, looked at without following a link (`symlink_metadata`).
  - `end` is the file the name leads to, found by following the chain, as before.
  - Each is a version, or nothing.
  - The hashing reads the leaf first. If that look fails other than as `NotFound`, the file is sent `Whole`, observation unknown.
- `hold_whole` asks the name itself first:
  - nothing found, and something there now (a link to nothing among them) → left as it is, and the file fails;
  - something found and something there now → it must be the same version.
- It then asks the end, as before.
- At either place, nothing there now — never anything, or what was found and has simply gone — leaves the name to the partial. The partial is made only while nothing has the name.

## Guard proof
- `sink::tests::the_wires_whole_resume_tells_an_empty_name_from_a_link_to_nothing` (Unix):
  - Nothing found, then a link to nothing (`elsewhere.bin`, in the destination) put at the name: the link is left, nothing is made where it points, and the file fails.
  - A file found, then removed: its partial is made at the empty name, unfinished (11 bytes for 10).
- `transfer_session::tests::what_decided_a_whole_resume_goes_with_it` (Unix) has a new case: a link to nothing is sent with the link itself as `leaf` and nothing as `end`.
- Mutations, each restored byte for byte:
  - the name itself not asked → red;
  - a file simply gone taken for a conflict → red;
  - the comparison looking at the name through its link → red.
- The resume, sink, session, copy and data-plane tests (359) pass. Windows and macOS (`CARGO_FEATURE_PURE=1`) cross-clippy are clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `ResumeBaseline::WholeOf`, `hold_partial`, `hold_whole`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.

## Known gaps
- The name's look and the chain's resolution are separate calls. A change between them goes unseen, as D-2026-11-23-1 accepts of the look and the removal.
