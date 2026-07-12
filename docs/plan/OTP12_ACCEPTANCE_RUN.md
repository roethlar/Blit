# otp-12 — symmetric-rig acceptance run (design)

**Status**: Active (owner "yes to both", 2026-07-12 — the doc's only open
question was ruled by D-2026-07-12-1; design codex round closed at
`92e1d51`. The zoey RIG RUN still requires its own fresh owner go at run
time — standing STATE rule.)
**Created**: 2026-07-12
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
**Contract**: `docs/TRANSFER_SESSION.md` (unchanged — this slice adds NO code
and NO wire surface; it is harness scripts + rig runs + committed evidence).
**Governs**: execution proceeds 12a → 12b → 12c → 12d, each commit through the
codex loop (D-2026-07-04-1); rig availability may reorder 12a–12c (the otp-2
precedent, REVIEW.md §otp). The verdict WALK is otp-13 and belongs to the
owner — this slice computes and commits the matrix; it declares nothing
(Earned Practices: checkpoints are owner-only).

## Why this doc

otp-12 is the plan's acceptance-evidence slice: rerun the otp-2 matrix on the
unified path — initiator/verb invariance A/B within noise AND every cell ≤
the better old direction + noise (`ONE_TRANSFER_PATH.md` slice 12, acceptance
criteria 1–2). Three rigs, three different measurement obligations, two of
them first-of-kind (reverse-initiator arms; delegated remote↔remote cells).
Every methodology rule below that is not new is inherited verbatim from the
reviewed otp-2/otp-2w harnesses — this doc cites rather than restates their
rationale (`docs/bench/otp2-baseline-2026-07-10/README.md` §Methodology
findings, `docs/bench/otp2w-baseline-2026-07-10/README.md` §Timing-overhead
correction).

## What otp-12 must produce (plan anchors)

1. **Invariance matrix** (criterion 1): per data direction × workload
   (large / 10k-small / mixed), wall time initiating from end A vs end B —
   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
   better of that cell's two old directions + noise (±10%), against the
   recorded old-path baselines, confirmed by interleaved same-session
   old-vs-new A/B (the otp-2 README's standing prescription for this rig
   class).
3. **Delegated cells** (owner rig designation, 2026-07-10, STATE Blocked):
   remote↔remote on the Windows box + skippy — the delegated trigger must
   not cost wall time vs the same session driven directly.

## Current state (verified at HEAD `ce36da3`)

Load-bearing facts, with evidence:

- One `copy` verb drives everything; a remote endpoint is `host:/module/path`
  or `host:port:/module/path`, default port 9031
  (`crates/blit-core/src/remote/endpoint.rs:28,64-91,165-195`).
- Carrier switch: default = TCP data plane (responder binds an EPHEMERAL
  listener, initiator dials — `transfer_session/data_plane.rs:129,204`;
  grant present ⇒ TCP, `transfer_session/mod.rs:805`); `--force-grpc`
  forces the in-stream carrier (`blit-cli/src/cli.rs:317-319`), and rides
  the delegated spec too (`proto/blit.proto:408`,
  `blit-daemon/src/service/delegated_pull.rs:334`).
- Remote↔remote is delegated-only (D-2026-07-11-1): `blit copy A:/m/p B:/m/q`
  always calls `DelegatedPull` on the **destination** daemon, which initiates
  the one session against the source daemon in the DESTINATION role
  (`blit-app/src/transfers/remote.rs:462-484`,
  `delegated_pull.rs:312-327,352`). There is no push-shaped delegated form.
  The RPC carries trigger + progress only (no-payload proof recorded at
  otp-10: `cli_data_plane_outbound_bytes == 0`).
- Delegation gate: destination daemon config `[delegation]
  allow_delegated_pull = true` + `allowed_source_hosts` allowlist
  (`blit-daemon/src/runtime.rs:139-145`); per-module `delegation_allowed`.
- Same-build handshake (D-2026-07-05-2): first frame both directions; exact
  `build_id` + `contract_version` equality or `BuildMismatch` refusal
  (`transfer_session/mod.rs:660-701`). Dirty builds mint distinct ids
  (`blit-core/build.rs:28-97`) — **all arms must be clean-tree builds; arms
  swap BOTH ends together (matched pairs)**.
