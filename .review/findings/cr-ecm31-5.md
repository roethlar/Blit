# cr-ecm31-5: on Windows the whole-file baseline is discarded and a hard-linked destination is resumed in place

**Severity**: HIGH — off Unix the wire's hold discards `ResumeBaseline` (cr-ecm30-1, cr-ecm31-2) and holds the destination as before; and the local resume opens an existing NTFS file for writing without asking its link count, so resuming one name of a hard-linked backup tree changes the others
**Status**: Admitted — NOT FIXED. Blocked: Windows-only code that nothing here can run, and no Windows host answers (`10.1.10.77` timed out; `.222` and `.173` no route; 2026-11-22).
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It restates cr-ecm29-3 and adds the discarded baseline.

## Evidence
"The non-Unix path discards the new baseline, and local resume still opens an existing NTFS file for writing without checking its link count. Resuming one name can change another backup snapshot. Use handle-based identity and link-count checks on both paths and exercise them on Windows." (`crates/blit-core/src/remote/transfer/sink.rs`, `hold_partial` off Unix; `crates/blit-core/src/copy/file_copy/resume.rs`, the local acquisition off Unix.)

## What it needs (not done)
- What cr-ecm29-3's record says: the link count through the opened handle (`GetFileInformationByHandle`, `nNumberOfLinks`) on both paths, a destination with more than one name not reused.
- And off Unix, `hold_partial` honouring `ResumeBaseline::WholeOf { leaf, end }`: the file found there (its identity through the handle — volume serial and file index), or nothing, and nothing else replaced.
- Guards that run on Windows; `scripts/windows/run-blit-tests.ps1` on a Windows host.

## Known gaps
- Until then, on Windows, `--resume` patches a hard-linked destination in place, and its whole-file hold looks for itself.
