# otp-12c — the delegated rig-D session (+ a direct-path re-baseline at the cutover sha)

**Slice**: ONE_TRANSFER_PATH otp-12, sub-slice 12c per
`docs/plan/OTP12_ACCEPTANCE_RUN.md` (D1 interleave, D2 verdict
arithmetic, **D4 delegated-vs-direct parity**, D5 script/CSV shapes,
D6 staging, D7 matrix size). Range `dcbd6ea..580cc71` (6 commits).
No `crates/` or `proto/` changes anywhere in otp-12 — suite stands at
the recorded **1484**; the gate for this slice is
`bash scripts/agent/check-docs.sh` (docs/scripts only).

## What

Two recorded sessions plus the harness that produced the second:

1. **Direct-path re-baseline** (`d12534d`,
   `docs/bench/otp12c-win-2026-07-13/`). The rig-W matrix re-run with
   the new arm at the **cutover sha `f35702a`** — 12b measured
   `e21cf84`, so no committed rig-W evidence existed at the sha the
   shipped binaries embed. Harness unchanged
   (`scripts/bench_otp12_win.sh`), old arm unchanged (`0f922de`).
2. **The delegated rig-D matrix** (`68bb490`,
   `docs/bench/otp12c-delegated-2026-07-13/`), produced by the new
   harness `scripts/bench_otp12_delegated.sh` (`c26bc2d` draft +
   found-live fixes `b49413d`, `a2dea3f`).

D4 parity axis, as implemented: both arms drive the SAME session code
with the same roles over the same data plane onto the same destination
disk, with the same destination-OS-keyed flush. The only difference is
who spawns the initiator — **delegated** = the Mac CLI calls
`DelegatedPull` on the destination daemon (remote→remote is
delegated-only since D-2026-07-11-1; no payload crosses the Mac, only
control + progress relay), **direct** = the destination host's own CLI
runs the equivalent remote→local pull. The delegated arm is timed on
the Mac around the blocking CLI call, deliberately INCLUDING the
trigger RPC + relay overhead (the honest end-to-end cost of
delegation); the direct arm is self-timed on the initiating host. Both
add the same flush.

## Cells / matrix (D7)

7 comparisons × 2 arms × RUNS: `sw_tcp_{large,small,mixed}` (source
skippy → dest Windows), `ws_tcp_{large,small,mixed}` (source Windows →
dest skippy), plus the secondary carrier smoke `sw_grpc_large`.
Primary session RUNS=4 → 56 timed runs. Verdict bar (D2, delegated
parity): `max(delegated, direct) / min ≤ 1.10`, integer-exact
(`10*hi ≤ 11*lo`). TCP rows are the verdict rows; the grpc row is
computed identically and labeled secondary.

## Results (recorded; **nothing self-adjudicated**)

**Primary, RUNS=4** (session `021026`): 56 runs, 7/7 cells complete,
**0 voided pairs** → **5 PASS / 2 FAIL**:

```
sw_tcp_mixed,delegated,delegated,direct,2154,1925,1.119,1.10,FAIL
ws_tcp_large,delegated,delegated,direct,4647,4115,1.129,1.10,FAIL
```

**Confirmation, RUNS=8 on exactly those two cells** (session `031155`,
`CELLS=sw_tcp_mixed,ws_tcp_large`, identical 5-hash staging manifest):
32 runs, 0 voided → **both PASS**:

```
sw_tcp_mixed,delegated,delegated,direct,2054,1985,1.035,1.10,PASS
ws_tcp_large,delegated,delegated,direct,4093,4370,1.068,1.10,PASS
```

Both FAIL cells met **D2's pre-registered escalation trigger** — each
straddles its bar (1.119 / 1.129 vs 1.10) AND has an arm whose spread
exceeds 25% (delegated 86.0% / 55.4%) — so the RUNS=8 re-run IS that
rule firing, not an ad-hoc retry. Per D2's supersession amendment
(2026-07-12, codex otp-12a-run F2) **the RUNS=8 medians govern the
escalated comparison**; the RUNS=4 rows stay committed and visible.
**Governing outcome: rig D 7/7 PASS.**

(Corrected at the review round — codex otp-12c F2. The first draft of
this record claimed "neither session supersedes the other" on the
theory that D2's escalation amendment covered only the converge-up
rows. That was wrong: the escalation rule and its amendment sit in D2
after *all four* bar definitions and speak of "a comparison"
generically — delegated parity included. Re-interpreting a
pre-registered rule after seeing the numbers is exactly what the plan
forbids, and the error ran in my favour by letting the slice avoid a
verdict. Acceptance remains the owner's at otp-13, but the arithmetic
is pre-registered and now applied as written.)

