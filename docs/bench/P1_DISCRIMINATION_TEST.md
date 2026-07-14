# P1 discrimination test — locate the ~300 ms, decisively

**Status**: Draft test design, 2026-07-15. **⚠ RE-SCOPED after the owner pointed at existing
data — most of the "hypotheses" below were already answered and this doc was reinventing them.**
Read the box first; the matrix that follows is retained only for the ONE question the existing
data does not close.

## ⛔ STOP — what is ALREADY MEASURED (do not re-derive it)

**blit's small-file throughput does not scale with concurrency. This is measured, documented,
and robocopy-tested.** Sources, all already in the repo:

- **`docs/bench/win-local-ab-2026-07-13/`** (4-arm interleaved, local D:→E: on netwatch-01, no
  network): `mixed` blit 1-worker 930 ms → `--workers 8` **784 ms (1.19×)**; robocopy `/MT:1`
  1052 → `/MT:8` **487 ms (2.16×)**. `small`: blit 1402 → 1336 (**1.05×**); robocopy 1540 → 697
  (**2.2×**). **At EQUAL concurrency blit WINS** (mixed 0.884, small 0.911); it loses only
  because it does not turn 8 workers into throughput. This IS "streams 8 against robocopy".
- **`docs/plan/LOCAL_SMALL_FILE_PATH.md`** — the mechanism, code-read: small files (<1 MiB) go
  through a per-file, ~5-syscall, single-worker apply (`local.rs:602` `sink_workers=1`; serial
  reads with no read/write overlap, `pipeline.rs:196-206`). **L3 (single worker / no scaling) is
  the PRIME suspect**; hypothesis L1 (tar framing) was DEMOTED because blit wins at 1 thread.
- **`docs/bench/10gbe-2026-07-05/DIAGNOSIS.md`** — over the wire, **small and mixed push already
  ran on effectively ONE stream** ("one connection carried all 10,000 files"; "Mixed 512 MiB+5k
  push: single stream"), bottlenecked at ~215–235 µs/file on the daemon. So multi-stream RAMP
  speed was never the small-file lever — the apply does not parallelize regardless of streams.
- **`docs/plan/MULTISTREAM_PULL.md`** — pull was historically **hardcoded 1 stream**; unification
  gave it multi-stream, and **pull-vs-push parity was an explicit open acceptance question from
  the start** ("pull throughput at parity with push … or the gap explained", line 62). **P1 IS
  that gap.** It is not a surprise; it is the unanswered acceptance question.

**Consequence for this test.** The ramp-speed hypothesis (H-ramp) and the fixture-shape ablations
below (`uniform`, `bulk8`, `small50k`, `midmix`) largely re-ask a settled question: blite's
small-file apply is single-worker and does not scale. They are NOT the test to run. The **one
thing the existing data does not close** is the specific link `OTP12_PERF_FINDINGS.md` Decision-1
names: **does the unified REMOTE receive/apply give the destination FEWER effective apply
workers in pull than in push** (client-as-Initiator vs daemon-as-Responder), so the same
non-scaling apply bottleneck bites harder on the pull arm? That — a code read of the receive
worker count + whether the sink apply serializes, pull vs push, confirmed by the phase
instrument's per-stream steady-state numbers — is the real P1 question. **Everything below is
kept only as the confirmation harness for THAT, not as a fresh hunt.**

---

*Original framing (retained for the confirmation matrix only). Pairs with the phase
instrumentation (`docs/bench/P1_INSTRUMENTATION.md`).*

## The question

P1: on a macOS↔Windows pair, over TCP, the **mixed** fixture transfers ~25–38 % slower when the
**destination** initiates (pull) than when the **source** does (push). ~300 ms. TCP-only,
mixed-only, pull-only, absent Linux↔Linux. Code reading found **one** push/pull asymmetry (the
pull destination acquires each epoch-N resize socket by a blocking inline dial,
`transfer_session/mod.rs:3125`, where push uses a non-blocking arm, `mod.rs:3122`) and ruled out
Nagle, socket buffers, and resize-count. It could **not** decide whether that asymmetry costs the
300 ms. This test decides it.

## What reading already established (so the test targets the right thing)

- All three shipped fixtures propose **8 streams** (`dial.rs:474`), so resize *count* is not the
  discriminator.
- **A single file rides ONE stream** (per-file serialization, `mod.rs:1466`). Therefore the
  shipped **large** fixture (1 GB, 1 file) uses **one** stream regardless of the 8-stream proposal
  — it *cannot* exercise multi-stream ramp, which is exactly why large is insensitive and passes.
  **The shipped fixtures do not cleanly separate "bulk" from "multi-stream".** This test adds
  fixtures that do.

## Hypotheses under test

| id | hypothesis | where the 300 ms would be |
|----|------------|---------------------------|
| **H-ramp** | pull reaches full stream parallelism slower because each epoch-N socket is a serialized synchronous dial; while ramping, files queue | the STREAM-RAMP phase; pull's stream-count-vs-time curve lags push's |
| **H-steady** | at full stream count, pull's steady-state throughput is lower (e.g. the pull-side resize sockets get kernel-default buffers, `socket.rs:74-80`, vs push's ramped buffers) | the STEADY-STATE phase; equal ramp, lower plateau throughput |
| **H-setup** | the pull connection/epoch-0 establishment is slower (control-connection direction, first-byte latency) | the SETUP phase, before payloads |
| **H-persmall** | a per-small-file cost that only the pull topology pays | scales with small-file COUNT, flat in bytes |
| **H-platform-dial** | the per-dial handshake itself is tens of ms on macOS↔Windows specifically (delayed-ACK / connection-setup pathology), invisible on a LAN-fast path | per-epoch dial duration in the RAMP phase, large only on the macOS↔Windows path |
| *(unlikely)* **H-mixture** | it is specifically the *coexistence* of one bulk file and many small files (scheduling interaction), not the byte/count profile | mixed fails but a same-bytes/same-count uniform fixture passes |
| *(unlikely)* **H-manifest** | need/manifest-direction cost (refuted by reading — needs finish before resizes, `mod.rs:1356/2856`; kept as a null check) | the MANIFEST/NEED phase |
| *(unlikely)* **H-bigstream** | the single monopolized bulk stream interacts with the small-file streams | fails only when a bulk file AND small files share the session |

