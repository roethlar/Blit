# otp-12c — Mac↔Windows re-run at the cutover sha (2026-07-13)

**Status**: Recorded. **Scope**: the rig-W matrix re-executed with the
new arm at `f35702a` — the sha the shipped cutover binaries embed (12b
ran its new arm at `e21cf84`). Old arm unchanged (`0f922de`). Run as
the direct-path baseline ahead of the 12c delegated session
(netwatch-01↔skippy). **This README declares nothing** — pass/fail
belongs to the owner at otp-13; it records the computed rows.

**Harness**: `scripts/bench_otp12_win.sh` at run commit `f35702a`
(clean detached worktree `blit_v2_f35702a`; includes the 12b fixes
`856af64`/`49dee5c`/`b0a7bd9` — CR-strip at source held: 0 CR bytes in
`runs.csv`, no post-processing this time). Invocation:
`MAC_HOST=10.1.10.91 OLD_CLIENT_PROVENANCE_BY_BUILD=1
bash scripts/bench_otp12_win.sh`. RUNS=4, ABBA, pair-void rule;
**198 timed runs, 24/24 cells complete**; 3 pairs voided on
DRAIN-TIMEOUT and re-run to completion (`push_grpc_mixed` slot 3,
`mw_tcp_large` slot 2, `wm_tcp_small` slot 1 — see
`drain-outcomes.txt`).
Session `005904.49434`, 00:59–01:51 local. Endpoints: Mac
`10.1.10.91:9031`, netwatch-01 `10.1.10.177:9031`.

## Builds (sha-verified; 7 hashes in `staging-manifest.txt`)

- **old arm**: `0f922de` both ends. Mac client is pre-cutover and
  embeds no id — provenance = clean-worktree build + staging manifest
  (`OLD_CLIENT_PROVENANCE_BY_BUILD=1` acknowledged, logged in
  `bench.log` preflight lines).
- **new arm**: `f35702a` both ends (embed-verified at preflight).

## Verdicts — 120 rows: 93 PASS / 12 FAIL / 3 FAIL-SAME-SESSION / 12 RECORDED

FAIL rows verbatim from `verdicts.csv`:

```
pull_tcp_mixed,converge,new,old_session,1192,956,1.247,1.10,FAIL
pull_tcp_mixed,converge,new,combined,1192,,,1.10,FAIL-SAME-SESSION
push_tcp_mixed,converge,new,old_session,1703,1491,1.142,1.10,FAIL
push_tcp_mixed,converge,new,combined,1703,,,1.10,FAIL-SAME-SESSION
push_tcp_small,converge,new,old_session,1975,1644,1.201,1.10,FAIL
push_tcp_small,converge,new,combined,1975,,,1.10,FAIL-SAME-SESSION
mw_grpc_large,cross,worst_arm,min_old_committed,1750,1289,1.358,1.10,FAIL
mw_grpc_mixed,cross,worst_arm,min_old_committed,1837,1408,1.305,1.10,FAIL
mw_grpc_small,cross,worst_arm,min_old_committed,1981,1462,1.355,1.10,FAIL
mw_tcp_large,cross,worst_arm,min_old_committed,1707,1294,1.319,1.10,FAIL
mw_tcp_mixed,cross,worst_arm,min_old_committed,1477,1284,1.150,1.10,FAIL
mw_tcp_small,converge,win_init,old_session,1814,1644,1.103,1.10,FAIL
mw_tcp_small,cross,worst_arm,min_old_committed,1814,1280,1.417,1.10,FAIL
wm_tcp_mixed,invariance,mac_init,win_init,1221,939,1.300,1.10,FAIL
wm_tcp_mixed,converge,mac_init,old_session,1221,956,1.277,1.10,FAIL
```

Reading notes (numbers only, no adjudication):

- The new arm also **beats** old-in-session on the small-pull side:
  `pull_tcp_small` 1301 vs 1480, `pull_grpc_small` 1479 vs 1663,
  `push_grpc_small` 2264 vs 2656 (medians, ms). The losses concentrate
  in TCP×{small,mixed}×push plus `pull_tcp_mixed`.
- `wm_tcp_mixed` invariance ratio **1.300** (12b recorded 1.237) —
  the same TCP×mixed×dest-initiator cell 12b flagged as code-shaped.
  It did not wash out at the cutover sha.
- The `mw_*` cross rows compare this session's worst new arm against
  the best old **committed** direction (the D3 gap bar); the same
  push>pull directional gap is recorded in-session by the `gap_*`
  rows (1.111–1.771, all RECORDED).
- Outlier on record: `pull_grpc_small/old` avg 4783ms vs median 1663
  (spread 799.9%) — a single slow run, visible in `runs.csv`;
  verdicts are median-based.

## Files

`runs.csv`, `summary.csv`, `verdicts.csv`, `meta.csv`,
`staging-manifest.txt`, `drain-outcomes.txt` (the harness emits it as
`drain.log`; committed under 12b's artifact name because the repo
`.gitignore` excludes `*.log` — which is also why 12b named it so).
Full per-run daemon/client logs stayed with the session at
`blit_v2_f35702a/logs/otp12_win_20260713T005904/blit-logs/` (152
files, not committed).
