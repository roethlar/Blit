# design-5-send-failure-masks-rejection — push rejection reason lost to a send-failure race

**Branch**: `master` (owner ratified 2026-06-12: "I maintain a strong bias for proper fixes, so 1")
**Commit**: `08d71a2`
**Found by**: first CI run with the w9-1/w9-4 ungated tests (run 27413185126) —
`push_to_read_only_module_is_rejected_and_module_untouched` failed on macOS
AND Windows while passing locally and on Linux.

## What

When the daemon rejects a push (e.g. read-only module), tonic drops the
request stream; a client mid-send hits the dead channel before its
response task delivers the daemon's terminal Status. The user then saw
`failed to send push request payload` instead of `module 'test' is
read-only`. Pure timing — local runs won the race, CI machines lost it.
The w9-4 test's strict error-text assertion (per its slice spec, "lock in
failure-message quality") caught it on its very first cross-platform run.

## Approach

- `helpers::prefer_server_error(response_rx, send_err)`: on a failed
  request-stream send, drain the response channel under a 2 s deadline for
  the daemon's terminal error and prefer it (`push rejected by daemon:
  <reason>`); fall back to the bare send error if none arrives.
- Applied at the three send sites inside the push main loop that own
  `response_rx`: header send, per-entry `FileManifest` send (the
  CI-observed site), and `ManifestComplete`.
- New regression: 500-file push at a read-only module — the client is
  guaranteed mid-manifest when the rejection lands, so the recovery path
  runs deterministically; asserts the read-only reason and an untouched
  module. 5× re-runs clean.

## Files changed

- `crates/blit-core/src/remote/push/client/helpers.rs` (+helper)
- `crates/blit-core/src/remote/push/client/mod.rs` (3 send sites)
- `crates/blit-cli/tests/readonly_enforcement.rs` (+1 test)

## Tests added

1 (suite 1369 → 1370).

## Known gaps

- Fallback-payload send sites (`stream_fallback_from_queue`, sink paths)
  still bubble bare send errors — they don't own `response_rx`. That is
  exactly w5-4's queued slice (one honest send-failure vocabulary,
  prefer the real error); this fix covers the manifest-phase sites where
  the CI failure lived.
- **Sentinel overlap, deliberate**: design-4 (pending, `ddfeb58`) and this
  slice both touch `push/client/mod.rs` — same code region, same
  discovery chain, both owner-ratified this session. Grade design-4
  first, then design-5; the commit ranges are sequential and
  independently diffable.
- The 2 s harvest deadline is a judgment call: long enough for a local
  RTT, short enough not to hang a dead connection's error path.