## The fixtures (generation only — NO blit code change)

Fixed total where noted so byte-magnitude is held constant against the shipped mixed (547 MB):

| name | contents | streams used | isolates |
|------|----------|--------------|----------|
| `big1` | 1 × 512 MB | 1 | bulk, single-stream (≈ shipped large) — control, expect PASS |
| `small5k` | 5000 × 2 KB (10 MB) | 8 | many small, trivial bytes — control, expect PASS |
| `mixed` | 1 × 512 MB + 5000 × 2 KB | 8 | **the shipped P1 fixture** — expect FAIL |
| `uniform` | 5001 × ~109 KB (547 MB) | 8 | **same bytes & count as mixed, no bulk file** → H-mixture vs byte/count profile |
| `bulk8` | 8 × 64 MB (512 MB) | 8 | multi-stream **bulk**, few files → H-ramp/H-steady without small files |
| `small50k` | 50000 × 2 KB (100 MB) | 8 | 10× the small-file count → H-persmall (cost ∝ count?) |
| `midmix` | 1 × 512 MB + 5000 × 20 KB (612 MB) | 8 | small-file *size* ×10, count fixed → separates count from per-file bytes |

## The matrix

For every fixture: **push vs pull**, **TCP**, fixed data direction, **N = 8 ABBA pairs**, cold
caches both ends, destination drained, the **phase instrumentation ON** (records SETUP /
STREAM-RAMP curve / STEADY-STATE throughput / MANIFEST-NEED times / per-epoch dial durations).
Plus two controls on `mixed`: **gRPC** (carrier control — expect PASS, TCP-specificity) and, if a
diagnostic stream-cap knob is added (see below), **pull with streams pinned to 1** (no resize at
all — the cleanest H-ramp kill/confirm).

Run it on the reviewed macOS↔Windows rig harness (`scripts/bench_otp12_win.sh` methodology:
fixtures, cold cache, ABBA, pair-void, the MSS/topology discipline). A **cheap first cut** runs
the same matrix on a fast controllable pair (nagatha↔q) purely to read the **ramp curve** — if
pull's ramp does not lag push's even there, H-ramp is weak and H-platform-dial or H-steady leads.

## Decision rule (what each outcome PROVES)

Read the phase breakdown first; the fixtures confirm.

1. **Phase breakdown on `mixed` pull vs push** localizes the 300 ms to SETUP, RAMP, or STEADY.
   That alone selects H-setup / H-ramp / H-steady.
2. **`bulk8` fails, `small5k` passes** → the effect needs multi-stream **bulk**, not small files →
   H-ramp or H-steady (confirm by which phase). **`bulk8` passes, `mixed` fails** → the small
   files are necessary → H-persmall or H-mixture.
3. **`uniform` fails** → it is the **byte/count profile** (what triggers the 8-stream ramp under
   sustained transfer), NOT the bulk+small mixture → H-ramp/H-steady, and H-mixture is refuted.
   **`uniform` passes, `mixed` fails** → H-mixture (the coexistence matters).
4. **`small50k` ratio ≫ `small5k` ratio** (cost grows with small-file count) → H-persmall.
   **`small50k` ≈ `small5k` ≈ pass** → per-small-file cost refuted.
5. **per-epoch dial duration is tens of ms on the rig but sub-ms on nagatha↔q** → H-platform-dial
   (a macOS↔Windows connection-setup pathology); the fix is then about the dial itself, and it
   would show up in the RAMP phase.
6. **streams-pinned-to-1 pull ≈ push (P1 gone)** → resize/ramp is the cause, decisively.
   **streams-pinned-to-1 still shows P1** → resize is exonerated; the cost is in the single-stream
   steady-state path or setup.
7. **gRPC mixed passes (known) while TCP mixed fails** → confirms it is in the TCP data-plane
   path, not the planner/need logic (which is carrier-independent).

The combination is exhaustive: it assigns the 300 ms to a phase (1), to bulk-vs-small (2,3), to
count (4), to the platform dial (5), to resize specifically (6), and to the TCP path (7). No
result leaves P1 unlocated.

## The one knob this needs (diagnostic only, gated)

Conditions 6 needs a way to pin the negotiated stream count (e.g. `local_receiver_capacity()`
`max_streams` overridable by a `BLIT_MAX_STREAMS` env var, diagnostic-only, default unchanged).
That is a small, behavior-affecting change and therefore goes through the review loop; it does
NOT change the wire format (it only lowers a negotiated ceiling, which the protocol already
permits). If it is judged out of scope, conditions 1–5 and 7 still locate P1 without it.

## Non-goals / honesty

- This does not need the Mac↔Mac acceptance rig; it is a diagnostic, not an acceptance run.
- A null on the cheap nagatha↔q cut proves nothing about the rig (LAN erases the RTT-sensitive
  phases); only the macOS↔Windows wall-time is decisive. The cheap cut is for the ramp CURVE
  SHAPE, not the magnitude.
- Every fixture add is generation-only; the only code change is the optional stream-cap knob.
