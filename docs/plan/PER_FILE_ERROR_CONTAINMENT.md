# Per-File Error Containment

**Status**: Active (D-2026-07-30-1; Q1 settled — mirror deletes proceed,
move source-deletion refuses while any per-file failure exists; pfc-6
metadata-only repair added by owner go, D-2026-07-31-1)
**Created**: 2026-07-30
**Supersedes**: nothing
**Decision ref**: D-2026-07-30-1, amended D-2026-07-31-1 (D-2026-07-09-1
supplies the governing principle)

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
- [ ] Metadata-only repair (pfc-6): a destination file with equal
      size/mtime and matching streams but divergent attributes converges
      with zero payload bytes re-sent; stream divergence still transfers
      fully; repair failure degrades to full transfer, never a new fatal.
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
  call-site set): `remote/transfer/sink.rs:412/438`
  (`FsTransferSink::write_file_stream` — the destination-side streaming
  write; the pfc-1 draft mislabeled these lines "DataPlaneSink", corrected
  at pfc-2 landing), `:560/571` (`copy_resolved_file_payload`), `:702/721`
  and `:780/798` (tar-shard parallel writers), `:933/950`; plus
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

### pfc-2 landing notes (recorded boundaries and deferred items)

- **Interim interlock (pfc-5 removes it):** containment is live for MIRROR
  sessions only. A non-mirror session (`!mirror_enabled`) keeps a contained
  per-file failure session-fatal at SourceDone — mirroring is the only
  session declaration proving no source deletion follows, so this exactly
  preserves pre-pfc-2 behavior for every move route and closes the
  move-deletes-unlanded-source window the pfc-2 review caught. pfc-5
  replaces this with the real `files_failed_total == 0` source-deletion
  gate and extends containment to non-mirror sessions.
- Every contained failure logs `log::warn!` at the single
  `record_failure` chokepoint (interim visibility until pfc-5's summary
  block).
- **Send-side source read stays fatal** (`prepare_payload`,
  pipeline.rs): skipping a granted file at the SOURCE trips the
  DESTINATION's "needed file(s) never delivered" protocol check, so
  source-side containment needs a wire skip signal — pfc-4 at the
  earliest, or a recorded non-goal.
- **`validate_payload` asymmetry (deliberate):** the wire receive path
  (`write_file_stream`) faults on peer-supplied metadata shape before any
  filesystem effect; the local File-payload path reads metadata from the
  local source, so the same validation failure there is contained as that
  file's failure.
- **`file_failed` cap identity (pfc-3 must decide):** past the 64-entry
  cap the predicate answers true for every path — conservative
  (suppresses completions, never reports a failed file complete), but a
  tar shard with >64 member failures would suppress completions for its
  healthy members; pfc-3 needs per-member identity or an explicit accept.
- **Byte accounting:** a contained flush-failure has already reported its
  payload bytes to the live byte counter while the summary counts 0 for
  the file — pfc-4 reconciles or records the divergence.
- A held resume-block failure is reported only when its completion record
  arrives; a sender that never sends the completion is a protocol
  violation today, so the drop is unreachable — noted, not coded around.

### pfc-3 landing notes

- All shard writers fold per-member results through one helper
  (`fold_shard_member_results`) gated on the SAME `failure_is_containable`
  predicate as the single-file paths (cr-pfc2-1's volume check included —
  guard-proven load-bearing on the shard paths). Structural tar parse and
  R47-F1 containment verification stay fatal above the writers.
- Exact per-outcome failed identity: `failed_paths`
  (`HashSet`, post-review) is uncapped per payload — bounded by the
  planner's member clamp (≤4096) — while the 64-cap bounds only the
  carried `FileFailure` details. A merged outcome answers `file_failed`
  conservatively (true for all); the ONLY completion lane reading a
  merged outcome is the single-file resume block record, where
  conservative equals exact — documented at `file_failed`; a future
  multi-file merged outcome must never feed completion filtering.
