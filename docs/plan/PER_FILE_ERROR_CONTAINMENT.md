# Per-File Error Containment

**Status**: Active (D-2026-07-30-1; Q1 settled — mirror deletes proceed,
move source-deletion refuses while any per-file failure exists)
**Created**: 2026-07-30
**Supersedes**: nothing
**Decision ref**: D-2026-07-30-1 (D-2026-07-09-1 supplies the governing
principle)

## Goal

A transfer session survives per-file errors. A single file that cannot be
written (I/O error, metadata non-convergence, locked/denied) is recorded and
reported in the end-of-operation summary naming the file and reason, with a
non-zero exit — the session completes the rest of the manifest. Additionally,
Windows attribute convergence tolerates the one known
filesystem-*synthesized* bit — HIDDEN reported on dot-named files by SMB
servers (Samba `hide dot files = yes` is that server's default) — in both the
apply and compare paths, so mirroring to a default-config Samba share
converges instead of erroring. Governing principle: D-2026-07-09-1 — "FAST,
SIMPLE, RELIABLE file transfer. if we abort the whole thing when we could
have fixed or surfaced a single error, we are violating all of those."

Motivating failure (2026-07-30, local mirror `D:\Apps\ → H:\apps\`,
`\\zoey\michael` SMB destination): source contained a stale rsync temp file
`.tcp_….png.XkR0Av`; Samba synthesized HIDDEN on the dot-named copy; the
apply-side read-back check saw 0x22 ≠ requested 0x20 and the entire mirror
aborted with `session INTERNAL: writing payload`.

## Non-goals

- No broad "ignore attribute mismatches" mode and no per-target
  configuration flags. The tolerance is exactly one semantically-justified
  case (extra HIDDEN on dot-named files); every other divergence remains an
  error (now per-file, not session-fatal). Accepted cost: nothing at read
  time distinguishes a server-synthesized HIDDEN from one a user set by
  hand on a dot-named destination file, so the latter is also tolerated
  (and never repaired) — on capable filesystems the apply path still sets
  the exact mask first, so the tolerance engages only when the setter
  could not clear the bit or the compare path sees it.
- No changes to target servers or their configs — the tool adapts, never the
  destination (owner directive, 2026-07-30).
- Session-fatal error classes stay session-fatal: transport death, protocol
  violations, containment/path-safety violations (R47-F1 class), tar-shard
  structural parse failures, destination-root unavailability, the
  incomplete-scan refusals at `transfer_session/mod.rs:4039` (mirror, otp-6b)
  and `:4052` (move, otp-9b F1). This plan does not weaken any refusal gate.
- No in-session retry logic. Convergence-on-retry via re-run stays the
  reliability model (D-2026-07-09-1 Q2).
- Source-side unreadable-entry handling is unchanged (already recorded via
  `record_unreadable_entry`, already gates mirror/move via `scan_complete`).
- No TUI work.

## Constraints

- Same-build peers only (D-2026-07-05-2): the proto change ships with a
  `CONTRACT_VERSION` bump (`crates/blit-core/src/transfer_session/mod.rs:78`,
  currently 5); no compatibility shims.
- ONE transfer path: everything lands in the unified session / shared sink;
  no side paths, no per-verb divergence.
- Windows parity: `scripts/windows/run-blit-tests.ps1` after touching
  `windows_metadata.rs`.
- Test count never drops; every behavioral change carries a red/green-proven
  guard test (AGENTS.md Verification).
- Failure lists carried in memory and on the wire are bounded (cap + total
  count) — no unbounded growth on catastrophic runs.

## Acceptance criteria

- [ ] Attribute tolerance: a dot-named file whose destination read-back is
      exactly `requested | FILE_ATTRIBUTE_HIDDEN (0x2)` converges in
      `set_and_verify_attributes` AND compares equal in
      `destination_matches`; a non-dot-named file with the same divergence,
      or any other extra/missing bit, does not. Proven by pure-function
      tests plus injected-read tests on both paths; red/green guard.
- [ ] Re-run convergence: after a mirror that exercised the tolerance, a
      second mirror run re-copies nothing (compare-side tolerance keeps the
      dot-named files converged).
- [ ] Per-file containment: an injected write failure on one file (e.g.
      access-denied on the destination path) no longer aborts the session —
      remaining manifest files transfer; the summary carries the failed
      file's relative path and reason; the CLI prints the failure block and
      exits non-zero (distinct partial-failure code, recommended `2`).
- [ ] Attribute non-convergence outside the tolerated case is a per-file
      failure (recorded, reported), not a session abort.
- [ ] Wire parity: the failure report survives the round trip in both
      carriers (in-stream and TCP data plane) and in both roles (initiator
      SOURCE and initiator DESTINATION), from the daemon DESTINATION back to
      the initiating CLI.
- [ ] Q1 posture (as ruled by owner) implemented and guard-tested.
- [ ] `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`
      green; Windows parity suite run; docs gate passes.

## Design

### Current state (verified 2026-07-30, all references at that tree)

- Apply-side convergence: `set_and_verify_attributes`,
  `crates/blit-core/src/windows_metadata.rs:321` — exact equality on
  `WINDOWS_PRESERVED_ATTRIBUTE_MASK` (0x27: READONLY|HIDDEN|SYSTEM|ARCHIVE),
  hard `bail!` on divergence.
- Compare-side: `destination_matches` → `destination_metadata_matches_with`
  (`windows_metadata.rs:257-290`) — `actual == *expected` on the full
  `WindowsFileMetadata` (attributes + named streams).
- Destination file-materialization sites (the complete `apply_attributes`
  call-site set): `remote/transfer/sink.rs:412/438` (DataPlaneSink),
  `:560/571` (`copy_resolved_file_payload`), `:702/721` and `:780/798`
  (tar-shard parallel writers), `:933/950`; plus
  `remote/transfer/tar_safety.rs:240/262` (`write_extracted_file`).
- Pipeline is first-error-wins: worker loop at
  `remote/transfer/pipeline.rs:440-531` propagates any
  `sink.write_payload(...)` error ("writing payload" contexts at
  `pipeline.rs:501/1270/1343/1382` — the latter three are the resume
  block/file paths) and sets the shared `cancelled` flag.
- `TransferSummary` (`proto/blit.proto:1059`) has no failure vocabulary:
  only `files_transferred`, `bytes_transferred`, `entries_deleted`,
  `in_stream_carrier_used`, `files_resumed`.
- CLI summary sites: `crates/blit-cli/src/transfers/remote.rs` (~:131 JSON
  final event, ~:526 summary), `transfers/remote_remote_direct.rs`
  (~:267-298), `transfers/local.rs` / `transfers/mod.rs` for the local verbs.
- Precedent for the CLI deliverable: otp-7 D4 rider (D-2026-07-09-1) — a
  mid-resume fault appears in the end-of-operation summary naming file(s)
  and suggesting a re-run, not only as a scrolling line.

### Half A — synthesized-attribute tolerance

One shared predicate in `windows_metadata.rs`, used by BOTH paths so they can
never drift:

```rust
/// True when `actual` equals `desired`, or when the only divergence is a
/// destination-synthesized HIDDEN bit on a dot-named file (SMB servers with
/// Samba's default `hide dot files = yes` report HIDDEN for names starting
/// with '.'; the bit is the server describing the name, not a metadata
/// defect). Both apply and compare use this one predicate.
fn attributes_converge(file_name: Option<&OsStr>, desired: u32, actual: u32) -> bool
```

Rule: `actual == desired`, OR (`actual == desired | 0x2` AND `desired & 0x2
== 0` AND the final path component begins with `.`). Extra-only, HIDDEN-only,
dot-names-only.

- `set_and_verify_attributes` takes the path's file name into the check
  (it already has `path`).
- `destination_metadata_matches_with` compares `named_streams` exactly and
  attributes via the predicate (it needs the file name — thread the path it
  already has through to the comparison).
- Manifest/source reads (`read_windows_metadata`) are untouched.

Tests: predicate unit tests (all four quadrants); apply-side injected
set/read closures (existing pattern at `windows_metadata.rs:836`); compare-
side injected read. Guard proof: revert the predicate to exact equality →
tolerance tests fail.

### Half B — per-file failure containment

1. **Failure record.** New type in `remote/transfer/sink.rs` (re-exported
   from `remote/transfer/mod.rs`):
   `FileFailure { relative_path: String, reason: String }`.
   `SinkOutcome` gains `failures: Vec<FileFailure>` plus
   `files_failed_total: u64` (the vec is capped at
   `MAX_REPORTED_FILE_FAILURES = 64`; the total keeps counting).
2. **Classification at the sink.** Each per-file write site catches errors
   that are attributable to exactly one file — open/create, payload write,
   named-stream replace, attribute apply, resume block patch, source read of
   that one file — records a `FileFailure`, and continues. Errors that are
   not attributable to one file (containment violations, tar parse errors,
   destination root creation, transport) keep returning `Err` and remain
   session-fatal. The tar-shard rayon writers collect per-file results
   instead of short-circuiting the whole shard.
3. **Pipeline merge.** `SinkOutcome::merge` (total at `pipeline.rs:519`)
   concatenates failure lists under the cap and sums totals. A worker only
   trips `cancelled` on `Err`, which now means session-fatal only.
4. **Wire.** `TransferSummary` gains
   `uint64 files_failed = 6; repeated FileFailure failures = 7;` with
   `message FileFailure { string relative_path = 1; string reason = 2; }`
   (list capped at 64 by the sender). `CONTRACT_VERSION` 5 → 6. Both roles
   populate it from the destination-side outcome; the responder DESTINATION
   path already returns its summary to the initiator, so the report rides
   the existing frame.
5. **CLI.** End-of-operation failure block (otp-7 D4 rider pattern): count,
   capped file list with reasons, "re-run to converge" hint; JSON events
   gain the same fields. Exit code: `0` clean, recommended `2` for
   "completed with per-file failures", existing non-zero for session
   failure. Move (`require_complete_scan` verbs): source deletion is gated
   on `files_failed_total == 0` — a failed file's source copy must survive
   (final posture per Q1 ruling).
6. **Mirror delete phase.** Per Q1 ruling. Note the classification-integrity
   argument: the extraneous set is computed against a complete source
   manifest (incomplete scans already refuse at `mod.rs:4039` before any
   transfer or deletion), and a write-failed file is in the manifest so it
   is never classified extraneous — copy failures do not corrupt the delete
   set.

### Risks

- Compare/apply drift if the tolerance predicate is duplicated — prevented
  structurally (one function, both callers).
- Silent-failure regression: containment must never downgrade a fatal class
  to per-file. The classification lives at the sink write sites where file
  identity is known; anything reached only by `?`-propagation stays fatal.
- Summary loss on the wire: guarded by the round-trip acceptance test in
  both carriers/roles.
- Windows-only surfaces (`windows_metadata.rs`) need the parity suite run on
  the owner's Windows rig; CI `windows-latest` covers the rest.

## Slices

One coherent, testable change per slice — sized for the review loop.

1. **pfc-1** — Half A complete: shared predicate + apply-side + compare-side
   + tests + red/green guard. (Smallest slice; alone it unblocks the
   motivating `D:\Apps` mirror.)
2. **pfc-2** — `FileFailure`/`SinkOutcome` extension + classification in the
   single-file write paths (`copy_resolved_file_payload`,
   `write_file_payload`, DataPlaneSink file path, resume patch sites) +
   pipeline merge + injected-failure unit tests.
3. **pfc-3** — tar-shard writers (both rayon paths + `write_extracted_file`)
   collect per-file failures; shard-level structural errors stay fatal;
   tests.
4. **pfc-4** — proto `TransferSummary` extension + `CONTRACT_VERSION` bump +
   both-roles/both-carriers propagation + round-trip tests.
5. **pfc-5** — CLI failure block + exit code + JSON fields + move
   source-deletion gate + mirror-delete posture per Q1 + integration tests.

## Open questions

- None. **Q1 is settled (D-2026-07-30-1, owner "go with recommendations"):**
  (a) mirror's extraneous-delete phase still runs under per-file write
  failures — the delete set's integrity is guaranteed by the existing
  complete-scan refusal (`transfer_session/mod.rs:4039`), and a write-failed
  file is in the source manifest so it is never classified extraneous;
  (b) move's source deletion refuses entirely while any per-file failure
  exists (`files_failed_total != 0` ⇒ no source deletion; re-run to
  converge, then delete).
