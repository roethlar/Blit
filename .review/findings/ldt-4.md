# ldt-4 — quiet rig-W evidence harness and analyzer

**Slice**: `LIVE_DIAL_TUNING` ldt-4. Measure the accepted adaptive controller
on rig-W `q`↔`netwatch-01` with identical physical paths under both initiator
layouts and both byte directions.

**Status**: Review fixes in progress — `f1`–`f3`, `f5a`, `f6`, and duplicate-key
`f7a` are guard-proved; exact ratio check `f7b` precedes neutral
fixed-SHA whole-change re-review. No candidate artifact has been staged and no generated harness
operation or live arm has run on either endpoint; only the read-only parser
check described below and the in-memory crash-guard SSH probe occurred.

**Branch**: `master`

**Implementation range**:
`e41b87173f2073a9b6694a62813eddc14a7844ad..7491b4fbdfc84bde2fa713894d292855faaf3b13`

**Artifact under test**: accepted ldt-3 runtime
`406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77`

## What

The adaptive policy and role parity are deterministic-test proven, but ldt-4
still needs a registered hardware experiment that can distinguish semantic
SOURCE/DESTINATION from connection initiator without changing the physical
source or destination path. The instrument must retain every partial result,
bind traces and manifests to each arm, restore any pre-existing Windows daemon
exactly, and make no worker-count target part of acceptance.

## Approach

- Run exactly 96 arms: six direction/fixture cells, eight adjacent pairs per
  cell, and an `ABBAABBA` first-role schedule across pairs. Fixtures are large,
  10k-small, and mixed in both byte directions.
- Invoke `blit copy` for every arm. Within a direction/fixture cell, both
  initiator layouts use the same exact source, active destination, and retained
  archive paths; only which endpoint calls the responder changes.
- Pin the accepted `406a7e5` client and daemon artifacts on both endpoints and
  require the running repository HEAD to equal the separately reviewed harness
  SHA supplied at launch.
- Create session, run, trace, manifest, and result paths exclusively. Successful
  destinations are atomically renamed to retained run IDs; failed sessions keep
  their partial evidence and are marked void. The harness contains no deletion
  path and refuses symlink/reparse traversal or overwrite.
- Record endpoint-local client duration, exact environment/provenance, binary
  hashes, PID/command ownership, socket lifecycle, dial samples/settlements,
  final inventory, and source/landed content manifests.
- Validate the immutable evidence set with a separate analyzer. It replays the
  production dial policy from raw samples, checks exact role/path/event binding,
  compares paired decision traces and throughput, and reports review-required
  conditions rather than inventing a preferred worker count.
- Before replacing the active Windows daemon, durably record and volume-flush
  swap intent. Recovery is idempotent and restores the exact prior daemon, or
  the originally absent state, without classifying or mutating an unrecorded
  active path.

## Files changed

- `scripts/bench_ldt4_rigw.sh`
- `scripts/ldt4_rigw_analyze.py`
- `scripts/ldt4_rigw_analyze_test.py`

## Candidate commits

- `8847065` adds the evidence analyzer; `052e194` and `ce05f7b` bind exact
  evidence paths and socket lifecycle.
- `d6d0b4f` adds the rig-W harness.
- `48ee28a`, `6edcc69`, `24fc3ae`, and `1088292` close durable Windows
  recovery, exact small-fixture path, daemon ownership, and environment-shape
  audit findings one at a time.
- `7491b4f` makes every self-test assertion propagate failure so mutations
  cannot silently pass under Bash errexit context.

## Tests and guard proof

- Bash 3.2 syntax and the offline harness self-test pass. An xtrace audit shows
  the self-test executes no SSH command.
- The analyzer compiles, parses under Python 3.9 grammar, and passes all 74
  synthetic tests.
- Analyzer mutations for evidence relocation, source/active/archive binding,
  socket attachment/write/receive/stop order, and policy replay turn the
  corresponding tests red; exact restoration returns green.
- Harness mutations that remove the flushed Windows swap intent, allow
  no-intent recovery to touch the active runtime, omit exact small-fixture
  guards, dispatch teardown to both endpoints, duplicate environment fields,
  or restore a production fixture defect turn self-test red; exact restoration
  returns green.
- A final independent read-only audit at `7491b4f` found the exact 96-arm
  schedule and path parity sound, all four prior audit findings closed, no
  deletion/overwrite/name-wide kill, fail-closed ownership and restoration,
  and a clean unchanged worktree.
- Full local gates pass: formatting, strict workspace clippy, 1,532 workspace
  tests with 2 ignored, documentation checks, and diff checks.
- Generated Windows prepare/restoration PowerShell was sent over SSH only to
  PowerShell's in-memory parser. It parsed cleanly; no generated command was
  executed and no endpoint file, process, daemon, or staging path changed.

## Known gaps

- Round-one Claude Fable 5/max openreview found seven admitted bounded fixes.
  The implementation is not accepted until they land one per commit and a
  fresh structured exact-base/head whole-change review passes the repo's
  fail-closed guard gate.
- No endpoint staging, daemon launch, live transfer, ldt-4 evidence, or hardware
  ADD/REMOVE claim exists for this candidate. Hosted Windows CI also remains
  unobserved.
- Some fixed fixtures may finish before the tuner has a useful live sample or
  resize opportunity. The analyzer explicitly marks a missing sample at or
  after the first tuner tick `REVIEW_REQUIRED`; deterministic ldt-2 guards, not
  a short hardware arm, remain the proof that ADD and REMOVE work.
- Client execution has no per-arm deadline. Process identity and signal cleanup
  are scoped, and interruption retains a void session, but an unresponsive arm
  can require operator interruption rather than timing out automatically.

## Reviewer comments

Claude Fable 5/max reviewed exact range
`e41b87173f2073a9b6694a62813eddc14a7844ad..0e4872162f09120188404d5d23448ff3a6298133`
under the neutral best-way question. The result was schema-valid with exact
SHAs, seven candidates, and `guard_confirmed=true`. Intake admitted
`ldt-4-r1-f1`, `f2`, `f3`, the narrowed comment-truth `f5a`, `f6`, and split
`f7a`/`f7b`; original analyzer candidates `f4` and `f5` were declined. Full
evidence and rationale: `.review/results/ldt-4-harness-r1.claude-verdict.md`.
