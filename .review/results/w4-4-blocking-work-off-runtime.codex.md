# codex review — w4-4-blocking-work-off-runtime @ 0feca34

Invocation: `codex exec -s read-only` (gpt-5.5, superpowers plugin
disabled), 2026-07-04. Raw session transcript trimmed to the final
findings per the established `.review/results/` size convention. The
exploration walked the push manifest-loop restructure (need-list
order, design-4 invariant, spin-up relocation, ManifestComplete
drain, resize-arm interleaving), the pull_sync offload, and the F2
containment posture (explicitly endorsing the rejection of the
lexical-containment alternative).

## Findings

1. **Medium** — `push/control.rs` (chunked drain): `pending_manifest`
   drains only at 128 entries or `ManifestComplete`, so
   `FILE_LIST_EARLY_FLUSH_BYTES` and `FILE_LIST_EARLY_FLUSH_DELAY`
   can no longer emit the first need-list before 128 manifest
   entries. Old behavior could flush after 64 KiB or 5 ms and spin up
   TCP mid-manifest; this delays `FilesToUpload`/negotiation and
   breaks the claimed early-flush cadence.

VERDICT: NEEDS FIXES. F2 canonical containment stays per-entry and
rejecting lexical containment was the right call. Pull-sync semantics
look preserved. Diff evidence adds 3 tests and removes none, matching
1472 -> 1475; I did not rerun the suite in this read-only sandbox.

tokens used: 89,879
