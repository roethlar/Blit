# Mac-to-Mac Thunderbolt ceiling probe — 2026-07-22

## Verdict

The direct Thunderbolt path is healthy and nearly saturates its 40 Gb/s link:
`iperf3` sustained 37.7–37.9 Gb/s in both directions with zero TCP
retransmissions. Exact 0.1.1 candidate `d1f1152d` moved an 8 GiB fixture at
28.6 Gb/s in 2.40 seconds. Apple openrsync moved the same fixture over its
unencrypted daemon protocol at 3.62 Gb/s in 18.99 seconds. Blit was 7.9x
faster and reached 75.7% of the measured TCP ceiling.

This is one explicitly approved, conservative ceiling probe, not a formal
acceptance matrix. It establishes a real product headroom gap between Blit and
the wire; it does not identify the cause of that remaining gap.

## Owner scope and write safety

The owner moved this probe ahead of publication, approved conservative SSD
writes, connected the direct cable, and approved the isolated Thunderbolt
network configuration. D-2026-07-22-4 records that narrow exception to the
post-release ordering in D-2026-07-22-1.

No benchmark payload was written to either SSD:

- Q's existing 1 GiB benchmark file was APFS-cloned eight times. The 8 GiB
  logical fixture consumed 12 KiB of new filesystem allocation.
- All destination payloads landed on a 12 GiB APFS RAM disk on Nagatha.
- The source clones and temporary binaries were removed from Q after hash
  verification. The RAM disk was detached. Local downloaded/build scratch was
  moved to Trash rather than permanently deleted.

## Endpoints and route certification

| Role | Host | Hardware | Memory | Thunderbolt address |
|---|---|---|---:|---|
| receiver | `nagatha.local` | MacBook Pro `Mac16,5` | 48 GiB | `172.31.254.1/30` |
| source | `Q.local` | Mac mini `Mac16,10` | 16 GiB | `172.31.254.2/30` |

Both hosts ran macOS 26.5.2 (`25F84`), arm64. System Information reported the
direct peer connected at 40 Gb/s. The automatic `169.254/16` addresses
overlapped other active interfaces, so the Thunderbolt Bridge services were
assigned the isolated addresses above, with no router or DNS. Both routes
resolved to `bridge0`, MTU 1500. Three source-bound pings in each direction had
zero loss and 0.472/0.484 ms average RTT.

## TCP ceiling

`iperf` 3.21 was built static without OpenSSL from the ESnet source archive
whose SHA-256 was
`656e4405ebd620121de7ceca3eaf43a88f79ea1b857d041a6a0b1314801acdd8`.
The resulting arm64 binary had SHA-256
`d29522d12b81211f02bf0643306b6d48cf0a33a2e52a6df84827358a97cd9fd8`
on both hosts. Each arm ran for 15 measured seconds after a two-second omit:

| Sender | Streams | Throughput | Retransmissions |
|---|---:|---:|---:|
| Nagatha -> Q | 1 | 37.8 Gb/s | 0 |
| Q -> Nagatha | 1 | 37.9 Gb/s | 0 |
| Nagatha -> Q | 4 | 37.7 Gb/s | 0 |
| Q -> Nagatha | 4 | 37.8 Gb/s | 0 |

The single stream already reached the link's practical TCP ceiling; more
streams did not improve it.

## Artifact and fixture identity

The ARM macOS archive came from successful CI run `29953569652` at exact
candidate `d1f1152dd16b8c2bf8409cb5637135e3f89068c0`:

| Artifact | SHA-256 |
|---|---|
| `blit-aarch64-apple-darwin.tar.gz` | `d1d7d9e547f703a7b5216cb3227baaf6b2bea848a85599312439cdccff19b726` |
| `blit` | `dc3cd55ad10903ef695db904f97ea3f6c0c7e6a300e4a163b95e766bced3cca0` |
| `blit-daemon` | `652a8e641d1211ab9d4a254f56b6f2d9db0626c71a39d8861fee522ebbc74018` |

The fixture contained eight 1 GiB files. Every source and destination file
hashed to
`cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
Hashing all source files before both timed arms both proved clone identity and
warmed the shared source extents.

## Transfer comparison

Nagatha received into separate empty directories on the same APFS RAM disk.
Q was the source for both tools, so physical byte direction and source cache
state were fixed.

Blit used the candidate daemon bound only to `172.31.254.1:19031` and its
default root export. The timed command was equivalent to:

```sh
/usr/bin/time -p blit copy \
  /Users/michael/blit-bench-work/thunderbolt_probe_20260722/ \
  172.31.254.1:19031:/default/ --yes --json
```

It reported 8,589,934,592 bytes and eight files transferred, no fallback, and
completed in 2.40 seconds: 3.58 GB/s or 28.6 Gb/s.

The rsync comparison used the Apple-provided openrsync implementation,
protocol 29 / rsync 2.6.9 compatible. It ran as an unauthenticated,
write-enabled daemon bound only to the isolated Thunderbolt address, with the
client using archive mode, `--whole-file`, and `--inplace`. Compression was
off by default. The timed command was equivalent to:

```sh
/usr/bin/time -p /usr/bin/rsync -a --whole-file --inplace --stats \
  --port=18730 \
  /Users/michael/blit-bench-work/thunderbolt_probe_20260722/ \
  rsync://172.31.254.1:18730/tb/
```

It reported 8,589,934,592 bytes of unmatched data and completed in 18.99
seconds: 0.453 GB/s or 3.62 Gb/s. No SSH encryption was in the path, so this
does not grade rsync on an unrelated encryption bottleneck.

## Interpretation and limits

- Blit is 7.9x faster than this macOS-native rsync baseline for the exact
  large-file fixture and route.
- Blit has about 24% headroom remaining below the measured TCP ceiling. The
  elapsed-time floor at 37.8 Gb/s is about 1.82 seconds; Blit took 2.40.
- The probe is one direction, one warm large-file shape, and one sample per
  tool. It isolates the engine and wire from destination storage, but does not
  represent cold-cache, real-disk, small-file, or mixed-tree behavior.
- The repeated APFS clones share physical source extents. That deliberately
  minimizes wear and source-cache size; it is not a test of sustained source
  SSD reads.
- No repeat is justified by these results: the gap to rsync is decisive, and
  any investigation of the remaining wire gap should start from profiling or
  existing counters, then use RAM-only confirmation under a separately
  approved tuning plan.
