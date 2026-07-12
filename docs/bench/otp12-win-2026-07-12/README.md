# otp-12b — Mac↔Windows acceptance session: converge-up + initiator/verb invariance (2026-07-12)

**Status**: Recorded. **Scope**: the owner-designated closest-spec pair —
rig W carries the plan's cross-direction half AND the headline
initiator/verb-invariance criterion (`docs/plan/OTP12_ACCEPTANCE_RUN.md`
D2–D3; parent criteria 1–2 as annotated by D-2026-07-12-1). **This
README declares nothing** — pass/fail belongs to the owner at otp-13;
it records the computed rows.

**Harness**: `scripts/bench_otp12_win.sh` at run commit `e21cf84`
(design/harness codex rounds: 12 findings accepted at `d3eae58`; two
found-live fixes after first rig contact: the pwsh scope-qualified
`$rc:R` sentinel parse at `e21cf84`, and the CR-in-drain-outcome CSV
split at `856af64` — see Post-processing). RUNS=4, ABBA, pair-void rule;
**192 timed runs, zero voided pairs, zero drain anomalies**.

## Builds (matched pairs, sha-verified; 7 hashes in `staging-manifest.txt`)

- **old arm**: `0f922de` both ends — Mac client rebuilt clean in a
  detached worktree (pre-cutover clients embed no id:
  `OLD_CLIENT_PROVENANCE_BY_BUILD=1`, provenance = build procedure +
  manifest); Windows daemon = the aside-copied native detached-checkout
  build (embeds `+0f922de`, Select-String-verified).
- **new arm**: `e21cf84` both ends (Mac local build; Windows native
  build from a fresh bundle; `blit.exe` client likewise staged).
- Rig note: the box is `netwatch-01` at **10.1.10.177** (the recorded
  10.1.10.173 went stale — DHCP); Mac 10 GbE at 10.1.10.54.

## Post-processing (recorded, reproducible)

The session's `runs.csv` was CR-sanitized after the run (`tr -d '\r'`;
original committed as `runs-raw-crlf.csv`): pwsh emits CRLF and the
bare `\r` in the drain column split every row under python's
universal-newline csv reader, verdicting everything INCOMPLETE off 192
valid runs. `verdicts.csv`/`summary.csv` were recomputed with the
harness's own verdict pass over the sanitized rows; the harness now
strips CRs at source (`856af64`). No timing value was altered.

## Block 1 — converge-up (Mac-initiated, old vs new interleaved): 10/12 PASS

Combined outcomes (`verdicts.csv` carries per-reference rows):
PASS everywhere except —

| cell | new | old same-session | ratio | committed | ratio | outcome |
|------|----:|----:|----:|----:|----:|---------|
| push_tcp_small | 2080 | 1811 | **1.149** | 1868 | 1.113 | FAIL-BOTH (spreads 3.8/3.0% — real) |
| pull_tcp_mixed | 1138 | 867 | **1.313** | 1284 | 0.886 | FAIL-SAME-SESSION (spreads 5.2/6.7%) |

No pre-registered escalation trigger fires (no straddle with >25%
spread — these are tight-spread results); both stand recorded for the
otp-13 walk. Rig context: today's old arms run far FASTER than their
2026-07-10 committed medians (e.g. old pull_tcp_mixed 867 vs 1284, old
push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
so the committed bars are easy and the same-session bars are the
binding ones.

## Block 2 — initiator/verb invariance (new pair): 11/12 PASS

The owner's sentence, measured: per direction × fixture × carrier,
`max(mac_init, win_init)/min ≤ 1.10`. Eleven cells PASS at ratios
1.003–1.057. The exception:

- **wm_tcp_mixed FAIL at 1.237** (mac_init 1127 vs win_init 911, tight
  spreads 8.2/3.3%): Win→Mac mixed over the TCP data plane is ~25%
  slower when the MAC initiates (pull-verb, destination role) than when
  Windows initiates (push-verb, source role). Independently
  corroborated by block 1 (`pull_tcp_mixed` new 1138 vs old 867) and
  NOT present on grpc (wm_grpc_mixed 1.013) or other fixtures (large
  1.023, small 1.011) — the signature is specifically
  TCP-carrier × mixed workload × destination-initiator. A
  code-shaped finding for the otp-13 walk (and the exact class of
  defect this criterion exists to catch).

## Cross-direction (F4 + the D-2026-07-12-1 discriminator)

- **Win→Mac: all six cells PASS** — the unified path beats even the
  better committed old direction (ratios 0.71–0.99).
- **Mac→Win: all six cells FAIL** `min(old_push, old_pull) × 1.10` —
  and the gap rows attribute it: the same-session old direction gap
  (`old_push/old_pull`) vs the unified gap (`new_mw/new_wm`) is
  **unchanged on large (1.979 → 1.951 tcp; 1.956 → 1.945 grpc)** and
  **narrowed on mixed (1.946 → 1.408) and grpc_small (1.929 → 1.644)**
  — the residue is the Windows destination write path, present
  identically without blit's old choreography (D-2026-07-12-1: such
  cells count as satisfying criterion 2's cross-direction half). The
  one exception: **tcp_small's gap widened (1.332 → 1.527)** — the
  widening tracks the push_tcp_small code gap above, i.e. that cell's
  cross miss is NOT fully platform-attributable.

## Cross-block consistency note

`push_tcp_small` (block 1 new arm) measured 2080 while `mw_tcp_small`
mac_init (block 2, nominally the same work) measured 1922 — 8% apart in
one session. Block-2 arms use precreated destination containers (design
F5) where block 1 keeps the otp-2w shapes; the delta is recorded here
rather than adjudicated.

## Reproduction

```
export WIN_SSH=michael@netwatch-01 WIN_HOST=10.1.10.177
export MAC_HOST=<mac 10GbE ip>  OLD_CLIENT_PROVENANCE_BY_BUILD=1
RUNS=4 ./scripts/bench_otp12_win.sh
PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
```

Staging per the harness header (aside-copy the old exes BEFORE moving
the checkout; bundle + native build; sha-named bins; the daemons launch
from `bins\active\` under the one `blit-otp12-daemon` firewall rule).
