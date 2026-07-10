# otp-2 — per-direction disk-to-disk baseline (harness + rig, no production code)

> Post-review note (codex F1, upheld): this rig's endpoints are
> hardware-asymmetric, so per D-2026-07-05-1 the recorded cells anchor
> PER-DIRECTION converge-up only; the plan's cross-direction bar is an
> owner question in STATE. The title below and any "symmetric baseline"
> phrasing in this doc predate that adjudication — the verdict file and
> the evidence README are authoritative.

**What**: Plan slice otp-2 — correct the sf-1 benchmark methodology
and record the OLD paths' per-cell, per-direction baseline on the
rig the owner designated (Mac ↔ zoey over Thunderbolt 10GbE), as the
converge-up reference for the otp-12 acceptance run. Rig access,
scope confinement (zoey `blit-temp` only), and the machine pairing
were owner-authorized in-session.

**Approach**:

- `scripts/bench_otp2_baseline.sh` — the corrected harness. Verdict
  cells are real-disk-to-real-disk (client APFS SSD ↔ daemon pool;
  never `/tmp`), cold caches both ends before every run (macOS
  `purge` via a scoped NOPASSWD rule + Linux `drop_caches`),
  durable-at-destination timed windows (transfer + destination
  `sync`), a daemon-host pool DRAIN before every run (stateful NVMe
  write tier), fresh never-seen destinations per run, MEDIAN of 4 as
  the cell statistic, no competitor rows (D-2026-07-04-4). Matrix:
  {large, small, mixed} × {push, pull} × {tcp, grpc} = 12 cells,
  fixture shapes kept from sf-1 for continuity.
- Both ends built from the SAME commit (`731023b`): macOS arm64
  client + static aarch64-musl daemon (zig cross-build — the toolchain
  recipe the July session proved but never recorded is now embodied in
  a reproducible command: `cargo zigbuild --release --target
  aarch64-unknown-linux-musl -p blit-daemon -p blit-cli`).
- `docs/bench/otp2-baseline-2026-07-10/` — README (rig, medians,
  methodology findings, otp-12 prescriptions), `summary.csv`,
  `runs.csv`, plus the two PROBE runs kept as evidence for why the
  harness has sync + drain (they show the 4–8× cache lottery and the
  2.7→13.4 s stateful-tier ascent respectively).
- The July tmpfs/warm data is re-labeled wire-reference only (README
  section; the plan's explicit sub-deliverable).

**Files**:

- `scripts/bench_otp2_baseline.sh` — NEW.
- `docs/bench/otp2-baseline-2026-07-10/{README.md,summary.csv,runs.csv,probe1-no-sync-runs.csv,probe2-no-drain-runs.csv}` — NEW.
- `docs/STATE.md`, `DEVLOG.md` — slice close + the new open question.

**Tests**: none (no production code; the plan marks this slice
"harness + rig"). Verification = the recorded runs themselves: 12/12
cells completed, byte-identical smoke round-trip pre-run, TCP < gRPC
in all cells, per-file costs consistent in shape with the July
diagnosis. The harness is idempotent and re-runnable (reproduction
block in the README).

**Known gaps**:

- **Push-cell residual spread ±10–20%** (one outlier per ~4 runs)
  even with drain+sync — inherent to the pool's tiered write path.
  Mitigation recorded as an otp-12 prescription: interleaved
  same-session A/B for push verdicts; the `731023b` binaries stay
  staged on zoey for that.
- **OPEN QUESTION routed to the owner** (STATE): the plan's
  cross-direction acceptance bar ("every cell ≤ the better of that
  cell's two old directions + noise") presupposes hardware-symmetric
  endpoints; this rig's write ends are asymmetric (SSD vs pool), so
  pull beats push ~1.6–1.7× in every cell for physics reasons.
  Proposed reading for hardware-asymmetric rigs: per-direction
  converge-up only. Not adjudicated here — plan changes are not this
  slice's call.
- Windows box + TrueNAS (owner-offered) are reserved for
  remote↔remote (delegated) testing per the owner's instruction —
  recorded in STATE; not part of this baseline.
- The harness assumes the daemon host exposes `/proc/diskstats` and
  the client is macOS (purge); it is rig-specific by design and says
  so.
