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

Fix: pending. Stamp the Windows result on q as its result line arrives, carry
that q monotonic timestamp out of the wrapper, and use it as the absolute
settle anchor. The q-local wrapper must likewise return its immediate q
completion timestamp. A delayed producer must prove that the Windows stamp is
captured before SSH/process teardown, and reverting to the post-return anchor
must make the guard fail.

### F5 — initiator role changes the physical destination path

**Verdict: Accepted (High).** `rid` contains `source_init` or
`destination_init`, and that value selects both the precreated local
destination and the remote module path. Adjacent paired arms therefore use
different parent paths even though the registered comparison says the only
varied property is the initiator. Pathname and parent-directory placement can
become a role-correlated cost.

Fix: pending and first in order. Keep `rid` for unique logs and evidence, but
route every arm on a given destination endpoint through one role-independent
canonical destination path, resetting that exact path before each arm. A
mutation-sensitive self-test must prove SOURCE- and DESTINATION-initiated arms
resolve to the same physical path and that `run_arm` cannot reintroduce `rid`
into either remote destination URI.

### G1 — required live launcher smoke has no executable mode

**Verdict: Accepted (pre-run execution gap, found during coder runbook
audit).** The finding requires a live Windows launcher smoke before data, but
the committed entry point exposes only offline `SELFTEST`, daemon-free
`PREFLIGHT_ONLY`, and the full registered run. `win_daemon_start` is otherwise
first reached inside block 1, after the harness has declared the registered run
started. The required CIM quoting proof therefore cannot be run as a separate
gate.

Fix: pending. Add a reviewed smoke-only mode that runs provenance and
preflight, starts and identity-checks the exact Windows launcher/daemon,
proves reachability, tears it down, and completes strict cleanup without
timing a transfer or creating `SESSION-COMPLETE`.

## Confirmed closures

Round 2 independently confirmed the original F1 fail-closed destination reset,
F2 excess-settle accounting, and F3 complete endpoint-local resize DAG plus
Rust emitter ordering. It also confirmed one `Transfer` RPC, SOURCE-send /
DESTINATION-receive semantics under either initiator, role-independent stream
targeting, and no legacy push/pull path.

No rig run is authorized at this verdict.