- Old-arm binaries route the OLD drivers: `e757dcc` (zoey pair, staged in
  `blit-temp/` — `.agents/machines.md`) and `0f922de` (Windows pair, checkout
  detached there) both PREDATE the verb cutover (`0fbc966`), so their verbs
  still call `Push`/`PullSync` — they are genuine old-path arms. Verified by
  ancestry + `git ls-tree` (old drivers present at both shas).
- July skippy binaries (`/mnt/generic-pool/video/blit-bin/`) are REV4-era:
  unknown commit, no `Transfer` RPC, no handshake — **unusable for any
  otp-12 arm**; skippy gets fresh staging (D6).
- Baselines on record: `docs/bench/otp2-baseline-2026-07-10/` (zoey,
  per-direction only — hardware-asymmetric endpoints, D-2026-07-05-1
  corollary) and `docs/bench/otp2w-baseline-2026-07-10/` (Mac↔Windows, the
  owner-designated cross-direction rig).
- Flags a harness touches that changed since the old scripts: none — `copy`,
  `--yes`, `--force-grpc` are name-stable; `--diagnostics-counter-file` is a
  global flag preceding the subcommand.
- SizeMtime safe-skip delta (STATE open question) cannot affect these cells:
  every timed run writes into a fresh, never-seen destination, so no
  same-size/dest-newer candidates exist in any arm.

## Rigs and what each anchors

| rig | endpoints | anchors | why scoped so |
|-----|-----------|---------|---------------|
| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |

Contingency: skippy is available for Mac↔Linux cells "if needed" (owner) —
used only if zoey is unavailable (it was under maintenance 2026-07-11); such
a substitution records fresh baselines and is per-direction only.

## Design decisions

### D1 — matched-pair interleaved A/B (build identity is the axis)

Each comparison interleaves arms in the deterministic counterbalanced
order `A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs — each arm leads half the
pairs, so arm never confounds with within-pair position on the stateful
rigs; pre-registered, no randomness, codex design F5) with `RUNS=4` per
arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
`delegated` (rig D). Interleaving is the verdict method, not a nicety:
zoey's tiered write path never fully stops being stateful (otp-2 README
§Run-to-run stability) and interleaving holds Defender state equal across
arms on Windows (otp-2w README §Readings). Arm swap = stop one daemon
pair, start the other (PID-scoped, stale-refusal preserved), always
outside the timed window. Old arms exist only where an old baseline exists
(rigs Z and W); invariance and delegated arms are new-build only — the old
path is known non-invariant (the plan's founding defect) and has no
delegated baseline.

Build discipline: one clean commit per arm. New arm = the run commit (same
sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
detached worktree (`git worktree add --detach` — the otp-11a precedent) and
stashed at `~/blit-bench-work/bins/blit-<sha>`. The handshake enforces new-
arm pair identity at the first frame; old arms predate it, so old-arm
provenance rests on the staging record (`.agents/machines.md`) plus a
sha256 manifest recorded in the evidence (Known gaps).

### D2 — verdict arithmetic (what the evidence computes; the owner declares)

All statistics per the recorded baselines: integer ms; median of 4, even
count = floor of the mean of the middle two; per-cell spread
`(max−min)/min` recorded.

**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
undrained pre-run window VOIDS its whole interleave pair (both arms at
that counterbalance position); the pair is re-run — appended at the same
position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
pair attempts per comparison. At the cap the cell is recorded
`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
a median over fewer than RUNS valid runs.

- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
  interleaved old arm AND the committed 2026-07-10 baseline median for
  that cell (codex design F2: the fixed pre-cutover bar must not be
  loosened by a slower old rerun). A cell passing same-session but
  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
  gets one pre-registered fresh-session re-run; a persisting drift stands
  as a recorded failure for the otp-13 walk. **Every unified arm of a
  data direction — both initiators on rig W, both blocks — must meet
  these bars independently** (codex design F3: the invariance ratio is an
  additional constraint, never a substitute ceiling — otherwise
  tolerances compound to 1.21×).
- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
  carrier × data direction, arm A (Mac-initiated) vs arm B
  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
  rows; grpc rows are recorded, same bar, labeled secondary.
- **Delegated parity (rig D, hard bar)**: per fixture × direction,
  `max(delegated, direct)/min ≤ 1.10`.
- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
  each unified direction's median vs
  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
  while passing per-direction converge-up AND invariance, the evidence
  additionally computes the **platform-residue discriminator** the otp-2w
  README pre-registered: compare the old arm's direction gap
  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
  session. Gap unchanged ⇒ the residue exists identically without blit's
  old choreography and lands on the platform write path (NTFS/Defender vs
  APFS — the plan's Non-goals: different hardware need not perform
  identically); gap closed ⇒ the code was the cost and the bar is met. The
  README records BOTH computations per cell; a discriminator-attributed
  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
  the otp-13 walk reviews the recorded numbers.

Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
interleaved in a fresh session; both sessions are committed.
**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
text defined the trigger but not which session governs): the RUNS=8
escalation session's medians govern the escalated comparison's combined
outcome — more data where noise or a straddle made RUNS=4 undecidable is
the escalation's entire purpose. The RUNS=4 rows stay committed and
visible; the otp-13 walk sees both sessions.**

### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)

For a FIXED data direction the two initiators are:

- **Mac→Windows**: arm A = Mac client pushes
  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
  arm B = Windows client pulls
  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
- **Windows→Mac**: arm A = Mac client pulls (staged
  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows
  client pushes the same staged tree as a local path
  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).

New plumbing this requires, each keyed by ROLE not verb:

1. **A daemon on the Mac** (new build only): config written like the rig
   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   itself** — the module exports the exact fixture trees arm A pushes,
   so both initiators read the same physical inodes; no fixture copy or
   move on the Mac (codex design F6)), local launch, pid file,
   stale-refusal, PID-scoped teardown. macOS application firewall must
   admit `blit-daemon` — gated by a preflight smoke transfer from
   Windows, not assumed.
2. **A Windows client** (`blit.exe`, new build, built natively alongside
   the daemon). Its timed window is measured ON Windows —
   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   self-timed pattern (README §Timing-overhead correction); the ssh
   round-trip cost stays outside the window by construction.
3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   walk. Cold caches both ends before every run (purge / standby-purge);
   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   receives; the Mac side has no drain equivalent — recorded decision: Mac
   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   pull cells did).

Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
cannot masquerade as an initiator effect.

### D4 — delegated cells = delegated-vs-direct parity (rig D)

Per data direction, the delegated arm and the direct arm drive the SAME
session code with the same roles on the same endpoints; the only deltas are
who spawns the initiator (daemon vs CLI) and the trigger/progress relay:

- **skippy→Windows**: delegated = Mac runs
  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
  (Windows daemon initiates, DESTINATION role); direct = Windows client
  pulls the same source to the same disk
  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
  daemon initiates); direct = skippy client pulls from the Windows daemon
  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).

