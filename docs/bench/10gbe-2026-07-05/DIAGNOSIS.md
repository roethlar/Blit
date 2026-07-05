# 10 GbE session — small-file limiter diagnosis (2026-07-04/05)

Durable extract of the measurement basis behind
`docs/plan/SMALL_FILE_CEILING.md` (the raw run dirs live under
gitignored `logs/`; every number the plan relies on is reproduced
here). Rig: skippy (TrueNAS SCALE, 32 cores, ZFS `generic-pool`,
enp66s0f1 @ MTU 9000) ↔ netwatch-01 (Arch, 32 cores, enp6s0 10 GbE @
MTU 9000, tmpfs local ends). Methodology: engine-vs-wire isolation —
async ZFS writes, ARC-warm re-reads, no sync between runs.

## Wire ceiling (iperf3 3.21)

- forward 9.88 Gbit/s single stream / 9.91 ×4 streams; reverse 9.91.
- Reference throughput ceiling ≈ 1.24 GB/s.

## Stream-count evidence (blitd stderr, small vs large push)

Large 1 GiB push — the dial negotiated 8 data-plane connections;
work-stealing gave the single file to one stream:

    stream complete: files=1, bytes=1073741824 (9.69 Gbps)
    (+7× "stream complete: files=0, bytes=0" clean idle teardowns)

Small 10k×4 KiB push — **one** connection accepted, one stream
carried all 10,000 files:

    stream complete: files=10000, bytes=40960000 (0.15 Gbps)
    aggregate 0.15 Gbps (40960000 bytes in 2.14s)   [runs: 2.14/2.26/2.36s]

Per-file arithmetic: 2.14–2.36 s ÷ 10,000 ≈ **215–235 µs/file
sequential** on the daemon (ZFS create+write+set-times per file).
Wire time for the same payload: 40 MiB @ 9.9 Gbit/s ≈ **34 ms** —
the wall is ~65× the wire cost.

Mixed 512 MiB+5k push: single stream, 2.6–3.0 Gbit/s data plane.

## Tripwire + fs-floor evidence (tool_comparison.csv, best of 2)

- rsyncd (native protocol, same ZFS target): 10k push **1.49 s** →
  proves a ≤ ~150 µs/file single-pipe receive floor exists on this
  filesystem; 10k pull 367 ms.
- blit 10k push 2.37 s (variance to 3.31 s), pull 446 ms.
- blit wins all large/mixed-pull/local cells (see CSV); losing cells
  are exactly the per-file-bound ones.
- rclone fairness (cmp_fair*.csv): `--ignore-checksum` local 1 GiB
  1011→227 ms (default hashing dominated); sftp small pull 5.0→2.7 s;
  its native unencrypted server (`serve webdav`) is worse than sftp
  on small files (10k push 315 s, pull 109 s) — sftp is rclone's
  best LAN transport; no rclone config approaches blit or rsync.

## Receive-side CPU (zero-copy revisit gate data, D-2026-06-12-1)

- Pull receiver (client, tmpfs sink): 0.45 cores at 9.5 Gbit/s
  (0.133 u + 0.315 s over 1.019 s wall).
- Push receiver (skippy daemon, ZFS sink): 127 cpu-ticks over 887 ms
  = **1.43 cores** at 9.5 Gbit/s — above the eval doc's "fraction of
  one core" estimate; far from saturation on 32 cores.

## Pull-side ceiling context

Client writes land on tmpfs (µs-class creates), so the 10k-pull wall
(446 ms blit / 367 ms rsyncd) is protocol + client per-file handling,
not storage — the ceiling class for this cell is ≪ 200 ms.
