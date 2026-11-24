# cr-ecm37-2: a retry scan's failure for a text name took the bytes of another file of its spelling

**Severity**: HIGH — a retry reports success while a file it was asked for is still missing.

A scoped (retry) scan names each requested path it cannot find as that file's failure. It names one that is text by its text alone. At the session's end, failures named by text alone are given bytes from the manifest (`named_raw`), where a text names the one granted entry of that spelling. The scan's failures are never on the manifest.

The reviewer retried `dup` + U+FFFD alongside a sibling, `dup` + byte `FF`. The UTF-8 file had vanished, and its failure was given the sibling's bytes; the sibling the retry had copied. The next `jobs retry` asked for the sibling, found nothing to do and exited 0, with `dup` + U+FFFD still missing.
**Status**: Reviewed, found incomplete — `combined-r39` (Codex openreview, 2026-11-24) reviewed `6ee7ed5e..86d69ee3` — the `combined-r38` fixes — with `guard_confirmed: true`: "The session-held directory, explicit failure index, structured unknown disposition and hash-based summary lookups are sound choices. I would keep them and require the following corrections." "With a single-file source and a `--files-from` list containing only `.`, the first pass rejected the entry. With two retries, the command copied the unselected source and exited successfully. The scan converts text failures into raw-byte identities, bypassing list-entry handling on Linux. On Windows and macOS, that raw-name path drops even ordinary text names. Preserve the distinction between a text path, a raw filename and a list entry that names no file." The remaining work is cr-ecm38-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0e0b6d3c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-4 (`c8fe7147`) in part.

## Evidence
"I retried `dup�` and a sibling named `dup` followed by byte `FF`. After the UTF-8 source vanished, Blit copied the sibling and assigned its bytes to the missing file's failure. The next `jobs retry` exited **0** while `dup�` was still missing. Distinguish a known UTF-8 identity from one that still needs a manifest lookup."

## Intake
Admitted as the reviewer put it.
- **Known identity.** A failure whose identity is known is recorded by its exact bytes. For a text name these are its text's bytes. This is how intake already records a header it refuses, so the manifest lookup never touches it.
- **Lookup.** Only failures recorded by text alone at a write site still need the manifest lookup. The manifest grants one entry per text and refuses a second at intake, so for those the lookup is exact.

## What
`record_scan_failures` (`transfer_session/mod.rs`) records each failure a scoped scan reports by the requested name's own bytes:
- its raw bytes, where the name is not text;
- its text's bytes, where it is.

A path the report shortened is no name, and is recorded as it came. The session's intake of `ManifestComplete.scan_failures` goes through it.

## Guard proof
- `transfer_session::tests::a_scan_failure_keeps_its_own_identity`: a scan failure for `dup` + U+FFFD keeps its own bytes when the rest are named from a manifest that maps its text to `dup` + byte `FF`:
  - in the exact set;
  - in the report.
- Mutation, restored byte for byte: a scan failure recorded by its text alone → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,116 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `record_scan_failures`; the intake uses it; guard.

## Known gaps
- The end-to-end case of a vanished UTF-8 name and its byte-named sibling is guarded at the account, not through the CLI.
