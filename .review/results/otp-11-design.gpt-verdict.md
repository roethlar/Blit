# otp-11 design doc — codex verdict adjudication

**Reviewed**: `docs/plan/OTP11_LOCAL_SESSION.md` @ commit `0da65d6`
(plan change, D-2026-07-04-1). Raw review:
`.review/results/otp-11-design.codex.md` (gpt-5.6-sol via codex
v0.144.1, VERDICT: CHANGES REQUIRED, 10 findings).
**Context**: codex reviewed the DOC commit; the 11a implementation
(`dfdddd6`) landed while the review ran and had already independently
fixed several of the findings — noted per finding.
reviewer: gpt-5.5-class (gpt-5.6-sol)

## F1 (High) — "D1 violates one-transfer-path; choreography changed, not just the carrier"

**Accepted in part (doc overclaim), disputed in conclusion.** The doc's
"UNCHANGED choreography" wording overclaimed: the local carrier DOES
collapse the need-grant/payload phase into the destination's local
apply — need batches are not sent and the source serves no payloads.
The doc is amended to state the carrier's contract delta precisely
instead of denying it. The conclusion (that this violates
D-2026-07-05-1 and must be replaced by byte relay) is disputed:
(a) the owner's invariant is that direction/initiator/verb never
select code — for local↔local there is no direction/initiator
asymmetry, and every semantic layer IS the shared code
(same `destination_needs` verdicts, same `granted` dedup, same
`plan_transfer_payloads` planner, same `FsTransferSink` writes, same
`mirror_delete_pass`, same summary shape, same session state machine
for hello/manifest/refusals); (b) the parent plan itself classes
carriers as transport facts inside one session ("the gRPC-fallback
lane becomes a byte-carrier option... not a separate transfer path")
and D-2026-07-05-3 fixes the receive sink as a capability-selected
write-strategy seam; (c) full byte relay fails the plan's own
converge-up/perf constraint by orders of magnitude on same-volume
clonefile/block-clone copies — a "pure" state machine that loses the
perf gate ships nothing. Resolution: doc §D1 rewritten (shared-layer
enumeration + explicit phase delta); a "Local carrier" note rides
`docs/TRANSFER_SESSION.md` at 11b so the contract doc names the
carrier explicitly.

## F2 (High) — sink File payload not single-file-safe (empty rel → ENOTDIR)

**Accepted — already fixed in `dfdddd6`** (found independently by the
ported pin `single_file_copy_lands_and_records_history`):
`write_file_payload` routes the file-root identity case past the joins
(`copy_root_file_payload`). Guarded by that pin.

## F3 (High) — session diff and sink re-check can disagree (partial-hash / mtime tolerance)

**Accepted as a real, pre-existing nuance; not a regression — behavior
parity with the old path.** The sink re-check
(`file_needs_copy_with_checksum_type`: size → first/last-MiB partial
hash → mtime tolerance) is the OLD local pipeline's own defense layer,
unchanged; the old streaming planner + this sink produced exactly the
same skip-after-plan outcomes and the same `files_written` accounting.
What remains true: local and remote can count differently inside the
tolerance window (remote always writes needed files). Filed in the doc
as an 11b follow-up decision: align the sink defense layer with
`header_transfer_status` (the one compare owner) or retire the
second-guess for session-driven writes.

## F4 (High) — dest-inside-src exclusion lost

**Accepted — already implemented in `dfdddd6`**
(`DestSubtreeExcludedSource`), guard-proven this session:
mutation (exclusion bypassed) → `nested_destination_does_not_self_copy`
FAILS → restored → passes.

## F5 (High) — sink-level resume is a topology-specific resume path

**Accepted in part.** The doc is amended to frame local resume as the
local carrier's block phase: the same semantic (hash the partial,
rewrite only differing blocks, full-file fallback) executed by the
shared `resume_copy_file` primitive without serializing block records
that would immediately be deserialized in the same process. It is a
carrier-level implementation of the one resume contract, not a skipped
feature; a local `--resume` behavior pin is added in the 11a fix round.
Running the wire block phase over in-process frames was considered and
rejected: it relays every changed block's bytes through the frame
channel — the same relay the carrier exists to avoid.

## F6 (Medium) — symlink parity claim false for `preserve_symlinks=false`; `skip_unchanged=false` omitted

**Accepted — doc scoped.** True: the old engine followed symlinks
under `preserve_symlinks=false` and honored `skip_unchanged=false`;
the session route does neither. Neither axis is reachable from any
production caller (the CLI never sets them; the TUI transfer path uses
defaults; `screens/f4.rs` is the browse screen, not a transfer). The
doc now claims parity for every reachable option value and retires the
dead axes with the options re-home at 11b.

## F7 (Medium) — mirror pass counts/plan-only didn't exist; SourceEmpty would hide deletions

**Accepted — already implemented in `dfdddd6`**:
`mirror_delete_pass(execute) -> (files, dirs)`, plan-only under local
dry-run, split stored for the local summary (guard-proven: swapped
counters fail the split pin; forced execute fails the dry-run pin).
The SourceEmpty concern is already excluded by construction: outcome
classification only reports SourceEmpty/UpToDate on non-mirror
default-compare runs (the shapes the old fast paths could reach), so a
mirror run always prints the deletion line.

## F8 (High) — retiring journal skip conflicts with the no-op ≤ old+10% gate

**Accepted in part (doc nuance), gate kept.** The old no-op path on
the bench rig was `no_work` — a FULL enumerate + per-entry
`should_copy_entry` stat pass (`engine/strategy.rs`), the same work
class as the session diff; the journal skip engaged only with a prior
snapshot on journal-capable filesystems and is not what the A/B's old
side measures (the retired `local_transfers.rs` no-op pin itself
asserted `no_work`, not `journal_skip`). The mirror-shaped no-op cell
forced the old path to full streaming. The doc now says this
precisely, and Known gaps carries the honest owner-visible note: on
journal-capable systems with very large unchanged trees, the retired
journal skip was an absolute-time win the session route does not
reproduce. The measured gate stands; a FAIL blocks 11b as designed.

## F9 (Medium) — deleting `execute_sink_pipeline_streaming` leaves callers/tests

**Moot by implementation — reversed in the doc.** 11a made it the
local apply pipeline (a live production caller); it is no longer on
the 11b dead-code list, and its tests stay.

## F10 (High) — floor arithmetic does not close; manifest "live-half tests" wrong

**Accepted.** Verified: all 16 `manifest.rs` tests drive
`compare_manifests`; none pins `header_transfer_status` directly.
Honest arithmetic (post-11a suite 1510): 11b retirements ≈ 71
(orchestrator 16, engine-non-dial 19, auto_tune 6, blit-core
integration 10, manifest block 16, `plan_local_mirror` 4) → 1439
without replacements; the ≥1483 end-of-plan floor needs ≈ +44 real
pins by otp-13. The doc's floor section is rewritten with a named
closure plan (direct `header_transfer_status` ports, local resume
pins, un-consolidated orchestrator ports, `record_local_history`
contract ports, mirror-pass unit pins, streaming-overlap port, new
session-edge pins) and the residual is flagged for the otp-13
checklist walk rather than hand-waved.

## Disposition

Doc amendment commit: (recorded alongside this verdict). Findings
F2/F4/F7 were already closed by `dfdddd6` (the 11a slice, reviewed
separately as `.review/results/otp-11a.codex.md`); F1/F5/F6/F8/F9/F10
close via the doc amendment; F3 is filed as an 11b decision item.