Supporting texture, numbers only: the primary FAILs ride high
delegated-arm spread driven by slow early slots; at n=8 comparable
spread appears on the **direct** arm too (31.5% / 64.0%) —
`ws_tcp_large`'s direct median moves 4115→4370 and lands *above* the
delegated median, while its primary delegated best (3000 ms) already
beat the direct best (3870 ms).

The re-baseline session (198 runs, 24/24 cells, 3 DRAIN-TIMEOUT pairs
voided and re-run, 0 CR residue): 93 PASS / 12 FAIL /
3 FAIL-SAME-SESSION / 12 RECORDED. Material: `wm_tcp_mixed` invariance
**1.300** at the cutover sha vs 12b's 1.237 — the
TCP×mixed×destination-initiator cell did **not** wash out; the
converge losses stay in TCP×{small,mixed}×push + `pull_tcp_mixed`,
while the new arm wins the small-pull side.

## Harness — the three bugs found live (each caught by its own gates)

- **`:?` messages with apostrophes swallowed 20 downstream
  assignments** (`b49413d`). Identical in kind to otp-12b's `772cfe6`
  — the apostrophe opens a quote that runs to the next one, so
  `SKIPPY_HOST` et al. silently became part of a string. Both `:?`
  messages are now apostrophe-free.
- **macOS `$TMPDIR` blows ssh's 104-byte ControlPath limit**
  (`b49413d`) — the mux socket path exceeded the AF_UNIX cap and every
  multiplexed call fell back with a warning. Mux dir is now `/tmp`.
- **skippy's NOPASSWD grant is exactly `tee /proc/sys/vm/drop_caches`**
  (`a2dea3f`), so the generic `sudo -n sh -c 'sync; echo 3 > …'` form
  was refused and the drop silently no-op'd (`|| true`) — **runs would
  have read WARM**. The preflight probe and the drop now both go
  through the exact grant.

## Files

- `scripts/bench_otp12_delegated.sh` (new, 686 lines).
- `docs/bench/otp12c-delegated-2026-07-13/` — README + `runs.csv`,
  `summary.csv`, `verdicts.csv`, `meta.csv`, `staging-manifest.txt`,
  `drain-outcomes.txt`; the RUNS=8 confirmation under `rerun-8pair/`.
- `docs/bench/otp12c-win-2026-07-13/` — same artifact set.
- `docs/STATE.md` (12c recorded; pruned to the 200-line cap, handoff
  log rotated), `DEVLOG.md`.

## Tests / gate

Docs+scripts only — `bash scripts/agent/check-docs.sh` **OK**;
`bash -n` clean on the harness. `cargo` suite untouched at **1484**
(`git diff dcbd6ea..HEAD -- crates proto` is empty). shellcheck is not
installed on this host (recorded, same as otp-12a/12b).

## Known gaps

- **Staging paths are not the plan's D6 shape.** D6 says stage skippy
  binaries to `$SKIPPY_BIN/bins/<sha>/`; the live rig has them flat at
  `/mnt/generic-pool/video/blit-bin/{blit,blit-daemon}` (the July
  REV4-era pair kept aside as `*.rev4-jul04`). The harness defaults
  follow D6 and the session passed the flat paths via
  `SKIPPY_BLIT`/`SKIPPY_DAEMON` overrides — recorded in the manifest,
  but the default path and the rig disagree.
- **`EXPECT_SHA` override in use.** HEAD at run time was `a2dea3f`
  (harness fixes) while the binaries embed `f35702a`; the two differ
  only by docs/scripts (`git diff f35702a..a2dea3f -- crates proto` is
  empty), so the session ran with `EXPECT_SHA=f35702a` and the manifest
  records the binary identity. The gate was satisfied by override, not
  by a rebuild.
- **The grpc cell is a single smoke** (`sw_grpc_large`), per D7 — one
  carrier data point, not a carrier matrix.
- **Windows daemon lifecycle**: launches must go through WMI
  `Win32_Process.Create`; Win32-OpenSSH reaps `Start-Process` children
  when the spawning ssh session closes (cost one debugging round —
  the daemon died silently and 9031 stealth-dropped, mimicking a
  firewall block).
- No `crates/` change ⇒ no Windows-parity run required for this slice.
