# cr-ecm38-6: a delegated retry's report went out unlinked, and a rendering could take another file's link

**Severity**: MEDIUM — a failure loses its name and reason, or takes another file's. Brought in by cr-ecm37-5. Two encoders linked report entries unevenly:
- **The delegated retry.** It wrote its final report onto the delegated summary entry by entry, with no link, so a 4,504-byte path came back as a rendering and lost its reason on decoding.
- **Already-shortened paths.** The shared encoder linked a failure whose path was already a shortened rendering, by that text. Where another file's literal name was that rendering, the failure took that file's place.
**Status**: Verified — `combined-r40` (Codex openreview, 2026-11-24) reviewed `86d69ee3..d7f5f5e9` — the `combined-r39` fixes and log-named-text — with `guard_confirmed: true`: "The held directory, explicit links from diagnostics to filenames, and preservation of named failures withheld from a retry are worth keeping. I would require the following corrections."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `984d247a`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-5 (`a57e4bd0`) in part.

## Evidence
"A delegated retry emitted a zero index for a 4,504-byte path, losing its reason on decode. Separately, an already-shortened display that matched another file's literal name acquired that file's index. Use the shared encoder for delegated retries and require an exact identity before assigning a link."

## Intake
Admitted as the reviewer put it.

## What
- **The delegated retry** (`RetryOutcome::fold_into_delegated`) writes its report through the shared encoder (`failures_to_report`), as the other summaries are written. Each entry is linked to its exact name in the failed-path list.
- **The link** (`report_links`) is given only to an exact identity. A failure whose path is already shortened is a rendering, and takes none.

## Guard proof
- `sink::cr_win_1_tests::a_rendering_takes_no_link`: a rendering whose text is another file's name takes no link; that file's own entry takes its link.
- `transfers::retry::tests::a_delegated_retry_links_its_report`: a delegated retry's report of one long name is linked. Read back, it is that name with its reason.
- Mutations, each restored byte for byte:
  - a rendering linked by its text → red;
  - the delegated report unlinked → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,126 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `report_links`; guard.
- `crates/blit-cli/src/transfers/retry.rs` — `fold_into_delegated`; guard.

## Known gaps
- None known.
