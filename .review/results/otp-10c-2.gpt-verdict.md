# otp-10c-2 — codex verdict adjudication

reviewer: gpt-5.5 (codex exec, model gpt-5.6-sol, read-only; raw output
`.review/results/otp-10c-2.codex.md`)
slice commit: `7aac28b`
codex verdict: NEEDS FIXES — 6 findings (5 Medium, 1 Low). Codex
explicitly confirmed: relocations verbatim, DelegatedPull no-payload
proof holds, A/B→absolute conversions sound, session invariants
untouched.

## F1 (Med) — spec capability/capacity fields semantically orphaned — **Accepted**

Verified: nothing reads `normalized.capabilities` or the spec-level
`receiver_capacity` anywhere — delegation stopped forwarding the spec
at otp-9b (the dst daemon initiates its own session and advertises its
own capacity in `SessionOpen`), and D-2026-07-05-2 made capability
bits meaningless. The proto comments still claimed a forwarding/
override boundary that no longer exists. Fix: deleted
`TransferOperationSpec.client_capabilities` (field 8, reserved),
`TransferOperationSpec.receiver_capacity` (field 12, reserved), the
`PeerCapabilities` message, the normalized members, the builder
population, and rewrote the DelegatedPullRequest/CapacityProfile
comments to the post-9b truth. `CapacityProfile` itself stays — live
in `SessionOpen`/`SessionAccept`.

## F2 (Med) — five newly orphaned helpers — **Accepted**

All five verified caller-less (they are `pub` in a lib crate, so no
dead-code warning fired): `EnumerationOutcome::is_complete`,
`manifest::files_needing_transfer`, `FileFilter::allows_relative`,
`FsTransferSink` path-tracker (field + `with_path_tracker` + `track`
+ per-write call sites — the old pull's purge bookkeeping), and
`BufferSizer::return_vec`. Deleted. **Noted beyond the finding**:
`manifest::compare_manifests` is also caller-less (the session uses
the per-entry `header_transfer_status`), but it is local-path-adjacent
surface with its own test block — deferred to otp-11's sweep rather
than widening this fix commit.

## F3 (Med) — relocated builder lost its direct pins — **Accepted**

True: `delegated_spec_from_options` is live code and its coverage
died with the old driver's `spec_extraction_tests`/wire test. Fix:
`operation_spec::delegated_spec_tests` (7 pins — endpoint Module/
Root/Discovery/empty-rel mapping, the old driver's full compare
precedence table, mirror Off/FilteredSubset/All, and field carriage
with a `from_spec` normalization round-trip standing in for the old
wire round-trip). Guard-proven: inverting the ignore_times/force
precedence in the builder fails the precedence pin; restored.

## F4 (Med) — containment claim unsupported — **Accepted**

Codex is right on both counts: the otp-6b role-suite pins cover
FilteredSubset SCOPING, not containment, and no test pinned
`mirror_delete_pass`'s per-target `verify_contained` wiring (a
mutation deleting the call survived the suite — reproduced). The
threat is narrower than the old daemon-authored-list one (delete
candidates come from the pass's own local plan, never the wire;
containment is defense-in-depth against symlink-shaped escapes), but
the wiring is load-bearing and now pinned:
`mirror_delete_pass_containment_check_gates_every_deletion` (foreign
canonical root ⇒ refusal with nothing deleted; real root ⇒ control
arm deletes). Guard-proven by commenting out the `contained(...)`
call — the pin fails; restored. The finding doc's overclaim is
corrected in place.

## F5 (Med) — stale live docs — **Accepted**

`docs/API.md` had never been swept (the 10c-1 sweep grep did not
include it): service block, whole Push/Pull operation sections, the
DataTransferNegotiation section, the `RemotePushClient` example, and
the versioning claim ("clients should specify the expected version")
all rewritten to the session truth (D-2026-07-05-2 for versioning).
Also fixed: `docs/ARCHITECTURE.md` Hybrid Data Plane section (dead
message → session grant/resize frames), `docs/WHITEPAPER.md` §6
(pull_sync protocol → the session's destination-side compare/mirror/
resume), `service/transfer.rs` module doc (dead dispatcher names;
its byte-progress gap note now points at the re-scoped REVIEW row),
and `REVIEW.md` `w6-2b` — an OPEN row whose prescription targeted the
deleted handlers; re-scoped to the served-session dispatcher, where
the byte-counter gap genuinely persists (kept open).

## F6 (Low) — tracked worktree snapshot still contains the old tree — **Accepted, owner-gated**

True: `.claude/worktrees/vigilant-mayer/**` (the stale snapshot
committed at `725aa07`) still contains all four drivers and the old
proto, so the literal "no longer exists in the git tree" reading of
the deletion proof holds only for the real workspace. Its removal is
ALREADY the standing STATE open question ("rec `git rm -r`, awaits
go") — destructive-action policy says the owner names the go, so this
stays deferred there rather than self-executed; the deletion proof is
scoped to the workspace source tree until then.

fix sha: (recorded after the fix commit — see REVIEW.md row)
suite after fixes: 1480 → expected 1488 (+8 pins; gate results below)
