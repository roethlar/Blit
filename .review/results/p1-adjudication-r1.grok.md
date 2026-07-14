I'll adjudicate P1 from the recorded bench data and harnesses directly—flush vs transfer times, the grpc control, and Linux results—not from the prior reviews' conclusions.Next I'll pull the baseline CSVs and decompose total vs flush on the P1 cell and controls.Flush looks symmetric on the P1 cell already — I'll quantify transfer-only time and run the same checks on grpc and Linux.Checking earlier P1 sessions for the same flush/transfer split, and reading the prior-review settle argument carefully.Computing per-session effect sizes to pin the numbers:## Verdict

**P1 is a real transfer-time effect.** The free-writeback / settle-gap story does **not** explain the recorded Mac↔Windows numbers, and it does **not** invalidate them. Confidence: **high** (the CSV already contains the diagnostic the artifact hypothesis needs, and every prediction fails).

The prior reviews that treated the old harness as “suspect until re-measured with settle” were **wrong about these data**. An equal settle is reasonable hygiene for a new instrument; it is **not** required to decide this dispute from what was already recorded.

---

## Reasoning (from the numbers)

### 1. Flush is symmetric on the P1 cell; the effect is almost entirely transfer

`ms = transfer_window + flush_ms`. If the gap after client exit let one arm do free dirty writeback, that arm should show a **smaller** `flush_ms`. On `wm_tcp_mixed` it does not.

| session | mac_init total / flush / xfer | win_init total / flush / xfer | Δ total | Δ flush | Δ xfer | % of Δ in xfer |
|---|---:|---:|---:|---:|---:|---:|
| otp12-win (1.237) | 1127 / **84.0** / 1042 | 912 / **82.5** / 829 | 216 ms | **+1.5 ms** | 212 ms | **98.6%** |
| otp12c-win (1.300) | 1222 / **80.5** / 1140 | 939 / **82.0** / 857 | 282 ms | **−1.5 ms** | 284 ms | **~100%** |
| q-baseline (1.385) | 1093 / **72** / 1021 | 790 / **73** / 720 | 304 ms | **−1 ms** | 300 ms | **99.0%** |

On the session that hit **1.385**, transfer-only medians are **1021 vs 720** (ratio **1.417**)—**larger** than the total ratio. Flush median delta is **1 ms** against a **~300 ms** effect. That is the opposite of “P1 lives in flush / free writeback.”

Absolute flush size also matters: ~**70–85 ms** for 5001 files on the Mac destination is fsync-walk scale (~14–17 µs/file), not media writeback of 547 MB. Same-session Mac→Win mixed pays ~**650–660 ms** flush on Windows for the same fixture—so when durability work is still pending, the harness **does** record hundreds of milliseconds. On the P1 cell it records ~70 ms on **both** arms.

### 2. Gap direction predicts the wrong flush pattern

On `wm_*`, dest is Mac. `mac_init` runs the client **locally** then fsyncs immediately; `win_init` runs the client under **ssh on Windows**, then the Mac fsync starts after that return path. The free-writeback hypothesis therefore predicts **more** free writeback (and a **smaller** flush) on **win_init**. Observed flush: win_init is equal or slightly **higher** (q: 73 vs 72; 12c: 82 vs 80.5). No hundreds-of-ms flush deficit on the fast arm.

If free writeback had “stolen” ~300 ms of durability from only one arm, the other arm’s flush would sit ~300 ms higher. Both sit on the same ~70 ms floor.

### 3. Same fixture, gRPC control: artifact should fire, it doesn’t

Writeback pending is a property of the **fixture + destination OS**, not the carrier. `wm_grpc_mixed` on q uses the **same** mixed tree, same Mac destination, same immediate-fsync harness, same initiator swap:

- totals: **1254 vs 1230** → ratio **1.020** (PASS)
- flush: **69 vs 71** ms (again flat)
- xfer: **1184 vs 1160** (noise)

A gap/writeback manufacturing path that depends on who typed the command would hit TCP and gRPC alike. The recorded signature is **TCP-only**, fixture-mixed, Win→Mac—matching a real path effect, not an accounting gap.

Same session, opposite direction `mw_tcp_mixed`: total ratio **1.043**, flush **658 vs 656** (flat), small xfer delta. Controls in-session sit at ~2–4%; P1 is **38%**.

### 4. Linux↔Linux uses the same immediate-flush style and shows no P1

`docs/bench/otp12-perf-2026-07-13/`: `sm_tcp_mixed` **1.091** PASS with flush **780 vs 780** (identical medians). Immediate flush **can** measure initiator symmetry when the transfer is symmetric. That weakens “immediate flush invents one-way asymmetry,” rather than supporting it.

(Linux can show noisy flush elsewhere, e.g. `ms_grpc_mixed`; that is a different cell and does not rescue the Mac↔Windows artifact claim for `wm_tcp_mixed`.)

### 5. What the prior review got right vs wrong

- **Right as abstract caution:** client-exit → fsync is not one continuous interval; a long, direction-asymmetric gap *could* bias totals if residual dirty writeback were large and unequal.
- **Wrong about these recordings:** the harness already splits `flush_ms`. For every Mac↔Windows P1 cell on record, that column is **symmetric and tiny relative to the effect**, and the entire failure lives in **transfer wall time**. The settle concern does **not** invalidate the measured P1 ratios.

A historically real durability-accounting bug (sync keyed by initiator) **was** caught because it polluted **gRPC too**. This free-writeback claim fails that same control.

---

## Bottom line

The data refute the artifact hypothesis as an explanation of P1 (wholly or in large part). P1 is real in the transfer window on Mac↔Windows TCP mixed.

**P1 REAL** — Treat the recorded Mac↔Windows invariance failure as genuine and investigate the transfer path (not a settle re-measure of the old sessions).
