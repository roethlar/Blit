# sf-2 — codex review adjudication

**Commit reviewed**: `c70c2ac` (sf-2: shape-correction stream resize
for many-file pushes)
**Raw review**: `.review/results/sf-2-shape-correction-resize.codex.md`
**Reviewer**: gpt-5.5 (codex exec, read-only sandbox)
**Verdict line**: NEEDS FIXES (1 finding)
**Fix commit**: `7627e7b` (sf-2: address review (1 finding))

## Findings

1. **`push/client/mod.rs:868` — Medium — ACCEPTED.** The shape target
   used `requested_files.len()`, but `prune_unrequested_payloads`
   **removes** entries from that set as payloads are matched
   (mod.rs:557/571, called from the Negotiation-arm drain). Verified
   against source: on a push whose early need batches arrive before
   the negotiation (e.g. 300 files: two 128-entry daemon flushes
   predate the client's Negotiation processing), the negotiation-time
   prune empties the set and later batches only rebuild the tail — the
   count permanently undercounts the true need shape, so the ramp can
   stall below the table target (codex's 128 + 9872 split example is
   the same mechanism). **Fix**: count from `files_requested`, the
   append-only accumulator already trusted for the completion check,
   matching the bytes side (`transfer_size_hint`, also append-only).
   Constraint recorded in `maybe_shape_resize`'s doc so a future
   refactor doesn't swap the sets back. Gate re-run: fmt/clippy clean,
   1483 passed / 0 failed.

Nothing rejected, nothing deferred. Codex also confirmed the +4 test
count statically (did not execute the suite in its read-only sandbox;
executed here: 1483/0).

reviewer: gpt-5.5
