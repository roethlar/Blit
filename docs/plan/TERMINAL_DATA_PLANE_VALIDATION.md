# Terminal data-plane hardware validation

**Status**: Historical — canceled before listener or payload
**Created**: 2026-07-23
**Supersedes**: nothing
**Decision ref**: D-2026-07-23-5

## Goal

Run the accepted terminal data-plane observer exactly once on the existing
Q-to-Nagatha Thunderbolt path. The retained terminal payload and blocked-write
totals must classify whether SOURCE socket writes consumed the slow sample's
available four-stream time, so the next 10, 25, and 40 Gb/s investigation is
selected from evidence rather than another transfer guess.

## Non-goals

- No transfer-policy, stream-count, buffer, prefetch, socket, filesystem,
  carrier, planner, protocol, CLI, or telemetry implementation change.
- No iperf, rsync, SSD-destination, comparison, cold-cache, repeat, retry,
  performance matrix, release, package, tag, publication, or push.
- No claim that blocked-write elapsed time is pure kernel blocking. It retains
  the observer's existing meaning: time awaiting payload `write_all` calls.
- No source-read or destination-write subdivision unless the terminal result
  proves material time remained outside socket writes.

## Constraints

- Build exact clean candidate HEAD after plan activation and prove its product
  tree contains accepted terminal observer head
  `6507444dbc839cf6c5d4392b1f50aa4cf1f9832a` without a later product diff.
- Use Q `172.31.254.2` as SOURCE and Nagatha `172.31.254.1` as DESTINATION over
  `bridge0`, MTU 1500. Stop before payload if either endpoint, route, interface,
  link, candidate identity, listener, or RAM-disk check differs.
- Build both binaries on a Nagatha RAM disk. Stage only the exact client on Q.
  Temporarily allowlist only the exact RAM-disk daemon in the macOS Application
  Firewall before the read-only control-plane preflight, then remove it.
- Expose the retained 1 GiB Q seed as eight APFS clones. Verify the seed once,
  verify clone sizes and allocation, and do not reread every source clone for
  preflight hashing. Q payload is read-only.
- Create a fresh 12 GiB Nagatha APFS RAM disk for the destination. Exactly one
  transfer process may move exactly 8,589,934,592 payload bytes. No automatic
  retry or second payload invocation is authorized.
- Q SSD writes are limited to APFS clone metadata, one staged client, and small
  text evidence, together below 32 decimal MB. No benchmark payload may be
  written to either SSD.
- Enable `BLIT_TRACE_SESSION_PHASES=1` and one unique `BLIT_TRACE_RUN_ID` on
  both endpoints. Retain complete client, daemon, identity, preflight,
  integrity, allocation, firewall, and cleanup evidence.
- After the run, stop the daemon, remove the temporary firewall entry, remove
  Q's clones and staged client, detach both RAM disks, and prove the retained
  seed and static Thunderbolt addresses remain intact.

## Acceptance criteria

- [ ] Exact candidate identity, binary hashes, clean product diff, endpoint
      route/link state, empty destination, listener, firewall allowlist, source
      seed identity, clone allocation, and SSD-write budget pass before timing.
- [ ] One client invocation exits zero, reports eight files,
      8,589,934,592 bytes, and `tcp_fallback: false`; all eight destination
      files have the retained seed's exact size and SHA-256.
- [ ] Exactly one SOURCE `dial_terminal_sample` appears after the send pipeline
      joins and before SOURCE `data_plane_complete`. Its terminal payload bytes
      equal 8,589,934,592 and its terminal stream count covers the fixed four
      streams.
- [ ] Offline analysis divides `terminal_blocked_ns` by the four-stream
      first-write-to-data-complete capacity. At least 75% is classified
      socket/backpressure-dominant; below 50% is classified off-socket-dominant;
      50–75% is mixed. The next investigation follows that class without
      changing transfer policy in this plan.
- [ ] Client/daemon process time, lifecycle spans, body throughput, terminal
      blocked fraction, destination tail, observer limits, SSD allocation, and
      comparison with the retained 35.578 and 19.153 Gb/s samples are recorded.
- [ ] Cleanup proves no listener, temporary firewall entry, Q session root,
      staged binary, destination RAM disk, or build RAM disk remains. The
      retained seed still has its exact size, allocation, and SHA-256.
- [ ] Plan activation, raw evidence, attribution, plan/state closure, and
      cleanup are committed. Nothing is pushed, tagged, released, or published.

## Interpretation

The fixed-membership capacity is:

`4 × (SOURCE data_plane_complete elapsed_ns - earliest SOURCE socket_write_begin elapsed_ns)`

The terminal blocked fraction is:

`terminal_blocked_ns / fixed-membership capacity`

A high fraction directs the next observer or tuning plan to network/receiver
backpressure. A low fraction directs it to source reads, queueing, or
scheduling. The middle band is recorded as overlapping causes and does not
justify a tuning change by itself.

## Slices

1. **tdpv-1 — activation `[x]`.** Record the one-run limits, interpretation
   rule, owner approval, and exact cleanup requirements.
2. **tdpv-2 — canceled before observation `[x]`.** The candidate was built and
   hashed on RAM, but the owner declined the new administrator authorization
   required for the exact daemon's Application Firewall entry. No listener,
   daemon, client invocation, or payload transfer ran. All temporary state was
   removed and the plan closed without an attribution result.

## Cancellation and cleanup

Exact candidate `7aef1fedaf2f8721c009ec773a69a94df4860791` had no
product diff after accepted observer head `6507444d`. Its RAM-built client was
SHA-256 `9441c132c6bf24018d9afdf27829864fcdeaab43238fba98ec8378b5f4725f4b`;
its daemon was
`10cb2394b081b586d5ebf2aa007d53257f9ea9c45021b66ccc7735b5e2f2737b`.
Both identified as `0.1.1+7aef1fedaf2f`.

The direct peer initially was physically absent. After the cable was reseated,
both bridges recovered their retained `/30` addresses, became active, and
passed bidirectional zero-loss pings. Nagatha then created fresh build and
destination RAM disks; Q created eight APFS clones and staged the 11,118,496
byte client. No benchmark payload was allocated or moved.

The exact new RAM daemon had no Application Firewall entry. The CLI add attempt
stopped at expired administrator authorization, and the later standard
authorization dialog was canceled by the owner before its command ran. The
daemon never started, port 19041 never listened, the destination remained
empty, and no client transfer process ran.

Cleanup removed Q's exact session root, staged client, and eight clones; the
retained seed remained 1,073,741,824 bytes with 2,097,152 512-byte blocks.
Nagatha detached the exact 12 GiB destination and 16 GiB build RAM devices.
There is no current-candidate firewall entry or listener.

A read-only firewall inventory exposed an older entry for
`/Volumes/BLIT_ETL_BUILD/etl3-review/target/release/blit-daemon`, conflicting
with the historical ETL cleanup record. Its provenance after that recorded
check is unestablished. It was neither reused nor removed; exact administrator
approval would be required to remove it.

## Open questions

- None.
