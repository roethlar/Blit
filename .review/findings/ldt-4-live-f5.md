# ldt-4-live-f5 — follow the verified current Windows endpoint identity

**Severity**: MEDIUM — the registered run cannot pass Windows preflight or
produce evidence while the harness dials a DHCP address the endpoint no longer
owns.
**Status**: Fixed, mutation-proved, full-gate green, tactically reviewed clean, and additively staged; live retry advanced to separate hostname finding f6.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `322a1611230e78c2268d91c45e6bd1e7ed24f953`

## Evidence

After q's registered `en8` / `10.1.10.54` link returned at MTU 9000 and active
10Gbase-T, exact staged harness `d53b5fd` failed closed on its first Windows
path preflight: pinned `10.1.10.177:22` timed out. The failure preceded harness
identity/artifact checks, evidence reservation, endpoint session creation,
fixture staging, daemon swap, or transfer.

q reached the router normally but had incomplete ARP for `.177`; nagatha also
could not reach it. Current DNS resolves `netwatch-01` to `10.1.10.173`, and
nagatha's existing strict host-key entry authenticated `.173` as
`NETWATCH-01`. Its ED25519/RSA/ECDSA keys exactly match q's trusted `.177`
entries, proving DHCP address drift rather than host substitution. The owner
directed the harness to adapt to verified reality and proceed.

## Predicted observable failure

Every launch stops at `assert_windows_registered_paths preflight` because SSH
to stale `.177` times out. No evidence namespace or live arm can exist.

## What

Move the registered Windows SSH/data-plane/analyzer identity together from
`.177` to `.173`, and pin that exact shared identity in offline tests.

## Approach

- Set both `WIN_SSH` and `WIN_IP` to verified-current `10.1.10.173` so SSH,
  route/ARP/MSS gates, daemon readiness, and transfer arguments cannot split.
- Set the analyzer's Windows identity to the same address.
- Make the Bash 3.2 self-test require `WIN_SSH == michael@$WIN_IP` and exact
  `.173`; make the analyzer tests independently require exact `.173`.
- Install only the already trusted matching host key under `.173` on q after
  review. Never disable strict host-key checking.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — current endpoint pins and shared-identity guard.
- `scripts/ldt4_rigw_analyze.py` — current evidence identity.
- `scripts/ldt4_rigw_analyze_test.py` — exact analyzer identity guard.

## Guard proof

- Exact `.173` candidate: Bash syntax passes; the 96-arm no-SSH self-test
  passes; all 76 analyzer tests pass.
- Reverting only the two production identities to `.177` makes the harness
  self-test fail with `Windows endpoint identity selftest failed` and the
  analyzer test fail on `.177 != .173`.
- Restoring `.173` returns both suites green.
- `cargo fmt --all -- --check`, strict workspace clippy, and the full workspace
  suite pass on the stable candidate. The first full-suite attempt correctly
  hit same-build refusal because docs changed concurrently between dirty binary
  builds; the exact failed test passed after edits stopped, and a targeted
  Cargo artifact clean plus untouched full rebuild passed without a code change.

## Coder dispute

None.

## Known gaps

Exact `322a161` is additively staged and q strictly authenticates `.173` with
the already trusted matching keyset. Its live retry cleared Windows SSH and all
three fixture manifests, then exposed separate stable-q-identity finding
`ldt-4-live-f6` before any arm or daemon preparation. A completed live run
remains.

## Reviewer comments

Grok 4.5/high reviewed exact range `31c12c9..322a161`, audited every SSH,
data-plane, topology, and analyzer identity use, ran the complete offline
suites, independently rejected old/split/wrong identities, and returned
`clean` with no findings. This is tactical advisory review, not formal
acceptance. Record: `.review/results/ldt-4-live-f5-r1.grok-verdict.md`.
