# ldt-4-live-f13 — keep SOURCE admission alive across tuner ticks

**Severity**: MEDIUM — the f12 byte drain lasted up to 20.7 seconds, but its
five payloads queued in milliseconds and stopped the tuner before any sample,
so ldt-4 still has no live membership transition.
**Status**: Fixed, reviewed, and staged; first live run voided by analyzer f14.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `af13fdb` (candidate); `a0c3e3f` (review guard)

## Evidence

Exact reviewed/staged harness `04e80082e12ce9836eda43afc70fb3b2d0eb07c9`
completed all four 5 GiB arms in retained session
`ldt4-20260722T001611Z-04e80082e12c`. There is no session void, every payload
manifest is exact, and Windows restored normally and byte-for-byte. The copied
evidence and independently reproduced analysis live at
`docs/bench/ldt4-rigw-sustained-2026-07-22/`.

The analyzer returned `REVIEW_REQUIRED`: arm review 4, decision review 0,
performance review 0. Every arm stayed at floor = peak = final = 4 with zero
tuner samples. SOURCE received terminal demand in 3.1–5.2 ms and sealed
membership in 3.3–5.4 ms, while its data plane drained for 4.3–20.6 seconds.
Exact code stops the tuner once terminal demand is known and every payload has
entered the bounded pipeline; drain duration after that point is irrelevant.

The initial live pipeline admits at most 25 whole-file payloads before SOURCE
must wait: four in the SOURCE mpsc, one held by its forwarder, 16 in the shared
`prefetch(4) * streams(4)` queue, and four owned by active workers. Forty 1 GiB
files therefore require at least 15 GiB to finish before terminal admission.
Even the physical 10Gbase-T line ceiling needs about 12.9 seconds for that,
while the earliest clean-signal ADD is tick 7, about 3.5 seconds after sampling
starts (six cheap-dial steps and the second sustained tick).

q's internal data volume now has only about 4 GB above its required
33,000,000,000-byte retained-space floor, so another large additive destination
pair cannot land there. q also has an unused-for-ldt4, writable local
case-sensitive APFS SSD at `/Volumes/Apps`, UUID
`33BAD653-9FA1-4236-966F-5BC4B221B34F`, connected over PCI-Express with about
11.7 TB free. Initiator-layout comparisons keep the same source and destination
volume within each pair; these diagnostic arms do not grade performance.

## Predicted observable failure

Increasing only total bytes in a small number of files can again produce a long
socket drain with no tuner sample. Reusing q's internal payload root for another
large pair would either violate the retained-space floor or require deleting
prior evidence, both forbidden. Using an unpinned alternate mount could silently
turn a missing/replaced volume into writes on the system disk.

## What

Keep the valid fixed matrix and f12 evidence unchanged. Replace only the live
diagnostic fixture/root in a new exact harness: 40 distinct 1 GiB files (40 GiB)
per source, with q's staging and destination sessions on the exact pinned Apps
volume and Windows staging/sessions in fresh additive namespaces. Run the same
four counter-ordered role arms. Continue requiring accepted ADD above four in
every arm and exact material transition parity within each pair.

## Approach

- Add a fail-closed q payload-volume gate before any reservation or staging:
  exact mount point `/Volumes/Apps`, UUID
  `33BAD653-9FA1-4236-966F-5BC4B221B34F`, case-sensitive APFS, PCI-Express,
  solid-state, writable, and backed by a local `/dev/disk*` filesystem. A
  missing, renamed, read-only, symlinked, or substituted mount voids the session;
  never fall through to a directory on the system volume.
- Use fresh q roots under `/Volumes/Apps/blit-ldt4-f13/` and fresh Windows
  fixture/session names. Preserve f12's sources, destinations, sessions, and
  evidence unchanged.
- Stage 40 separate plain 1 GiB files from the already validated canonical large
  fixture on each endpoint. Require exact 40-file/42,949,672,960-byte manifests
  and byte identity across endpoints before any arm.
