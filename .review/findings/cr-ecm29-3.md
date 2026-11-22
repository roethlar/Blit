# cr-ecm29-3: on Windows a hard-linked destination is resumed in place

**Severity**: HIGH — on NTFS, resuming one name of a hard-linked destination writes through to every other name, so one backup snapshot's resume changes another's
**Status**: Admitted — NOT FIXED. Blocked: the fix is Windows-only code that nothing in this session can run, and no Windows host is reachable (2026-11-22).
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It is the Windows half of cr-ecm28-5 (`6d7be846`).

## Evidence
"The wire hash decision accepts every non-Unix file, and local resume opens the existing file for writing. On NTFS, resuming one name can therefore change another backup snapshot. Check the link count through the Windows file handle and give a multiply linked destination a fresh file." (`crates/blit-core/src/transfer_session/mod.rs`, the wire's hash decision: off Unix every opened regular file is reusable; `crates/blit-core/src/copy/file_copy/resume.rs`, the local acquisition: off Unix the existing file is opened for writing.)

## Restated
`combined-r31` (Codex openreview, 2026-11-22) reviewed `fd603571..93ecf7b2` — the six `combined-r30` fixes — with `guard_confirmed: true` restated it within cr-ecm30-6: "On Windows, the previously admitted NTFS hard-link issue remains open. Local and wire resume need a handle-based link-count check before patching." It stays blocked: `10.1.10.77` timed out, `.222` and `.173` had no route (2026-11-22).

## What it needs (not done)
- Off Unix (Windows), read the link count through the opened handle (`GetFileInformationByHandle`, `nNumberOfLinks`) in both places, and treat a destination with more than one name as not reusable: the wire hashes nothing for it, and the local resume makes it a fresh partial.
- Guards that run on Windows: a hard-linked destination (`std::fs::hard_link`) resumed locally and over the wire leaves the other name's bytes as they were.
- `scripts/windows/run-blit-tests.ps1` on a Windows host.

## Known gaps
- Until then, on Windows, `--resume` patches a hard-linked destination in place, as it did before cr-ecm28-5.