- `write_extracted_shard`/`write_extracted_file` have NO production
  caller (the pre-unification consumers died with their drivers) —
  sequential convenience wrappers, tests only; stale caller docs
  corrected.
- audit-17 shape is contained at the shard level (siblings land, the bad
  member is named); a non-mirror `copy` still ends fatal at SourceDone
  via the pfc-2 interlock until pfc-5 removes it.
- Boundary pins (structural/traversal fatal tests) are pins, not
  red/green guards — they short-circuit above the fold and go red under
  no pfc-3 revert; recorded honestly.
- **pfc-4 addition:** the pipeline byte lane reports planned sizes for
  contained shard members while the in-stream lane reports
  `bytes_written` — the carriers disagree on bytes for a failed-member
  shard; pfc-4's byte-accounting reconciliation now covers this case.
- Session-level end-to-end containment coverage (a real session with a
  blocked file, both carriers) is owed by pfc-5's integration tests.

### pfc-4 landing notes

- Wire: `TransferSummary` carries `files_failed` (exact total) +
  `failures` (sender-capped at 64) as contract v6; the ONE summary
  construction site covers both roles/carriers/daemon;
  `LocalMirrorSummary` carries the same shape for pfc-5.
  `docs/TRANSFER_SESSION.md` moved in lockstep, including the
  resume-block byte semantics now stated as contract
  (`bytes_transferred` = bytes the destination applied, stale resume
  blocks of a later-failed file included).
- cr-pfc2-2 closed at the TOTALS level via
  `ProgressEvent::SummaryReconciled { files_failed, bytes_landed }`:
  the SOURCE adopts the destination's authoritative byte total and
  retracts failed completions once, at the summary boundary, gated on
  `files_failed != 0` (unconditional adoption would erase dry-run
  planned-byte progress). The per-file SOURCE event stream remains
  optimistic by design — the wire cannot attribute per-file identity
  past the cap; pfc-5 renders failed files from the summary list.
- Live-counter withdrawal: a contained flush/metadata-tail failure
  withdraws exactly its reported `header.size` from `ByteProgressSink`
  (saturating), closing the pfc-2 byte-divergence item on all three
  lanes.
- Notes for pfc-5: `ProgressTotals.files_failed` is populated on SOURCE
  lanes only (destination lanes filter completions instead — do not
  read it as universal); `files_completed` is non-monotonic at exactly
  one point (the reconciliation) and every delta consumer uses
  saturating subtraction; the 64-entry wire cap is sender-side only
  (same-build refusal is the defense — a decoder clamp would be
  defense-in-depth if peers ever loosen).

### pfc-5 landing notes

- The pfc-2 interim non-mirror interlock is REMOVED; containment applies
  to every session kind. Its replacement, the Q1(b) gate
  (`refuse_source_delete_on_failures`), lives in **blit-app** — one
  shared function serving BOTH shipped surfaces — after the verify round
  caught that gating only the CLI would have left blit-tui's four move
  routes deleting sources whose files never landed. All eight
  source-deleting routes (CLI local/push/pull/delegated + TUI
  local/push/pull/delegated) call it ahead of any deletion.
- CLI surfacing: end-of-operation failure block (count, capped list,
  elided-remainder note, re-run hint) on all four routes incl. delegated
  (consuming cr-pfc4-1's accessors); exit code 2
  (`EXIT_PARTIAL_FAILURE`) for completed-with-failures; JSON gains
  `files_failed` + `failures` and still exits 2. The block prints
  outside `print_summary` deliberately: a run whose only file failed
  classifies UpToDate and that fn early-returns.
- audit-17 is CLOSED end to end: a non-mirror copy with one rejected
  filename completes, lands siblings, names the file with its reason,
  and exits 2 (integration-proven against the real binary).
- Recorded gaps / follow-ups:
  - TUI failure-report RENDERING stays a non-goal per this plan (the
    TUI gates moves — data safety — but still shows a partial-failure
    transfer as green); a TUI slice needs its own owner go.
  - The remote routes' block/exit-2 have no e2e harness (blit-tui/cli
    suites lack a two-daemon fixture); the surface fn is shared with
    the proven local path and the wire carriage is daemon-e2e-proven.
  - A DETACHED delegated transfer completing with failures has no
    surfacing path (the job record holds the summary; jobs UI work).
  - "Up to date: 0 changed" + `"outcome": "up_to_date"` still renders
    when the only file failed (the block right below contradicts it) —
    cosmetic-adjacent classification follow-up.
  - The lifecycle trace records Success for an exit-2 run (session
    succeeded, files failed) — semantics noted, not changed.
  - The three remote TUI move-route gates are compile-verified (same
    shared function; no TUI two-daemon fixture) — recorded honestly;
    the local TUI route is red/green-proven.