- Preflight each payload volume for one stable 40 GiB source, two retained 40
  GiB destinations, and the existing 33,000,000,000-byte floor: at least
  161,849,018,880 free bytes before staging. Keep q's internal-volume floor gate
  separately for the small evidence tree.
- Bind the new session to both the original fixed evidence and the exact f12
  session/final-inventory SHA-256
  `17348aaa261b936e04c104553d7b5c4bbcf008968306a29c4dea922535110eef`.
- Extend structural/analyzer tests to reject a reduced file count, old internal
  payload roots, missing volume identity/capacity gates, lost predecessor
  binding, or weakened accepted-ADD/transition-parity requirements. Mutation-
  prove the admission-horizon guard, restore it, and run full repository gates.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — pinned payload volume, fresh additive roots,
  40-file fixture, capacity and predecessor gates.
- `scripts/ldt4_rigw_analyze.py` — exact f13 fixture/predecessor contract while
  preserving fixed and f12 evidence modes.
- `scripts/ldt4_rigw_analyze_test.py` — valid f13 session and rejection tests.
- `.agents/machines.md` already records the dated q payload-volume identity and
  scope from the finding-opening commit.

## Guard proof

- The Bash 3.2 syntax check and exact four-arm no-SSH self-test pass.
- All 86 analyzer tests pass, including a valid horizon session and rejection
  guards for a changed predecessor digest, substituted payload-volume UUID,
  below-floor payload volume, and insufficient retained-destination capacity.
- Changing only production `HORIZON_FILE_COUNT=40` to `25` makes the exact
  self-test fail with `horizon fixture is not the exact registered 40-file/40
  GiB shape`; restoring `40` returns it green.
- The retained f12 evidence re-analyzes as `REVIEW_REQUIRED` with all six
  analysis outputs byte-for-byte identical to the committed copies, proving
  the new matrix mode did not change predecessor interpretation.
- `cargo fmt --all -- --check`, strict workspace clippy, the complete workspace
  test suite, and `scripts/agent/check-docs.sh` pass.

## Staging proof

- New complete-history bundle `/Users/michael/blit-ldt4-stage-a0c3e3f.bundle`
  verifies locally and on q at SHA-256
  `922a286e5fa68cbe77f3647cb82e27b13783c498668cec37006389d196ee8709`.
- New retained q checkout
  `/Users/michael/Dev/blit_v2_harness_a0c3e3f` is detached and clean at exact
  reviewed code head `a0c3e3f18afd5528c6f636ee54708f4d8d5127e9`, with 1,980 commits and no
  replacement refs.
- q-native Bash 3.2 syntax/four-arm no-SSH self-test and all 87 analyzer tests
  pass. `/Volumes/Apps/blit-ldt4-f13` remains absent; staging created no fixture,
  endpoint session, daemon, or transfer.

## Coder dispute

None.

## Known gaps

All four fresh arms completed, but `ldt-4-live-f14` owns the exact SOURCE
control-action analyzer mismatch that voided final analysis. Fix/review/restage
and one additive rerun remain. The fixed matrix's two performance findings
remain separate.

## Reviewer comments

Claude Opus 4.8/max reviewed exact
`75211b3a4725f8ae1952fa9f517cd593943e8b37..af13fdb444c94c29f9260fa710918c338d95dd5e`
in session `ec904253-4a0d-4eb9-b080-071b77fda80c`, reported
`guard_confirmed: true`, and found the committed implementation correct and
fail-closed. One Low guard gap was admitted as `ldt-4-live-f13-r1-f1`: the
analyzer's real 40-file/40-GiB tuple was hidden by synthetic fixture patching.
The separate fix is mutation-proved. The same Opus session re-reviewed exact
`af13fdb..a0c3e3f`, returned clean with no findings and
`guard_confirmed: true`, and left the detached tree exact and clean. Records:
`.review/results/ldt-4-live-f13-r1.opus-verdict.md` and
`.review/results/ldt-4-live-f13-r2.opus-verdict.md`.
