# otp-12 rig-W MTU experiment — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 2** (codex review of `35b9620`:
NOT READY, 4 BLOCKER + 3 HIGH — all accepted; adjudication in
`.review/results/pf-0-prereg.gpt-verdict.md`). **No data exists yet.** This
file is committed BEFORE the run so the decision rule cannot be authored
around the numbers. Results land beside it in `README.md` + CSVs.

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (**Active**, D-2026-07-13-1).
**This is an ENVIRONMENTAL experiment, not a code counterfactual** — it is
not one of H1–H7 and changes nothing in the tree. It runs first because it is
the cheapest experiment available and it can invalidate the premise of the
expensive ones. **A PASS licenses evidence for a plan amendment only**: it
cannot reshape pf-1, re-baseline, or close P1/P2 without a reviewed amendment
(codex F8).

## The question — restated, because the original premise was FALSE

`netwatch-01` ran at **MTU 1500 for every benchmark ever recorded**
(otp-2w, otp-12a/b/c). Jumbo has therefore **never once been exercised**.

**CORRECTION (codex F6).** The queued rationale — repeated in `docs/STATE.md`
— says P1's cell is "TCP × **mixed** — the most packet-heavy fixture we
test". **That is false.** Segment counts at MSS 1448:

| fixture | bytes | segments @1448 | segments @8948 |
|---|---|---|---|
| **large** | 1 073 741 824 | **~741 500** | ~120 000 |
| mixed | 547 110 912 | ~377 800 | ~61 100 |
| small | 40 960 000 | ~28 300 | ~4 600 |

**`large` is the packet-heaviest fixture, by ~2×.** `mixed` is P1's cell
because that is where the failure was *observed*, not because of packet
count; what distinguishes `mixed` is the **interleaving** of one bulk stream
with 5000 small files. The premise is therefore the weaker, honest one:

**Falsifiable premise**: reducing per-packet overhead (MTU 1500 → 9000)
removes some or all of P1's invariance gap.

## Instrument validation — what it proves, and what it does NOT

| check | method | result |
|---|---|---|
| Mac interface MTU | `ifconfig en9` | 9000 (`en9` = 10.1.10.54, Aquantia) |
| Windows interface MTU | `Get-NetIPInterface` | `NlMtu` 9000; NIC `Jumbo Packet = 9014 Bytes` |
| L2 path both ways | DF ping (Win→Mac 8972 B; Linux→both 8972 B; Mac→Win 8000 B) | **OK** — the macOS 8192 raw-socket cap forbids more from the Mac; it is a ping limit, not a network one |
| **negotiated MSS, both directions** | `getsockopt(TCP_MAXSEG)` (macOS) + `ss -ti` (Linux) | **8948** each way |

`8948 = 9000 − 40 − 12 (timestamps)`; at MTU 1500 it is **1448**. So the path
**permits** a 6.18× segment reduction.

**WHAT THIS DOES NOT ESTABLISH (codex F5).** 8948 is the **ceiling**, not the
**fill**. It does not prove blit's data plane actually emits full-size
segments during the timed transfers — application write boundaries, Nagle,
and record framing can all leave segments short of the MSS. **Segment fill is
unmeasured.** The wall-clock comparison between matched MTU conditions is
therefore the test; the MSS number only establishes that the opportunity
exists.

**A GLOBAL NULL IS A LEGITIMATE RESULT.** If nothing moves at 9000, the
correct reading is "per-packet cost is irrelevant to blit on this rig", NOT
"the instrument lied". (Revision 1 had this backwards and made a real
possible outcome unfalsifiable.)

**An instrument was tested and DISCARDED.** Windows
`Get-NetAdapterStatistics` reported **10 680 received bytes per "packet"**
during a 1 GiB transfer — larger than a 9014-byte frame — so that NIC
coalesces on receive despite reporting `RSC IPv4Enabled: False`. NIC packet
counters cannot discriminate 1500 from 9000 here and are not used.

## Design — BOTH MTU conditions are measured (codex F1)

Revision 1 ran jumbo alone and would have compared it to 12b/12c, which
differ in **NIC** and **sha**. No prior session is a valid control, and a
FAIL at jumbo would have proved only that jumbo is *insufficient* — never
that MTU contributes nothing. So MTU is measured as an actual variable:

