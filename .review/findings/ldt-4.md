# ldt-4 — quiet rig-W evidence harness and analyzer

**Slice**: `LIVE_DIAL_TUNING` ldt-4. Measure the accepted adaptive controller
on rig-W `q`↔`netwatch-01` with identical physical paths under both initiator
layouts and both byte directions.

**Status**: Closed as a pre-release evidence slice by D-2026-07-22-1. Exact
`96a4e3b` completed all 96 fixed arms; later supplements exercised real resize.
The first complete horizon session is valid after corrected-analyzer
reanalysis, its fresh repeat was redundant, and causal tuning is post-release.
Canonical session ledger: `docs/bench/ldt4-evidence-audit-2026-07-22/`.

**Branch**: `master`

**Implementation range**:
`e41b87173f2073a9b6694a62813eddc14a7844ad..d53b5fdd3b85fd61f377de917e16ba19aa65d137`

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
- `b9b8080` closes `ldt-4-live-f2`: make the two generated Windows daemon log
  paths explicit PowerShell array elements and pin their exact source form.
- `a39f0c5` closes `ldt-4-live-f3`: recognize only a missing-start state with
  zero PIDs, no post-launch markers, and a closed port before returning clean.
- `d53b5fd` closes `ldt-4-live-f4`: make both generated Windows responder
  startup concatenations explicit PowerShell array elements in launch and identity
  commands.

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
- At `b9b8080`, restoring the exact live-failing unparenthesized PowerShell log
  loop turns the offline self-test red. Exact restoration passes the 96-arm
  self-test, all 75 analyzer tests, formatting, strict clippy, the full
  workspace suite, documentation checks, and diff checks.
- At `a39f0c5`, disabling the missing-start branch or weakening the durable
  pre-launch flush independently turns the offline self-test red. Exact
  restoration passes; the current function also returned the exact no-launch
  result against retained Windows arm `ldt4-001` through SSH without a
  registered session or runtime file write.

## Known gaps

- The Windows→q ADD/REMOVE split is confounded with cold/warm source order.
- Fixed-cell q→Windows ratios 1.197 and 1.131 remain performance findings.
- Both are post-release under D-2026-07-22-1; no controller change or further
  hardware run is authorized before release.

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

Claude Fable 5/max then reviewed exact range
`4e0fdc307ba26e81f8532cd191089fa291c7f1aa..5a2265e202a4ca5b4bbf08f8b58b7ff59ff75a8b`
under the same neutral question. The schema-valid result was clean with no
findings, exact SHAs, and `guard_confirmed=true`. Its independent mutation
restored the old `mv -n` promotion; the offline self-test turned red, exact
restoration passed all 96 no-SSH arms, and the detached worktree ended clean.
Full record: `.review/results/ldt-4-canonical-r2.claude-verdict.md`.

The next attached launch used that exact reviewed head and retained tag
`ldt4-20260717T052509Z-5a2265e202a4`. Canonical fixture staging and
cross-endpoint manifests passed, but PowerShell treated the intended
`daemon.out` and `daemon.err` array as one space-joined filename. No timing row
was accepted. Teardown then required the not-yet-created `start.cmd`, exposing
separate `ldt-4-live-f3`. Read-only post-run checks proved both ports closed,
no session process remained, the active Windows daemon was restored to durable
prior SHA `1510d8d0…0512`, and the exact staged daemon remains retained under
the session-specific tested name. `ldt-4-live-f2` is fixed at `b9b8080` and
`ldt-4-live-f3` at `a39f0c5`, each with independent red/restored-green guards.
The owner placed further formal Fable openreviews on capacity hold and allowed
tactical code review by Grok or Claude Opus 4.8 instead.

Grok 4.5/high then tactically reviewed exact range
`5a2265e202a4ca5b4bbf08f8b58b7ff59ff75a8b..a39f0c570191d65f197e4ab58eade375ec52e6d6`
in a retained clean detached worktree. The same session resumed after an
initial process cancellation and returned `clean` with no findings after Bash
3.2 self-test, in-memory PowerShell array/parser probes, ordering-window audit,
and launched-state ownership verification. This advisory result is not formal
openreview acceptance. Record:
`.review/results/ldt-4-live-fixes-r1.grok-verdict.md`.

Exact `a39f0c5` was then bundled and staged without replacing the retained
bundle or checkout. q's Bash 3.2 self-test passed, all three fixture manifests
matched across endpoints, and the live environment gate passed. Retained
session `ldt4-20260717T062334Z-a39f0c570191` voided on its first arm before
daemon launch or timing because two unparenthesized concatenations made
PowerShell emit a 20-item `start.cmd` rather than 12 intact lines. Cleanup
closed both ports, left no session process, restored the exact prior Windows
daemon, and retained the tested daemon and evidence. `ldt-4-live-f4` fixes both
array elements and is mutation-proved at `d53b5fd`; tactical code review and
fresh additive staging remain.