Timing: the delegated arm is timed on the Mac around the CLI invocation
(the CLI blocks until the relayed Summary), plus the destination's
self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
(that is the honest end-to-end cost of delegation; on this LAN the trigger
is sub-ms against multi-second cells). The direct arm is self-timed on the
initiating host plus the same flush. Destination flush: Windows ⇒
`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
window loop with a device-regex knob).

Carrier: TCP is the verdict carrier; one secondary grpc pair
(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
selection reads `SessionOpen.in_stream_bytes`/policy, never role or
initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
measured properly on rig W.

Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
`allowed_source_hosts` naming the peer (each is destination in one
direction); bench modules writable, `delegation_allowed` not narrowed.

### D5 — three self-contained scripts; the frozen baselines stay frozen

`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
precedent: duplicate the shape, don't refactor recorded evidence;
`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
old scripts, both recorded sharp edges:

- **Exit codes are checked**: the old harnesses swallow the blit exit code
  inside the timed window; otp-12 records it per run (`exit` column) and a
  nonzero exit voids the interleave pair per the D2 valid-run rule — a
  failed transfer must never contribute a time.
- **Multi-token flags ride an array**, not an unquoted scalar.

CSV schema (all rigs):
`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
(`valid` = the PAIR's fate under the D2 valid-run rule — an
individually-clean run whose partner voided reads `no`; amended at the
12a harness slice)
`summary.csv`:
`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
(medians over valid runs only — the D2 valid-run rule)
`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
reference CSVs; corrected at the 12a review, codex F9),
`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
self-adjudicated; added at the 12b harness slice), `arm` ∈
`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
`initiator` = host name, `kind` ∈ `converge|invariance|delegated|cross`.
Verdict outcome vocabulary: per-reference rows carry `PASS|FAIL`; a
comparison's `combined` row carries the registered D2 set
(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
nothing else is legal, and a missing committed-reference row aborts the
verdict pass (fail closed).

Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
`dd` block-size spelling handled per host), staged untimed; pull sources
shared across arms (bytes are bytes — recorded explicitly); every timed
destination is fresh and never-seen (`SESSION_TAG` + arm + run in the
path).

