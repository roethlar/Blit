# otp12-pf1-rigw-harness round 2 — GPT verdict

- Reviewer: `gpt-5.6-sol` (`xhigh`) via `codex-cli 0.144.4`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..8fbd4866cbf83ab6af4d8a0467dbb9680172d3b0`
- Review timestamp: `2026-07-15T09:39:24Z`
- Raw review: `.review/results/otp12-pf1-rigw-harness-r2.codex.md`
- Review verdict: `NEEDS FIXES`

## Adjudication

### F4 — Windows-client completion has an uncharged pre-anchor interval

**Verdict: Accepted (High).** `win_client_run` stops its Windows stopwatch and
emits the result inside PowerShell, but `run_arm` captures the q monotonic
settle anchor only after the complete SSH command exits. Time spent delivering
the sentinel and tearing down the remote command is therefore after the client
but before both `transfer_ms` and the settle interval. Destination writeback
can progress during that role-correlated hole without being measured or
charged.

Fix: `6ba5408`. The Windows result is stamped on q as its flushed result line
arrives, that q monotonic timestamp is carried out of the wrapper and used as
the absolute settle anchor, and the q-local wrapper returns its immediate q
completion timestamp. Delayed-producer, post-return-anchor, and cross-process
clock mutations each turn the self-test red.

### F5 — initiator role changes the physical destination path

**Verdict: Accepted (High).** `rid` contains `source_init` or
`destination_init`, and that value selects both the precreated local
destination and the remote module path. Adjacent paired arms therefore use
different parent paths even though the registered comparison says the only
varied property is the initiator. Pathname and parent-directory placement can
become a role-correlated cost.

Fix: `1231e42`, landed first. `rid` remains unique for logs and evidence, while
every arm on a destination endpoint uses one session-scoped role-independent
canonical path that is reset before and removed after each arm. Remote-URI and
local-selector mutations each turn the self-test red.

### G1 — required live launcher smoke has no executable mode

**Verdict: Accepted (pre-run execution gap, found during coder runbook
audit).** The finding requires a live Windows launcher smoke before data, but
the committed entry point exposes only offline `SELFTEST`, daemon-free
`PREFLIGHT_ONLY`, and the full registered run. `win_daemon_start` is otherwise
first reached inside block 1, after the harness has declared the registered run
started. The required CIM quoting proof therefore cannot be run as a separate
gate.

Fix: `18d3cde`. `LAUNCHER_SMOKE=1` is mutually exclusive, runs full provenance
and preflight, starts only the exact Windows launcher/daemon, proves q
reachability, identity-stops it, retrieves daemon stderr, proves closed ports,
and completes strict cleanup without registering or timing a transfer,
invoking the analyzer, or creating `SESSION-COMPLETE`.

### G2 — partial CIM startup could outrun its PID journal

**Verdict: Accepted (pre-run safety gap, found during follow-up coder audit).**
`Invoke-CimMethod` could create the launcher before either remote PID file was
written. A later startup error could therefore reach failure cleanup with no
owned PID; if the launcher spawned and then exited, parent-only discovery was
also insufficient to stop the orphan deterministically.

Fix: `454ebce`. The generated launcher waits on a bounded block-local gate and
cannot execute the daemon until its launcher PID is atomically placed and read
back. Failure teardown also recovers the unique exact launcher command, its
exact parented child, and a child that races the first query. Gate-order and
identity mutations each turn the self-test red.

## Confirmed closures

Round 2 independently confirmed the original F1 fail-closed destination reset,
F2 excess-settle accounting, and F3 complete endpoint-local resize DAG plus
Rust emitter ordering. It also confirmed one `Transfer` RPC, SOURCE-send /
DESTINATION-receive semantics under either initiator, role-independent stream
targeting, and no legacy push/pull path.

No rig run is authorized until the complete fixed range receives a fresh
mandatory Codex pass and the standalone live launcher smoke plus endpoint
preflight are green.
