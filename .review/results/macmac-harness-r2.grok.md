I'll review the harness, verdict logic, tests, preregistration, and the two existing working harnesses independently — no other reviews first.Comparing timing and durability in the existing harnesses, and verifying whether macOS `time.monotonic()` is process-relative.Claim (a) is empirically real on this machine. Digging further into timing, the Linux harness comparison, and claim (b).Noting the zoey harness already rejects cross-process monotonic. Re-checking the timer site and a few more failure modes.Verifying the preflight fail-open claims and other structural issues.# Third-party instrument review — mac↔mac OTP-12 harness

Independent review of the code as it stands. Empirical checks on this host’s system Python (`3.9.6`, CLT). Adversarial.

---

## PART 1 — Independent findings

| # | Location | Sev | Finding |
|---|----------|-----|---------|
| 1 | `bench_otp12pf_mac.sh:397–400`, `412` | **BLOCKER** | **Transfer timer is not a wall clock.** `t0`/`t1` are separate `python3 -c` processes using `time.monotonic()`. On this macOS 3.9, monotonic is process-start-relative (~0 at import). Measured: 500 ms real sleep → **3 ms** reported. `RUN_MS ≈ RUN_FLUSH` (fsync only). Invariance is graded on fsync noise, not transfer cost. **Masks** any real srcinit/destinit transfer gap; **can manufacture** a one-directional “effect” if fsync cost differs by arm/path. Linux harness brackets with `/proc/uptime` (system-wide); win/zoey use `time.time()` or one Stopwatch; zoey even documents why cross-process monotonic is wrong. |
| 2 | `otp12pf_mac_verdict.py:115–140`; prereg §POWER GATE | **BLOCKER** | **VANISHES equivalence margin is `0.10 × srcinit_median`, not the reference effect.** `DELTA_REF_MS=230` is written to CSV only (line 35, 155) and **never used** in the decision. A true, tight effect of 230 ms on a 2500 ms arm (ratio 1.092, bar PASS, CI=[230,230] ⊂ ±250) yields **VANISHES** — i.e. “P1 absent / Windows may be required” while a rig-W-sized absolute Δ is present. Same class: all `d_i=190`, `src=2000` → VANISHES (breach=200). |
| 3 | `otp12pf_mac_verdict_test.py:57–67` | **HIGH** | Guard test **does not cover** finding 2. It only blocks the old *range* bug (spread case → PARTIAL). Constant 190 ms / 230 ms effects that still VANISHES are untested. Passing the suite does not mean the decision rule is safe. |
| 4 | `otp12pf_mac_verdict.py:175–178` | **HIGH** | **RIG-VOID fails open.** Prereg: any control **bar FAIL** voids the rig. Code requires `bar==FAIL` *and* outcome ∉ `{VANISHES,INCONCLUSIVE,UNDERPOWERED}`. Control with bar FAIL, CI crossing 0 → `INCONCLUSIVE` → **not** RIG-VOID → session can still declare **VANISHES**. Reproduced: grpc controls ratio=1.200 bar=FAIL, session VERDICT=VANISHES. |
| 5 | `bench_otp12pf_mac.sh:194–197` | **HIGH** | **`embeds_clean` rejects clean binaries.** `grep -c` with 0 matches prints `0` and exits 1; `|| echo X` then appends `X` → dirty becomes `0\nX`, which fails `^[0-9]+$`. Preflight dies with a false “not CLEAN” (wrong diagnosis). Fail-closed for running, but the gate is broken. |
| 6 | `bench_otp12pf_mac.sh:207` | **HIGH** | **`norm_mac` uses gawk `strtonum`**, absent on stock macOS `awk` → ARP normalization errors → `link_gate` cannot pass. Preflight blocked (fail-closed), gate implementation wrong. |
| 7 | `bench_otp12pf_mac.sh:225–230` (spotlight) | **MED** | **Spotlight gate can fail open:** if `mds_stores` is missing from `top`’s sample, awk yields 0 → treated as quiet. Same class as “cannot measure ⇒ 0%”. |
| 8 | `bench_otp12pf_mac.sh:344` | **MED** | Drain hardcodes **`iostat … disk0`**. If the bench volume is not that device’s stats, drain certifies the wrong disk (void miss or false quiet). |
| 9 | `bench_otp12pf_mac.sh:398` | **LOW** | Timed-run client stderr is `/tmp/mm-client.err` (overwrite, not under `OUT_DIR`). Voids without durable client diagnostics — ops risk, not a direct arm bias. |
| 10 | `otp12pf_mac_verdict.py:111` | **LOW** | Integer bar: exact 1.10 ratio is **PASS** (`10*hi <= 11*lo`). A precise 10% effect never REPRODUCES. |
| 11 | fsync path vs transfer timer | **(consequence of #1)** | In-process fsync timing (`:375–386`) is fine; durability count/byte void is fine; equal `SETTLE_MS` is fine **for relative fsync**. None of that saves the measurand if transfer ms is garbage. |

**Not found as arm-biased (given current design):** destination-keyed fsync with count/byte match; equal settle; purge-then-drain order; ABBA pairing; pair-void on exit/cold/drain; registered cell allowlist. Those are real improvements over past harnesses — and they do not make the instrument safe while #1–#2 stand.

---

## PART 2 — Adjudication of the two claims

### (a) Transfer timer broken (two-process `monotonic`) — **CONFIRMED**

Code:

```397:400:scripts/bench_otp12pf_mac.sh
  out="$(hrun "$ih" "t0=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))')
echo \"R:\$((t1-t0)),\${rc}:R\"" ...
```

Evidence on this Mac, system Python 3.9.6:

| Pattern | 500 ms sleep result |
|---------|---------------------|
| Two `python3 -c` + `time.monotonic()` (harness) | **~3 ms** |
| Two `python3 -c` + `time.time()` (win/zoey) | **~522 ms** |
| One process, monotonic around `sleep` | **~510 ms** |

`time.monotonic()` values sit near **0.0x s** per process; a second process does not share the first’s origin. So `t1−t0` ≈ spawn jitter, not transfer duration. Then `RUN_MS = (≈0) + RUN_FLUSH` → rows are essentially fsync time. Claim (a) is correct.

### (b) VANISHES with real rig-W-sized effect — **CONFIRMED**

Engine:

```115:140:scripts/otp12pf_mac_verdict.py
        breach = 0.10 * s_med
        ...
        elif bar == "PASS" and ci_lo > -breach and ci_hi < breach:
            out = "VANISHES"
```

Injected: `srcinit=2500`, all eight `d_i=230` (both TCP×mixed cells; clean controls).

| Quantity | Value |
|----------|-------|
| ratio | 1.092 → bar **PASS** (`10×2730=27300 ≤ 11×2500=27500`) |
| `BAR_BREACH` | `0.10×2500 = **250** ms` |
| D, CI | **230**, **[230, 230]** |
| 230 ∈ (−250, 250)? | **yes** (strict) |
| **Session verdict** | **`VANISHES`** |

Reference Δ (230 ms) is **inside** the equivalence band built from the **bar** (250 ms), so the engine reports “effect excluded” while a rig-W-sized absolute effect is present on every pair. Claim (b) is correct. The power gate as implemented does **not** require excluding `DELTA_REF_MS`.

---

## VERDICT

**NOT SAFE TO RUN on the nagatha↔q rig.**

Even if preflight were forced past #5/#6, finding **#1** makes every row’s transfer component meaningless and can manufacture or mask the one-directional signature under test; finding **#2** can still mint a confident **VANISHES** in the presence of a rig-W-sized absolute effect. The DO NOT RUN banner already on the file is commentary only — it does not `exit`. Do not collect or bank data from this instrument until the timer is system-wide or single-process (as the other harnesses already do), the equivalence/power rule is fixed against the reference effect (or an explicit absolute/relative policy that cannot hide Δ_ref), RIG-VOID matches the prereg, and the guard tests cover the claim-(b) case.
