# End-to-end transfer latency attribution — 2026-07-23

## Verdict

The historical 0.448-second gap outside the payload observer did not recur.
In the successful instrumented run, the existing first-socket-write-to-data-
plane-complete span was 3.587845 seconds and the complete client process
reported 3.60 seconds. At `/usr/bin/time`'s 0.01-second precision, only
7.155–17.155 ms lay outside that data span.

The new lifecycle trace directly observed 13.645 ms outside the data span:
8.623 ms from process trace origin through the first socket write and 5.022 ms
from data-plane completion through command terminal. Control connection, RPC
opening, and HELLO/OPEN/ACCEPT establishment together took 4.687 ms. Argument
parsing through route selection took less than 1 ms, and result rendering
through command terminal took 0.041 ms. The residual outside the trace is
bounded only by the external timer's rounding, at 0–3.449 ms.

Therefore none of the newly divided fixed product stages explains the prior
0.448 seconds. That historical gap was either cold work before async `main`,
external wrapper/environment state, or another non-reproducing condition. The
successful run was necessarily warm after resolving a macOS firewall prompt,
so this evidence cannot divide cold loader/runtime construction more narrowly.

The successful sample's data body moved 8,589,934,592 bytes at 19.153 Gb/s;
the complete client process delivered 19.089 Gb/s. The data body therefore
dominates this sample. That is substantially below the earlier 35.578 Gb/s
body result, but this plan authorized one observation rather than an A/B or
repeat, so the difference is not a regression verdict and earns no transfer-
policy change by itself.

## Exact successful run

Run ID: `etl5-20260723T054219Z-a3be4a64`.

- SOURCE: Q, `172.31.254.2/30`, Mac16,10 with 16 GiB.
- DESTINATION: Nagatha, `172.31.254.1/30`, Mac16,5 with 48 GiB.
- Both routes used `bridge0`, MTU 1500, and the direct peer reported 40 Gb/s.
- Candidate head: `a3be4a64fbfb7a7ff3e867d40a3b75ba582a1517`.
- Reviewed product head: `dd1ac0adf029b3f9c72f17acf13c6f423aac9264`;
  there is no product diff from that reviewed head to the candidate.
- Client SHA-256:
  `c3ebc375d69fdae84f40a72efbc6daac28b635f7e92af4e5648ee3480ac8181d`.
- Daemon SHA-256:
  `2b6273860274f8151e0574e1addbc84eacadf0e220bcf37fd41a8ac2c22a1da4`.
- Both binaries identify as `0.1.1+a3be4a64fbfb`.

The source was the retained 1 GiB seed exposed as eight APFS clones. Every
source and destination file was 1,073,741,824 bytes and matched SHA-256
`cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
The client reported eight files, exactly 8,589,934,592 bytes, and
`tcp_fallback: false`; it exited zero. Nagatha received into a fresh 12 GiB
APFS RAM disk.

Q allocated three 4 KiB filesystem blocks while creating the clones: 12,288
bytes of APFS metadata, not 8 GiB of copied extents. The staged client occupied
11,100,160 allocated bytes. Total Q SSD allocation was therefore 11,112,448
bytes, below the 32,000,000-byte budget. No benchmark payload was written to
either SSD.

## Lifecycle attribution

| Span | Time |
|---|---:|
| Trace origin → async-main entry | 0.072 ms |
| Async-main entry → argument parse complete | 0.822 ms |
| Context load | 0.057 ms |
| Dispatch entry → route selection begin | 0.001 ms |
| Route selection | 0.079 ms |
| Route complete → control connect begin | 0.084 ms |
| Control connection | 1.096 ms |
| Connection complete → Transfer RPC begin | 0.032 ms |
| Transfer RPC opening | 2.821 ms |
| RPC complete → session establishment begin | 0.022 ms |
| HELLO/OPEN/ACCEPT establishment | 0.769 ms |
| Established session → body return | 3,595.624 ms |
| Body return → render begin | 0.006 ms |
| Result rendering | 0.038 ms |
| Render complete → dispatch end | 0.001 ms |
| Dispatch end → command terminal | 0.001 ms |
| Trace origin → command terminal | 3,601.551 ms |

The correlated schema-1 session observer used session ID
`7ec836d983f776fb`. Four streams were live and peak, with receiver ceiling 32.
The first socket-write begin to source data-plane completion span was
3.587845 seconds. Source data-plane completion to summary receipt was
4.940 ms. Destination receive-task stops spread across 78.196 ms.

The client consumed 0.35 user + 3.02 system CPU seconds, averaging 0.936 CPU
cores over its reported wall time. Maximum RSS was 39,141,376 bytes and peak
memory footprint was 30,867,984 bytes. The daemon's complete 101.59-second
lifetime consumed 0.51 user + 3.10 system seconds; charging all of that CPU to
the transfer gives a conservative 1.003-core upper bound. Daemon maximum RSS
was 31,244,288 bytes and peak footprint was 24,347,056 bytes.

## Firewall-blocked setup attempt

The first timed invocation transferred zero bytes and is not treated as a
performance sample. It established TCP, then remained at
`transfer_rpc_open_begin` for 120.24 seconds with an empty destination. macOS
Application Firewall logs at the exact start timestamp show it queued a new
inbound `blit-daemon` flow and sent an unanswered filtering-decision prompt.
A later read-only gRPC probe was held behind the same pending response, while
a raw TCP probe across the same Thunderbolt addresses succeeded.

The exact RAM-disk daemon was then temporarily allowlisted. A read-only
`list-modules` call immediately succeeded, proving the control plane before
the successful transfer. The temporary firewall entry was removed during
cleanup. No clones were recreated and the blocked invocation wrote no payload.

## Observer limits

- Lifecycle timing begins inside async `main`; dynamic loading, Tokio runtime
  construction, and OS teardown remain outside it.
- `/usr/bin/time` prints wall time to 0.01 seconds, so its small residual is an
  interval rather than a falsely precise point estimate.
- The outer SSH wrapper lasted 3.827152 seconds. Aligning Q event wall stamps
  with Nagatha's wrapper clock assigns 175.573 ms before async-main and
  50.143 ms after command terminal, but that division depends on cross-host
  clock synchronization and is not product runtime.
- Clearing the firewall gate and running the read-only proof warmed the exact
  client before the successful sample. The historical cold-start gap therefore
  cannot be reproduced or attributed from this run.
- This is one sample with no comparison arm. Its lower data-plane rate cannot
  select a buffer, stream, filesystem, or carrier change.

## Cleanup

The exact Q clone directory and staged client are removed; the retained seed
still has its original size, allocation, and hash. The daemon is stopped, port
19041 has no listener, the temporary firewall entry is absent, and the 12 GiB
destination RAM disk is detached. The build RAM disk was detached after the
raw evidence was retained. Static Thunderbolt addresses and all earlier
evidence were left intact.

Machine-readable calculations and the complete client, daemon, preflight,
firewall, integrity, and cleanup records are under `raw/`.
