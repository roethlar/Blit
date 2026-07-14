# P1 phase-timing instrumentation

**Status:** Diagnostic benchmark instrument; off by default.

## Enable it

Set `BLIT_PHASE_TRACE` in every process whose endpoint summary is wanted. The
variable is presence-gated: leave it unset for normal transfers. It is
independent of `--trace-data-plane`; the existing flag still prints immediate
connect messages, while this instrument buffers timestamps and prints only
after the transfer summary has completed.

For a two-ended capture, start the daemon with the variable and redirect its
stderr, then run exactly one client transfer with the variable set. For
example, on macOS/Linux:

```bash
BLIT_PHASE_TRACE=1 ./target/release/blit-daemon \
  --config /path/to/bench-config.toml 2>daemon-phase.log
```

On Windows PowerShell:

```powershell
$env:BLIT_PHASE_TRACE = '1'
.\blit-daemon.exe --config D:\blit-test\bench-config.toml 2> daemon-phase.log
```

Example source-initiated push and destination-initiated pull of the same mixed
fixture, using fresh destination directories:

```bash
# Push: SOURCE is the client/initiator.
BLIT_PHASE_TRACE=1 ./target/release/blit copy \
  /bench/mixed/ win-host:9031:/bench/p1-push/ --yes 2>push-client.log

# Pull: DESTINATION is the client/initiator.
BLIT_PHASE_TRACE=1 ./target/release/blit copy \
  win-host:9031:/bench/mixed/ /bench/p1-pull/ --yes 2>pull-client.log
```

The daemon must also inherit `BLIT_PHASE_TRACE` to produce the peer-side lines.
Do not run concurrent transfers into the same log: every line is structured,
but deliberately carries no allocated transfer identifier in the hot path.

## Output

Every timestamp is milliseconds from that endpoint's own session entry, before
the hello/open handshake. Clocks on different hosts are not synchronized; do
not subtract a client timestamp from a daemon timestamp. Compare the same
`role=` across the push and pull runs.

All output is on stderr and starts with `[phase-trace]`:

- `summary`: `total_ms` ends after the source receives, or the destination
  sends, `TransferSummary`. `setup_to_first_payload_ms` is session start to the
  first TCP stream carrying a non-END record. `payload_ms` and `payload_mib_s`
  cover first payload through local data-plane drain. `summary_bytes` is the
  destination-attested total; `completed_payload_bytes` is the trace wrapper's
  sanity count and should match it for a completed TCP transfer.
- `manifest`: local send/receive times for `ManifestComplete` and
  `NeedComplete`. A source normally has the `*_sent` manifest field and the
  `*_received` need field; a destination has the converse. On the destination,
  `need_complete_sent_ms` must precede every epoch-1-or-later arm/dial start.
  That is the runtime check that need generation finished before resize socket
  acquisition began.
- `steady`: `full_streams` is the highest socket count that actually became
  ready. `start_ms` is that final stream's ready time and `end_ms` is the local
  data-plane drain. `completion_bytes` counts payloads that completed after
  `start_ms`; `mib_s` divides that count by the interval. Accounting at payload
  completion avoids a clock read on every record or byte.
- `stream`: one line per stream. Epoch 0 is the initial connection.
  `proposed_ms` appears on the source; `armed_ms` appears on the push
  destination. `action=dial|accept`, `action_start_ms`, and `ready_ms` bracket
  socket acquisition/authentication. `first_payload_ms` is the first actual
  payload use (an END-only stream stays `na`). `done_ms` is completion of that
  stream's sink/END work.
- `head_idle_ms` is time after full parallelism was available but before that
  stream first received work. `tail_idle_ms` is time after that stream finished
  while another stream kept the data plane alive. These expose start/tail work
  starvation without a timestamp in the per-record scheduler.

`na` means the event is not owned by that endpoint, did not occur, or the TCP
data plane was not used. A P1 capture must say `carrier=tcp`.

## Localize the approximately 300 ms

Use one clean push and one clean pull of the identical 512 MB + 5000 x 2 KB
fixture, same build and endpoints, with the destination freshly reset each
time. Keep the four endpoint groups: push source/destination and pull
source/destination. Compare in this order:

1. **Setup:** compare `setup_to_first_payload_ms`, then epoch-0
   `action_start_ms -> ready_ms`, the manifest fields, and epoch-0
   `first_payload_ms`. If the pull is already about 300 ms behind before its
   first payload, P1 is in connection/session/manifest setup, not steady TCP
   transfer.
2. **Ramp:** compare the source `proposed_ms` sequence and, especially, the
   destination stream lines. Push should show `armed_ms -> ready_ms` with
   `action=accept`; pull should show `action=dial` from
   `add_dialed_stream`. Compare each acquisition duration and the `steady
   start_ms` at the final stream. If first-payload setup is similar but pull
   reaches the same `full_streams` about 300 ms later, or the sum of its resize
   dial spans explains the gap, P1 is stream ramp.
3. **Steady state:** if both runs reach the same `full_streams` at comparable
   times, compare destination `steady duration_ms`, `mib_s`, and per-stream
   head/tail idle. Roughly equal setup/ramp but a lower pull steady-state rate,
   a longer drain, or about 300 ms more boundary idle places P1 after ramp.
4. **Reconcile:** the phase carrying P1 should explain the difference in
   same-role `total_ms`. If no reported interval does, the missing time is an
   interior scheduling/write gap and needs a second, separately reviewed
   instrument rather than inferring that the resize dial caused it.

The decisive P1 comparison is therefore:

```text
push destination responder: epoch-N armed -> accepted/ready -> first payload
pull destination initiator:  epoch-N dial start -> dial ready -> first payload
```

Align those with each run's source proposal times and final `steady start_ms`.
Only a measured pull-specific excess in those spans supports the resize-ramp
hypothesis.

## Deliberate limits

The instrument does not timestamp every payload, record, read, write, or byte;
doing so would perturb the benchmark it is meant to explain. Consequently,
`completion_bytes` assigns a payload to the interval in which it completes (a
large file can span the ramp boundary), and idle reporting covers only the
observable head and tail gaps. Interior idle gaps between two payloads on the
same stream are intentionally skipped. No wire field, connection, task,
ordering rule, or transfer decision changes when tracing is enabled.
