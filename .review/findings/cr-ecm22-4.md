# cr-ecm22-4: a source of mode 0000, and the wire's two samples

**Severity**: HIGH — a source of mode 0000 counted as no mode, so a destination others can read kept its inode, and a reader holding it read the source's bytes
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm22-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-3 (`abe9d4a1`) and cr-ecm20-2 (`9ee5cb63`).

## Evidence
"A Linux acquisition probe showed that a mode-`0000` source reused the inode of a `0644` destination. If a privileged backup can read that source, an existing destination reader retains access to subsequent writes. Treat zero permission bits as a known mode. On the wire, use one metadata sample for reuse eligibility and the hash baseline so a permission change between the two reads cannot invalidate the decision."

## Intake
Admitted as the reviewer put it. On the wire a mode is unknown only for a Windows source, whose header carries its Windows metadata in place of a mode; every other source's mode is known, 0000 too.

## What
- Local (`open_resume_partial`): a source mode of 0000 is a mode — the most private — and a destination others can read takes a fresh partial for it.
- Wire (`compute_resume_block_hashes`): the source mode is `None` only for a Windows source; 0000 is asked like any other. And ONE look at the opened file decides reuse and is the baseline the hashes are of.

## Guard proof
- `resume::tests::a_source_of_no_mode_bits_is_more_private_than_its_destination`: a source opened, then made 0000, resumed into a 644 destination — another file at the name.
- `transfer_session::…::hashing_a_resume_partial_changes_nothing` (cases added): for a source of mode 0000 no hash, `Whole`; for one with no mode, the hashes; the file unchanged throughout.
- Mutations, each restored byte for byte: a local source of mode 0000 taken for none (red); a wire one (red).
- The resume, sink and session tests (289) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_resume_partial`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`, its call; guard.

## Known gaps
- The single look has no guard of its own: no seam stands between two looks that are now one.
