# SSD-backed direct-Thunderbolt probe — 2026-07-22

## Verdict

Exact 0.1.1 candidate `d1f1152d` moved 12 GiB from Q's internal APFS SSD to
Nagatha's internal APFS SSD in 7.73 seconds: 1.667 GB/s or 13.335 Gb/s. Apple
openrsync moved the same physical source fixture in the same direction to a
separate directory on the same destination SSD in 33.81 seconds: 0.381 GB/s
or 3.049 Gb/s. Blit was 4.37x faster by external wall time.

Every one of the twelve source files, twelve Blit destination files, and
twelve rsync destination files matched SHA-256
`cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
Both tools reported the exact 12,884,901,888-byte payload and exited zero.

The SSD-backed Blit arm reached 46.6% of the RAM-backed arm's 28.6 Gb/s, a
53.4% throughput reduction, and 35.2% of the same-direction 37.9 Gb/s TCP
ceiling. The combined source-read/destination-write storage path, not the
Thunderbolt network, limited this run. This one direction cannot assign the
storage limit between Q's source reads and Nagatha's destination writes.

## Comparison

| Arm | Payload | Wall time | GB/s | Gb/s |
|---|---:|---:|---:|---:|
| iperf, Q -> Nagatha | memory | 15.00 s | 4.74 | 37.9 |
| Blit, RAM destination | 8 GiB | 2.40 s | 3.58 | 28.6 |
| Blit, SSD source + destination | 12 GiB | 7.73 s | 1.667 | 13.335 |
| openrsync, SSD source + destination | 12 GiB | 33.81 s | 0.381 | 3.049 |

The RAM rows are context from
`docs/bench/thunderbolt-macmac-2026-07-22/README.md`; they were not rerun.

## Scope and identities

The active plan was `docs/plan/THUNDERBOLT_SSD_PROBE.md`, approved under
D-2026-07-22-5 with a 40 decimal GB ceiling and no retries or repeats. Session
tag: `tbssd-20260723T001314Z-d1f1152d` (UTC; 2026-07-22 local).

- SOURCE: Q, Mac mini `Mac16,10`, 16 GiB, macOS 26.5.2 (`25F84`), internal
  APFS solid-state volume, `172.31.254.2/30`.
- DESTINATION: Nagatha, MacBook Pro `Mac16,5`, 48 GiB, same macOS build,
  internal APFS solid-state volume, `172.31.254.1/30`.
- Both routes resolved to `bridge0`, MTU 1500. Peer ARP records were on
  `bridge0`; source-bound pings passed; both Macs reported the direct peer at
  40 Gb/s immediately before fixture creation.
- Q had 32.9 GiB free before setup; Nagatha had 124 GiB free. Both exceeded
  the plan's fail-closed free-space gates.
- Candidate archive SHA-256:
  `d1d7d9e547f703a7b5216cb3227baaf6b2bea848a85599312439cdccff19b726`.
- Candidate `blit` SHA-256:
  `dc3cd55ad10903ef695db904f97ea3f6c0c7e6a300e4a163b95e766bced3cca0`.
- Candidate `blit-daemon` SHA-256:
  `652a8e641d1211ab9d4a254f56b6f2d9db0626c71a39d8861fee522ebbc74018`.

## Fixture and cache control

Q's retained 1 GiB seed already had the expected hash above. The run created
twelve ordinary `dd` copies, not APFS clones. Each file was exactly
1,073,741,824 bytes. File allocation totaled 12,926,844,928 bytes, proving the
source was physically backed rather than twelve references to one extent.

The fixture was synchronized and every source file was hashed. Q's authorized
`/usr/sbin/purge` ran immediately before each timed arm. Thus both tools read
the physical 12 GiB source after cache eviction. The destination directories
used `.noindex` suffixes to avoid deliberate Spotlight indexing of the test
payload.

## Blit arm

The exact daemon bound only to `172.31.254.1:19031`, disabled mDNS, and
exported the empty Blit SSD destination as its default root. Q proved the
export before timing. The client reported:

- twelve files and 12,884,901,888 bytes transferred;
- `tcp_fallback: false`;
- no deletes or resumes; and
- `real 7.73`, `user 0.29`, `sys 2.88` seconds.

The daemon was stopped before all twelve destination files were sized and
hashed. File allocation totaled 12,963,823,616 bytes.

## rsync arm

Both Macs used the Apple-provided openrsync, protocol 29 / rsync 2.6.9
compatible. The daemon bound only to `172.31.254.1:18730` and exposed one
write-enabled module restricted to `172.31.254.2`. The client used archive
mode, `--whole-file`, and `--inplace`; compression was off and SSH was not in
the path. It reported:

- twelve transferred files and 12,884,901,888 bytes of unmatched data;
- 12,886,475,885 bytes sent and 310 bytes received;
- no matched data or partial failure; and
- `real 33.81`, `user 10.53`, `sys 4.41` seconds.

The daemon was stopped before all twelve destination files were sized and
hashed. File allocation totaled 12,982,026,240 bytes.

## Write accounting

Filesystem allocation for all plan-created material was:

| Material | Allocated bytes |
|---|---:|
| Q physical source fixture | 12,926,844,928 |
| Nagatha Blit destination | 12,963,823,616 |
| Nagatha rsync destination | 12,982,026,240 |
| Q exact candidate client | 11,038,720 |
| rsync config and captured logs | 16,384 |
| **Total** | **38,883,749,888** |

That is 38.884 decimal GB, 1.116 GB below the owner's 40 GB ceiling. Whole-
volume free-space deltas were not used for accounting because APFS and
unrelated system activity make them noisy; the table sums each generated
file's allocated 512-byte blocks.

## Limits

- This is one sample per tool, in one physical direction, with fixed Blit-then-
  rsync order. The owner explicitly prohibited automatic repeats.
- Q's cache was purged before both arms, but ordinary macOS background work was
  not disabled. Nagatha showed modest Terminal/WindowServer/Spotlight activity;
  no build, test, review, or other deliberate load ran during either arm.
- The repeated file content is incompressible for practical purposes and
  neither tool compressed it, but all twelve files contain the same bytes.
- The result isolates a combined SSD path. It does not identify which SSD or
  filesystem stage owns the 53.4% reduction from the RAM-backed Blit result.
- Timing alone does not authorize a product change or another transfer. Any
  attribution or tuning starts with read-only code/counter analysis.

Raw client and daemon output is in `raw/`. Cleanup is recorded after this
evidence commit, as required by the plan.
