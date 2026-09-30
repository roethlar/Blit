# cr-ssc6-1: Retry passes discard failures they did not observe

**Severity**: HIGH — a retry pass drops failures it did not observe; a path missing from the retry scan exits 0 while absent at the destination, and a remote move with a truncated retry set can delete source files that were never retried (data loss under a success exit)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `02e102bd`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b342d636..e14d1b72 (ssc-6), record .review/results/ssc-6-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/retry.rs:198 — the retry set may be truncated, but line 223 replaces the entire previous failure state with the subset pass's failures; crates/blit-cli/src/transfers/mod.rs:1016 then permits move source deletion using that reduced count.

## Predicted observable failure
A retry scan that no longer sees a requested path exits successfully although the destination lacks it; this reproduced locally with exit 0. More severely, a remote move whose >1 MiB failed-path list is truncated can retry only the named subset, clear files_failed, and delete source files that were never retried or transferred.

## Reviewer's suggested approach
Track pending failures by path and remove only paths positively confirmed transferred or already correct. Preserve unattempted/truncated failures and synthesize failures for requested paths missing or unreadable during the retry scan; move must refuse deletion while any such failure remains.

## What
A `files_from`-scoped scan reports every requested path it did not enumerate on `ManifestComplete.scan_failures` (missing → `source: missing at retry`, unreadable → `source: unreadable at retry: …`; bounded like the summary's failed-path list, overflow counted); the destination records each before the diff. `TransferSource::files_from_scope` exposes the scope through every wrapper (local route included). The CLI loop carries the unnamed remainder of a truncated retry set forward as `unretried`, counts it in `files_failed` (exit 2, move gate) and reports it once as a synthesized `(not retried)` entry.

## Guard proof
Session pins on both carriers (missing and unreadable requested paths become failures), a CLI pin removing the failed source file during the wait (exit 2, named as missing), unit pins for the truncated remainder and the move gate. Mutations: scan_failures not reported → session + CLI guards red; unretried remainder dropped → unit guard red (`scratchpad/cr-ssc-mutations-2.txt`).

## Known gaps
A retry pass positively accounts only for paths it was given; the truncated remainder is counted and named as a group, not per path (the wire could not carry them).
