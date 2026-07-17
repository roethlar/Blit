# ldt-4 — quiet rig-W evidence harness and analyzer

**Slice**: `LIVE_DIAL_TUNING` ldt-4. Measure the accepted adaptive controller
on rig-W `q`↔`netwatch-01` with identical physical paths under both initiator
layouts and both byte directions.

**Status**: Fixed-SHA re-review candidate. The original harness was accepted at
exact reviewed head `4e0fdc3`; its first launch voided before any arm or Windows
runtime swap on unequal fixture content. `b0c6ce3` established canonical stable
fixtures, and canonical-fixture Fable round one reviewed exact `ef48920` and
admitted two Low staging corrections. `1302b90` and `fdf7b37` close them with
mutation proof, and full local gates pass. A neutral exact-head review remains
before another live launch.

**Branch**: `master`

**Implementation range**:
`e41b87173f2073a9b6694a62813eddc14a7844ad..fdf7b3771c00c950ca40fb7e9904a91c89f8a72d`

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
- `598fa59`, `096304b`, and `0efa4e0` close round-one Windows fetch framing,
  ambiguous hard-crash baseline, and padded-PID recovery findings;
  `92a5a89` closes the pre-review Bash-to-PowerShell generation defect exposed
  by the required parser check.
- `f67ef75`, `80ecaad`, `efc796a`, and `f470b06` correct the accepted-resize
  contract comment, cover every independent replay reason, reject duplicate
  trace keys, and require exact blocked-ratio recomputation.
- `b0c6ce3` closes live finding `ldt-4-live-f1`: stage the canonical Windows
  large and mixed fixtures through a retained per-session incoming namespace,
  validate exact content before no-clobber promotion, and register the same
  stable q paths in the harness and analyzer.
- `1302b90` closes `ldt-4-r3-f1`: replace `mv -n` promotion with the existing
  exclusive atomic rename primitive.
- `fdf7b37` closes `ldt-4-r3-f2`: reject wrong canonical shape before copy and
  derive the staging capacity reservation from that validated manifest.

## Tests and guard proof

- Bash 3.2 syntax and the offline harness self-test pass. An xtrace audit shows
  the self-test executes no SSH command.
- The analyzer compiles, parses under Python 3.9 grammar, and passes all 75
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
- Exact-current-head local gates pass: formatting, strict workspace clippy,
  1,532 workspace tests with 2 ignored, 30 focused release dial tests, all 75
  analyzer tests, documentation checks, and diff checks.
- Generated Windows prepare/restoration PowerShell was sent over SSH only to
  PowerShell's in-memory parser. After it exposed and the coder fixed f1's
  shell-generation quoting, exact fetch, prepare, and normal restoration all
  parsed cleanly; no generated command was executed and no endpoint file,
  process, daemon, or staging path changed.
- The first exact `4e0fdc3` live launch stopped in fixture preflight with zero
  arms and no Windows runtime swap after proving the two 1 GiB fixtures had
  unequal content. The void evidence and both endpoint session namespaces are
  retained under tag `ldt4-20260717T032327Z-4e0fdc307ba2`.
- At `b0c6ce3`, reverting the stable q large path, copying directly to the
  final path, or restoring the analyzer's old q source registry turns its
  corresponding guard red. Exact restoration passes Bash syntax, the 96-arm
  no-SSH self-test, all 75 analyzer tests, formatting, strict workspace
  clippy, and the full workspace suite.
- At `1302b90` and `fdf7b37`, restoring `mv -n`, restoring post-copy canonical
  shape validation, or removing only the low-space refusal turns the offline
  self-test red. Exact restoration passes all local gates listed above.

## Known gaps

- Exact product and harness artifacts are staged additively and one preflight
  void is retained, but no live arm, transfer datum, hardware ADD/REMOVE claim,
  or adaptive verdict exists. Hosted Windows CI also remains unobserved.
- Canonical-fixture Fable round one admitted two Low corrections: use the
  existing exclusive atomic rename helper for stable promotion, and reject a
  canonical manifest's wrong shape before copying it. Both corrections are
  fixed and mutation-proved with full local gates green; a fresh exact-head
  review remains.
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

Claude Fable 5/max then re-reviewed exact range
`e41b87173f2073a9b6694a62813eddc14a7844ad..4e0fdc307ba26e81f8532cd191089fa291c7f1aa`
under the same neutral question. The schema-valid result was clean with no
findings, exact SHAs, and `guard_confirmed=true`; its threshold-direction
mutation made 13 focused dial tests fail, exact restoration made all 30 pass,
and the detached worktree ended clean. Full record:
`.review/results/ldt-4-harness-r2.claude-verdict.md`.

The first accepted-harness launch exposed `ldt-4-live-f1` before any arm: the
old fixture contract relied on independently created q and Windows sources and
correctly refused their mismatch, but did not establish a canonical
byte-identical pair. Its canonical-staging correction now has two admitted Low
follow-ups; the corrected head still needs a fresh formal Fable review before
it can replace `4e0fdc3` as the exact live harness.

Claude Fable 5/max reviewed exact range
`4e0fdc307ba26e81f8532cd191089fa291c7f1aa..ef48920720b02d09a490c1c07f6acd35651aba65`
under the same neutral question. The valid mechanically constrained retry
returned two Low candidates with exact SHAs and `guard_confirmed=true`; its
path-registry mutation made the offline harness self-test fail, exact
restoration passed all 96 no-SSH arms, and the worktree ended clean. Intake
admitted `ldt-4-r3-f1` (exclusive atomic fixture promotion) and
`ldt-4-r3-f2` (pre-copy canonical shape/space validation). Full dispatch,
safety, guard, and intake record:
`.review/results/ldt-4-canonical-r1.claude-verdict.md`.
