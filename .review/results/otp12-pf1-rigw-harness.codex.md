- `scripts/bench_otp12pf_rigw.sh:1431,1527` — **High** — A destination-removal failure can still produce a valid, artificially fast arm. Because `prepare_destination` is invoked on the left of `||`, Bash disables `set -e` inside it; failed q removal is masked by successful `mkdir`, while Windows removal explicitly uses `SilentlyContinue`. Stale canonical files may then be skipped yet pass landed-manifest verification.

- `scripts/bench_otp12pf_rigw.sh:1571, scripts/otp12pf_rigw_analyze.py:441` — **High** — Role-specific clock-probe delay can provide up to roughly 750 ms of uncharged background writeback, reducing `flush_ms` and biasing durable totals. The analyzer accepts `settled_ms=999` while excluding all settling from the measurand; only the first 250 ms is actually balanced.

- `scripts/otp12pf_rigw_analyze.py:1198,1224` — **High** — Causally impossible destination traces receive `ANALYSIS-PASS`: the validator does not require `resize_received → socket_dial_begin` for destination initiators or `resize_arm_ready → socket_accept_begin` for source initiators. Both mutations pass, allowing misleading phase attribution and eventual completion marking.

VERDICT: NEEDS FIXES