New env knobs: `MAC_HOST` (the Mac's 10 GbE IP — required, no default),
`MAC_MODULE_ROOT` (default `$MAC_WORK` — see D3), `SKIPPY_SSH` (default
`admin@skippy`), `SKIPPY_HOST`, `SKIPPY_BIN` (default
`/mnt/generic-pool/video/blit-bin`), `SKIPPY_DISK_REGEX`,
`OLD_SHA_ZOEY=e757dcc`, `OLD_SHA_WIN=0f922de`.

Verification entry point for harness commits (no crates/proto touched; the
cargo gates don't exercise bash): `bash -n` on each script + shellcheck
where installed + `bash scripts/agent/check-docs.sh` + the codex review;
the methodology itself is verified by the probe/recorded-run discipline
(otp-2 precedent) and each script supports `PREFLIGHT_ONLY=1` (run every
preflight check and exit before fixtures).

### D6 — staging per host

| host | old arm | new arm |
|------|---------|---------|
| Mac | rebuild client at the pinned sha in a detached worktree → `~/blit-bench-work/bins/blit-<sha>` | `cargo build --release` at the run commit |
| zoey | clean `e757dcc` zigbuild staged as `blit-daemon-e757dcc` — the 2026-07-10 staging at `blit-daemon` FAILED provenance (a dirty `731023b` build; correction note in the otp-2 README) and is left untouched as the otp-2 artifact | `cargo zigbuild --release --target aarch64-unknown-linux-musl` → staged beside as `blit-daemon-<sha>` (never overwrite); everything stays inside `blit-temp/` |
| Windows | copy the detached-checkout exes ASIDE first (`D:\blit-test\bins\0f922de\`) before any checkout movement | fresh git bundle (pushes are owner-gated; origin lags at `6d37a22`) → checkout run commit → native `cargo build --release` (daemon AND `blit.exe` client) → `D:\blit-test\bins\<sha>\` |
| skippy | none (no old baseline; July binaries unusable) | `cargo zigbuild --release --target x86_64-unknown-linux-musl` (static — sidesteps the recorded glibc 2.36 ceiling) → `$SKIPPY_BIN/bins/<sha>/` (pool paths are exec-friendly; `/tmp` and `/home` are noexec) — `blit` + `blit-daemon` |

Windows daemon-swap mechanics: the active arm's exe is COPIED to the fixed
path `D:\blit-test\bins\active\blit-daemon.exe` and launched from there —
one program-scoped firewall rule total (the rule is exe-path-scoped;
sha-named dirs keep provenance, the copy log records each swap). Launch
stays WMI `Win32_Process.Create` + stale-refusal + PID-scoped teardown
(otp-2w README §Host plumbing). A staging manifest (sha256 of every binary
on every host) is recorded in each evidence README.

### D7 — matrix size and session budget

| rig | comparisons | timed runs | est. wall |
|-----|------------:|-----------:|----------:|
| Z converge-up | 12 (3 fixtures × 2 dirs × 2 carriers) | 96 | 1.5–2.5 h (drains dominate) |
| W converge-up | 12 | 96 | ~1.5 h |
| W invariance | 12 (3 × 2 dirs × 2 carriers, new-only) | 96 | ~1.5 h |
| D delegated | 6 (3 × 2 dirs, TCP) + 1 grpc smoke | 56 | ~1 h |

Each rig session needs the owner's machines on and otherwise idle; sessions
are independent and may run on different days (each records its own rig
state).

## Staging (sub-slices; each commit through the codex loop)

- **otp-12a — rig Z**: `bench_otp12_zoey.sh` (harness commit; codex; fix) →
  recorded run → `docs/bench/otp12-zoey-<date>/README.md` + CSVs (evidence
  commit; codex; fix). Preflight gates: staged old pair present; new musl
  daemon staged beside it; **fresh owner go for daemon runs on zoey**
  (standing STATE rule) and zoey out of maintenance.
- **otp-12b — rig W**: `bench_otp12_win.sh` covering converge-up block +
  invariance block; same two-commit shape. Preflight gates: bundle
  delivered + old exes copied aside + new native build (daemon + client);
  Mac daemon smoke from Windows (firewall).
- **otp-12c — rig D**: `bench_otp12_delegated.sh`; same shape. Preflight
  gates: fresh skippy staging on the pool; `sudo -n` drop_caches on skippy;
  delegation config both daemons; reachability smokes in both directions
  (control port + a 1-file TCP-carrier transfer — the data plane binds
  ephemeral ports, so the smoke IS the firewall test).
- **otp-12d — assembly**: `docs/bench/otp12-acceptance-<date>/README.md` —
  the plan-level verdict matrix assembling every comparison row
  criterion-by-criterion (the artifact otp-13 walks). Docs-only commit.
  The plan's acceptance-criteria checkboxes are NOT flipped here — that
  is the otp-13 owner walk (codex design F4; checkpoints are owner-only).

Rig order may flex with availability; 12d requires all three.

## Evidence layout

`docs/bench/otp12-{zoey,win,delegated}-<date>/` each carry: `README.md`
(otp-2 README shape: Status/Scope, Build with all arm shas, Rig, results
tables, stability, methodology deltas, reproduction), `runs.csv`,
`summary.csv`, `verdicts.csv`, `drain-outcomes.txt`, `staging-manifest.txt`
(sha256 per binary per host). `docs/bench/otp12-acceptance-<date>/README.md`
is the assembly. Raw session logs stay under `logs/` (untracked) as usual.

## Known gaps / risks

- **No rig is truly fs-identical.** The plan's "symmetric rig" is
  instantiated by the owner-designated closest-spec pair; rig W's two
  directions still land on different OS write paths (APFS vs NTFS +
  Defender at its normal state). D2's discriminator computation is the
  pre-registered, evidence-backed handling; a platform-residue cell counts
  as satisfied per D-2026-07-12-1.
- **Old-arm provenance is a staging record, not a handshake** (old paths
  predate it). Mitigated by machines.md provenance + the sha256 manifest;
  accepted residual risk.
- **First-of-kind surfaces**: a daemon on the Mac (application firewall
  unknown until the smoke) and a client on skippy (musl-static, untested
  there — the zoey zigbuild recipe retargeted). Both are preflight-gated;
  failures block the affected block only.
- **zoey availability**: under maintenance 2026-07-11; daemon runs there
  need a fresh owner go regardless (STATE rule).
- **Delegated arm includes trigger/relay overhead by design** — recorded,
  expected sub-ms on this LAN; if it ever dominates a cell, that IS a
  finding, not noise.
- **Suite/test count**: untouched — no crates/proto changes anywhere in
  otp-12; the ≥1483 floor stands at 1484 from otp-11b.

## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)

- **Q1 — cross-direction residue on rig W**: RESOLVED "yes" — a cell that
  beats its own old direction, is initiator-invariant, and misses the
  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
  platform write-path residue (same gap in the old arm, same session)
  **counts as satisfying the cross-direction half of criterion 2**
  (D-2026-07-12-1). The evidence still records both computations per
  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
  is not a blocker.