| held constant | value |
|---|---|
| new-arm sha | `f35702a` (worktree `blit_v2_f35702a`, binaries verified to embed `+f35702a`) |
| old-arm sha | `0f922de` |
| Mac NIC / IP | Aquantia, 10.1.10.54 |
| harness, fixtures, RUNS, CELLS | identical across both conditions |

| varied | condition A | condition B |
|---|---|---|
| **Windows MTU** | **9000** | **1500** |

Two back-to-back sessions on the same quiet machine, **identical scope**,
**RUNS=8** (P1's bar is defined at RUNS=8 — `OTP12_PERF_FINDINGS.md:548`; and
4 samples cannot resolve this — codex F3).

`CELLS=wm_tcp_mixed,pull_tcp_mixed,mw_tcp_mixed,wm_grpc_mixed,wm_tcp_large,wm_tcp_small,push_tcp_small`

- `wm_tcp_mixed` — **P1's cell** (the measurand)
- `pull_tcp_mixed` — the other half of P1's bar
- `mw_tcp_mixed` — opposite-direction control (passes today: 1.044)
- `wm_grpc_mixed` — opposite-carrier control (passes today: 1.021)
- `wm_tcp_large` — **the bulk-packet positive control** (see below)
- `wm_tcp_small` — fixture control (passes today: 1.027)
- `push_tcp_small` — P2

**Per-session instrument check**: the MSS is re-measured with
`getsockopt(TCP_MAXSEG)` **after each MTU flip and before each session**, and
recorded. A session whose MSS is not the expected value (8948 / 1448) is
void. The condition is proven, never assumed.

MTU is flipped on Windows (the ssh session is elevated):
`Set-NetIPInterface -InterfaceAlias Ethernet -NlMtu <1500|9000>`, restored to
9000 afterwards.

## Decision rule — the parent's UNIFORM scale, on a Δ that is actually measured

For `wm_tcp_mixed` in each condition, the parent defines the gap as the **arm
difference** (`OTP12_PERF_FINDINGS.md:501`):

    Δ(mtu) = median(mac_init) − median(win_init)     [same session, new-vs-new]

**MTU's recovery** is then the share of the gap it removes:

    r = (Δ_1500 − Δ_9000) / Δ_1500

graded on the parent's **pre-registered** scale (`OTP12_PERF_FINDINGS.md:516`),
with no post-hoc bands:

