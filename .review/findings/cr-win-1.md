# cr-win-1: Copy/mirror push retries can falsely clear a named-stream failure

**Severity**: HIGH — a retry pass can skip a file whose first-pass failure left the main bytes intact (e.g. a rejected NTFS stream), clear the failure, and exit 0 with an incomplete backup
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 5eeff4ac..8a1f04cb (Windows fixes win-1..5), record .review/results/ssc-win-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:417 — retry passes now invoke the deferred push with move_verb=false, selecting the ordinary SizeMtime/SizeOnly comparison at crates/blit-cli/src/transfers/remote.rs:493; that comparison explicitly ignores named streams at crates/blit-core/src/transfer_session/mod.rs:5929.

Context: before win-4 (`44442e54`) every push — main pass and retries — compared with the move rule (IgnoreTimes); win-4 restored the copy rule for the main pass and, with it, for the retry passes too.

## Predicted observable failure
On a Windows push using --size-only, if the destination writes the main file but rejects an NTFS alternate data stream, the first pass records a per-file failure. The retry sees the matching main-file size, skips the file without checking streams, replaces the failure set with an empty result, and exits 0 although the stream is missing—silently producing an incomplete backup.

## Reviewer's suggested approach
Use the ordinary copy comparison only for the main pass. Retry-only entries should be transferred unconditionally or verified with an exhaustive comparison that includes named streams; add a Windows guard where an ADS-tail failure remains reported until the stream is successfully applied.

## What
(coder fills in) Proposed: retry passes compare with IgnoreTimes (unconditional re-send of the retry set) on every route; the main pass keeps the copy rule.

## Guard proof
(red/green proof; mutation described) Proposed guard: a portable retry-pass test where the first pass fails a file after its main bytes landed with matching size/mtime, and the retry must re-send it (not skip it); plus the reviewer's Windows ADS-tail guard, run on the Windows machine.

## Known gaps
(none yet)
