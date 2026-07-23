# Mac-to-Mac Thunderbolt RAM-path profile — 2026-07-23

## Verdict

The exact 0.1.1 candidate's large-file byte mover is already close to the
direct link ceiling. It moved 8,589,934,592 bytes from first data-plane record
write to data-plane completion in 1.931515 seconds: 35.578 Gb/s, or 93.9% of
the previously certified same-direction 37.9 Gb/s TCP ceiling.

External client time was 2.38 seconds, or 28.874 Gb/s. The wire's ideal time
for 8 GiB is 1.813179 seconds, so the total end-to-end excess was 0.566821
seconds. Only 0.118336 seconds belonged to the measured payload interval;
0.448485 seconds, 79.1% of the excess, was outside it.

The next target is therefore not large-file buffering or stream count. The
phase observer starts only after control-channel connection, Transfer RPC
opening, and the HELLO/OPEN/ACCEPT establishment exchange. It also ends before
final CLI rendering and process exit. Existing evidence cannot divide the
0.448-second outside interval among process startup, tonic connection/RPC
setup, session establishment, and post-summary teardown/output. Instrumenting
those boundaries is the smallest useful next step; changing transfer policy
from this result would be guesswork.

## Exact run

Session: `tbramprofile-20260723T005535Z-d1f1152d`.

- SOURCE: Q, Mac mini `Mac16,10`, 16 GiB, macOS 26.5.2 (`25F84`),
  `172.31.254.2/30`.
- DESTINATION: Nagatha, MacBook Pro `Mac16,5`, 48 GiB, same macOS build,
  `172.31.254.1/30`.
- Both routes resolved to `bridge0`, MTU 1500; source-bound pings passed and
  both Macs reported the direct peer at 40 Gb/s.
- Exact candidate: `d1f1152dd16b8c2bf8409cb5637135e3f89068c0`.
- Candidate `blit` SHA-256:
  `dc3cd55ad10903ef695db904f97ea3f6c0c7e6a300e4a163b95e766bced3cca0`.
- Candidate `blit-daemon` SHA-256:
  `652a8e641d1211ab9d4a254f56b6f2d9db0626c71a39d8861fee522ebbc74018`.
- Candidate archive SHA-256:
  `d1d7d9e547f703a7b5216cb3227baaf6b2bea848a85599312439cdccff19b726`.

The source was eight APFS clones of Q's retained 1 GiB seed. Every source file
was 1,073,741,824 bytes and matched SHA-256
`cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
Hashing all eight names warmed their shared physical extent. Q's filesystem
reported exactly the same free-block count before and after clone creation, so
no payload extents were allocated.

Nagatha received into a fresh 12 GiB APFS RAM disk. The client reported eight
files, exactly 8,589,934,592 bytes, `tcp_fallback: false`, and a complete JSON
success result. All eight RAM-destination files had the exact expected size and
SHA-256 before detachment.

## Phase attribution

Both endpoints emitted schema-1 session-phase records for session ID
`2d835d5ce55bdd57`.

| Source event | Elapsed from traced SOURCE origin |
|---|---:|
| Four epoch-0 sockets dialed and attached | 2.374 ms |
| Manifest complete sent | 3.969 ms |
| Need complete received | 4.555 ms |
| First payload queued | 4.684 ms |
| Membership sealed at 4 streams | 4.915 ms |
| First data-plane record write begins | 4.935 ms |
| Data plane complete | 1,936.450 ms |
| Summary received | 1,938.573 ms |

Membership sealed before the first 500 ms tuner tick, so the trace contains no
`dial_sample` and no resize. This is the expected consequence of all eight
payloads fitting the bounded pipeline: the tuner lifetime follows payload
admission, not later socket drain. It is not the cause of this run's material
gap, because the unchanged four-stream, 16 MiB-buffer data path still reached
93.9% of the wire ceiling.

The four destination receive tasks stopped at 1,882.691, 1,891.861,
1,935.368, and 1,938.827 ms. The 56.135 ms first-to-last spread is 2.9% of the
payload interval and explains part of the remaining 6.1% data-path gap, but one
sample cannot distinguish scheduler noise from a systematic tail imbalance.

## CPU and memory

The client used 0.31 user and 2.71 system CPU seconds over 2.38 seconds,
averaging 1.27 cores. Its peak memory footprint was 32.85 MB (maximum resident
set 41.12 MB). This confirms that Q's 16 GiB RAM is not a capacity limit.

The daemon accumulated 0.66 user and 3.85 system CPU seconds over its complete
161.62-second launch-to-deliberate-stop lifetime. It was idle outside the
1.939-second session, so 4.51 CPU seconds is a conservative upper bound on
transfer work: at most 2.33 average cores during the payload interval. Its peak
memory footprint was 19.66 MB (maximum resident set 27.92 MB). Neither endpoint
approached CPU-core or memory-capacity exhaustion, although kernel/system work
dominates both process accounts.

## Evidence handling and limits

- This is the one authorized profile, not an A/B matrix. There was no iperf,
  rsync, reverse, retry, or repeat arm.
- A pre-run snapshot caught ordinary Nagatha desktop activity, including a
  short `osanalyticshelper` spike and Spotlight/WindowServer work. An unrelated
  one-core Python test was allowed to finish before the run. Nothing was killed
  or disabled. The 2.38-second result nevertheless agrees with the prior
  2.40-second profile-free arm, and the in-session byte interval is far enough
  above the external result to support the fixed-overhead attribution.
- The orchestration shell used zsh's read-only `status` variable immediately
  after the SSH command. That bookkeeping line failed, so the wrapper did not
  append the direct SSH exit code or post-run interface counters. It did not
  affect the already-completed transfer. The client emitted its terminal
  success JSON and resource record; the matching daemon session emitted
  `summary_sent`; the client process exited; and eight independently hashed
  destination files prove complete semantic success. The evidence is retained,
  not discarded or rerun because an outer recorder failed after the result.
- The phase trace deliberately does not cover the pre-establishment and
  post-summary interval. No narrower claim about the 0.448 seconds is possible
  without adding those observation boundaries.

## Write safety and cleanup

No benchmark payload hit either SSD. Q's clone creation changed free blocks by
zero. Its only material generated SSD file was the exact 11,038,720-byte
allocated client binary; captured logs and repo evidence are about 40 KiB,
well below the plan's 32 MB ceiling.

After integrity validation, the candidate daemon was interrupted, reaped, and
its complete process accounting retained. The exact Q session root containing
the eight clone names and staged client was removed after path and inventory
checks. The RAM disk was detached and port 19041 was confirmed listener-free.
Q's retained seed, both static Thunderbolt addresses, the candidate artifact
in Trash, and every prior evidence directory remain.

Raw client, daemon, preflight, integrity, setup, and calculation records are in
`raw/`.
