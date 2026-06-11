# design-1 — CLI pull progress double-counts bytes on the TCP data plane

**Source**: Design-coherence review Phase A (`docs/audit/DESIGN_MAP_2026-06-11.md` §1.6),
mechanism re-verified by hand 2026-06-11 before filing.
**Severity**: Medium (RELIABLE — user-visible wrong numbers that contradict the
tool's own completion summary).

## What

The TCP receive pipeline reports each received file **twice** to the progress
sink with the full byte count both times:
`crates/blit-core/src/remote/transfer/pipeline.rs:233-235` calls
`p.report_payload(0, outcome.bytes_written)` **and**
`p.report_file_complete(header.relative_path.clone(), outcome.bytes_written)`
for the same file.

The CLI monitor folds both arms into the same accumulator:
`crates/blit-cli/src/transfers/remote.rs` adds `bytes` in its
`ProgressEvent::Payload` arm **and** in its `ProgressEvent::FileComplete` arm
(both `total_bytes = total_bytes.saturating_add(bytes)`). Net effect: the live
`[progress]` line and the final `[progress] final:` line show ~2× the real
bytes on TCP pulls, then the `Pull complete: N bytes` summary printed
immediately after (from the wire report) contradicts them.

The TUI documented and dodged exactly this trap (`blit-tui` `progress_accum.rs`
carries per-direction folding rules); the CLI never got that fix.

## Root cause (design)

`ProgressEvent` has no semantic contract for which arm carries bytes; three
producer families assign three incompatible meanings (see map §1.6). This
finding fixes the CLI symptom; the contract problem stays with the design
review (Phase B/C).

## Proposed fix (slice-sized)

Align the CLI fold with the TUI's pull rule (count bytes from one arm only on
this path), or change the TCP receive producer to report bytes on exactly one
arm — whichever preserves the gRPC-path accounting (gRPC pull reports bytes on
`Payload` with `FileComplete` carrying 0, per map §1.6). Add a regression test
that drives both producer shapes through the CLI fold and asserts totals equal
actual bytes once.

## Cross-references

- Map §1.6 (progress reporting) — full pipeline inventory.
- `blit-tui/src/progress_accum.rs:12-20` — the documented trap and the
  per-direction folding precedent.