- `r ≥ 50%` → **CONFIRMED DOMINANT** — MTU is the main cause of P1
- `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING** — real, but not the whole story
- `r < 20%` → **KILLED** as a material cause

Reported **separately** (they are different questions, and revision 1
conflated them):

- **Does P1 pass at jumbo?** `wm_tcp_mixed` invariance ≤ 1.10, computed with
  the harness's exact integer arithmetic `10·hi ≤ 11·lo`
  (`bench_otp12_win.sh:668`) — not the printed 3-decimal ratio.
- **Does MTU contribute?** the `r` above. A FAIL of the bar with `r ≥ 20%`
  means *MTU is a real contributing cause AND P1 still fails* — both true at
  once. Revision 1 could not express that.

### The masking guard (codex F4 — the old one was porous)

Revision 1's guards let a shared 1000 ms floor pass all three tests. With
both MTU conditions measured, masking is now **observed, not inferred**:

- **Fast-arm guard**: `win_init` at 9000 must not regress against `win_init`
  at 1500 by more than 5% (`100·win_9000 ≤ 105·win_1500`).
- **Degradation, not fix**: if BOTH arms are slower at 9000 while the ratio
  improves, the ratio improvement is **bottleneck compression**, and it is
  reported as a degradation — never as a P1 pass.

### The positive control (codex F6 — replaces "something must move")

**`wm_tcp_large` is the bulk-packet control**: ~741k segments at 1500,
falling to ~120k at 9000 — the largest per-packet saving available anywhere
in the matrix. Pre-registered:

- If blit benefits from jumbo **at all**, `wm_tcp_large` improves at 9000 vs
  1500 by **≥ 5%** on the median of each arm.
- If `wm_tcp_large` does **not** improve (< 5%) and no other TCP cell
  improves, then **blit does not benefit from jumbo on this rig at all** — and
  any movement in `wm_tcp_mixed`'s ratio is therefore **not** an MTU effect
  and must be read as drift, not as a fix. This is the falsifier that stops a
  lucky ratio from being sold as a result.

## Verdict rows that are VOID in the jumbo condition (codex F7)

The harness grades against the committed baseline
`otp2w-baseline-2026-07-10/summary.csv` (`bench_otp12_win.sh:105`), **which
was recorded at MTU 1500**. In the 9000 condition:

- **VOID**: every `converge … old_committed` row, every
  `cross … min_old_committed` row, **and every block-1 `combined` row** — the
  `combined` verdict is PASS only if the committed leg also passes
  (`bench_otp12_win.sh:697-702`), so it silently embeds the stale reference.
  (Codex's evidence: 12b's P2 reads `FAIL-BOTH` where 12c's reads
  `FAIL-SAME-SESSION` *solely* because the committed leg flipped.)
- **SOUND**: **invariance** rows (new-vs-new, one session — this is P1) and
  `converge … old_session` rows (both arms on the same network).

**None of this experiment's conclusions use the committed baseline.** The
measurand is the 9000-vs-1500 comparison between two matched sessions.

**Consequence for the plan**: P2's bar requires ≤1.10 against **BOTH**
references (`OTP12_PERF_FINDINGS.md:553`). At jumbo the committed reference is
stale, so **P2 cannot close at jumbo until the baseline is re-recorded there**.
That is a plan amendment; it goes through the loop and is not assumed here.

## Residual limitations (stated, not hidden)

- **Session ordering.** Condition A runs before condition B on the same
  machine; a thermal or cache-history bias would land on B. Each ratio is
  session-internal (both arms interleaved ABBA within a session), which is
  what makes the ratio robust to this. If the two conditions land within a
  few percent of each other, that is not resolvable by ordering alone and the
  escalation is a repeat of condition A after B.
- **Segment fill is unmeasured** (above). A null does not distinguish "blit
  leaves segments short" from "per-packet cost does not matter".
- **Rig hostname**: `netwatch-01` intermittently fails to resolve (DHCP/mDNS —
  `.agents/machines.md`). Both sessions pass `WIN_SSH=michael@10.1.10.177`
  explicitly so a resolution failure cannot silently retarget a run.

## The runs

```sh
cd /Users/michael/Dev/blit_v2_f35702a     # clean detached worktree @ f35702a
CELLS=wm_tcp_mixed,pull_tcp_mixed,mw_tcp_mixed,wm_grpc_mixed,wm_tcp_large,wm_tcp_small,push_tcp_small

# condition A — MTU 9000 (fleet is already here); verify MSS = 8948 first
WIN_SSH=michael@10.1.10.177 MAC_HOST=10.1.10.54 \
  OLD_CLIENT_PROVENANCE_BY_BUILD=1 RUNS=8 CELLS=$CELLS \
  bash scripts/bench_otp12_win.sh

# flip Windows to 1500, verify MSS = 1448, then condition B — identical scope
# Set-NetIPInterface -InterfaceAlias Ethernet -NlMtu 1500
WIN_SSH=michael@10.1.10.177 MAC_HOST=10.1.10.54 \
  OLD_CLIENT_PROVENANCE_BY_BUILD=1 RUNS=8 CELLS=$CELLS \
  bash scripts/bench_otp12_win.sh

# restore: Set-NetIPInterface -InterfaceAlias Ethernet -NlMtu 9000
```

Staging verified: worktree clean at `f35702a`; `target/release/{blit,blit-daemon}`
embed `+f35702a`; Windows has `D:\blit-test\bins\{0f922de,f35702a}`; no stale
daemon on either host; Mac old client at `~/blit-bench-work/bins/blit-0f922de`.

**Rig-state changes made during validation** (recorded, not hidden):
netwatch-01's `known_hosts` gained the Mac's key at 10.1.10.54, and an SMB
session to `\\10.1.10.54\blit-bench-work` was established by the owner (for a
separate robocopy baseline). No blit code, config, or fixture was touched.