### Half C — metadata-only attribute repair (pfc-6, D-2026-07-31-1)

Field evidence (2026-07-31, `H:\apps` pre-existing backup regions, e.g.
`SysinternalsSuite`): destination files whose size and mtime exactly match
the source read back attributes `Normal` (0x00) against source `Archive`
(0x20). `finalize_need_verdict` treats Unchanged-with-metadata-mismatch as
a full re-transfer (`transfer_session/mod.rs:5134-5159`), so a mirror over
such a tree re-sends every byte to repair one attribute bit.

Change: when `header_transfer_status` is `Unchanged` and
`destination_matches` fails, split the divergence:

- **Attributes-only divergence** (named streams match by name/size/checksum
  — the manifest-level compare carries stream checksums via
  `read_windows_metadata(path, false)`): the DESTINATION repairs in place
  at diff time — `apply_attributes` with the manifest attributes, judged by
  the shared pfc-1 `attributes_converge` predicate — and the file never
  enters the need list. The destination owns its filesystem in every
  topology, so this works identically for local and remote sessions.
- **Any stream divergence** (name/size/checksum): full transfer as today —
  stream bytes need the payload.
- **Repair failure degrades to full transfer** (`NeedVerdict::Transfer`);
  if the transfer's apply then also fails, the pfc-2 per-file containment
  owns it. Repair never introduces a new session-fatal path.

Counting: repaired files increment a destination-local repaired counter
surfaced in the local summary (and the CLI line in pfc-5); the wire
`TransferSummary` is not extended for it in this slice (pfc-4 owns wire
changes; fold the counter there only if pfc-4 has not landed yet —
otherwise a follow-up wire field needs its own slice note).

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
   both-roles/both-carriers propagation + round-trip tests. Also owns
   review finding **cr-pfc2-2**: initiator-SOURCE completed-file accounting
   must reconcile against the destination's returned failure report (a
   sender-side FileComplete is not destination confirmation); and the
   byte-accounting divergence from the pfc-2 landing notes.
5. **pfc-5** — CLI failure block + exit code + JSON fields + move
   source-deletion gate + mirror-delete posture per Q1 + integration tests.
   Also removes pfc-2's interim non-mirror interlock: containment extends
   to non-mirror sessions once the `files_failed_total == 0` gate guards
   every source-deleting route.
6. **pfc-6** (added D-2026-07-31-1) — metadata-only attribute repair at the
   destination diff: attributes-only divergence repairs in place with zero
   payload bytes; stream divergence still transfers; repair failure
   degrades to full transfer. Guard: a size/mtime-equal, attribute-divergent
   destination file converges with zero payload bytes (test fails when the
   repair is reverted to full re-transfer... assert on bytes, not just
   convergence); a stream-divergent file still re-copies.

## Open questions

- None. **Q1 is settled (D-2026-07-30-1, owner "go with recommendations"):**
  (a) mirror's extraneous-delete phase still runs under per-file write
  failures — the delete set's integrity is guaranteed by the existing
  complete-scan refusal (`transfer_session/mod.rs:4039`), and a write-failed
  file is in the source manifest so it is never classified extraneous;
  (b) move's source deletion refuses entirely while any per-file failure
  exists (`files_failed_total != 0` ⇒ no source deletion; re-run to
  converge, then delete).
