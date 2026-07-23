# SSD-backed direct-Thunderbolt comparison

**Status**: Complete — both one-shot arms, all hash checks, the evidence
commit, and exact-path cleanup are complete
**Created**: 2026-07-22
**Depends on**: D-2026-07-22-4 and the certified RAM/wire probe in
`docs/bench/thunderbolt-macmac-2026-07-22/README.md`
**Supersedes**: nothing
**Owner authorization**: leave the direct cable connected; run the SSD-backed
follow-up only after writing this plan; up to 40 GB may be written when it
materially improves the evidence

## Owner decision

### D1 — Approved one-shot write budget and direction

Use Q's internal APFS SSD as SOURCE and Nagatha's internal APFS SSD as
DESTINATION, preserving the Q -> Nagatha direction from the RAM comparison.
Create one 12 GiB physically allocated source fixture, then run exact-candidate
Blit once and Apple openrsync once into separate fresh 12 GiB destination
trees. Hard cap benchmark payload writes at 36 GiB total: 12 GiB on Q plus
24 GiB on Nagatha. This is 38.7 decimal GB, leaving about 1.3 GB beneath the
owner's 40 GB ceiling. No automatic retry or repeat is permitted.

**Status**: approved 2026-07-22 — owner: **"you can write up to 40gb if that
will give you more data"**. Twelve whole-GiB files are the largest version of
the retained file shape that remains below 40 decimal GB across three writes.

## Goal

Measure the RAM probe's same 1 GiB large-file shape, extended from eight to
twelve files, with real SSD reads and writes on both endpoints. Record:

- exact-candidate Blit wall time and throughput;
- unencrypted Apple openrsync wall time and throughput;
- Blit's SSD-backed throughput relative to the RAM arm's 28.6 Gb/s;
- byte-for-byte destination integrity; and
- actual bounded payload-write accounting.

The result is an exploratory one-shot comparison. It determines whether the
remaining direct-Thunderbolt headroom is still visible with the Macs' internal
storage in the path; it is not a release acceptance matrix.

## Non-goals

- Tuning or changing Blit code.
- Revalidating the already certified 37.7–37.9 Gb/s TCP ceiling.
- Testing the reverse direction, initiator-layout parity, small files, mixed
  trees, resume, cold-versus-warm policy, or repeated-run variance.
- Comparing against rsync-over-SSH or a separately installed GNU rsync.
- Turning an optional throughput ceiling into a release gate.

## Fixed environment

- SOURCE: Q, Mac mini `Mac16,10`, 16 GiB, internal APFS SSD, current free
  space about 32.9 GiB.
- DESTINATION: Nagatha, MacBook Pro `Mac16,5`, 48 GiB, internal APFS SSD,
  current free space about 124 GiB.
- Route: Q `172.31.254.2/30` -> Nagatha `172.31.254.1/30`, `bridge0`, MTU
  1500, direct negotiated 40 Gb/s Thunderbolt.
- Product: the ARM macOS CI artifact from run `29953569652`, exact candidate
  `d1f1152dd16b8c2bf8409cb5637135e3f89068c0`; archive SHA-256
  `d1d7d9e547f703a7b5216cb3227baaf6b2bea848a85599312439cdccff19b726`;
  `blit` SHA-256
  `dc3cd55ad10903ef695db904f97ea3f6c0c7e6a300e4a163b95e766bced3cca0`;
  `blit-daemon` SHA-256
  `652a8e641d1211ab9d4a254f56b6f2d9db0626c71a39d8861fee522ebbc74018`.
- Comparator: `/usr/bin/rsync` on both Macs, openrsync protocol 29 / rsync
  2.6.9 compatible, daemon transport bound only to the isolated Thunderbolt
  address, compression off by default, no SSH or authentication overhead.

## Safety and evidence constraints

- D1 is a hard write ceiling, not an estimate. The 36 GiB payload budget is
  38.7 decimal GB. Tooling, configs, and logs may add at most 100 MiB, keeping
  total plan-created writes below 40 decimal GB.
- A partially written failed arm counts against the budget. Stop immediately,
  retain its output and logs, and ask the owner before any retry.
- No A-B-B-A, second sample, reverse arm, larger fixture, or analyzer-driven
  rerun. Ambiguity is reported as ambiguity.
- Use fresh, session-unique roots under
  `/Users/michael/blit-bench-work/` on both hosts. Preflight must prove each
  root does not exist. Never reuse or recursively clean an older root.
- Create twelve physical 1 GiB source files from Q's retained seed
  `/Users/michael/blit-bench-work/src_large/large_1024M.bin`. Do not use APFS
  clones. Record free blocks before and after and require physical allocation
  to grow by 11.5–12.5 GiB; otherwise stop before transfer.
- The seed and all twelve source files must be exactly 1,073,741,824 bytes and
  hash to
  `cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
- Flush fixture writes, hash the fixture, then run Q's authorized
  `/usr/sbin/purge` immediately before each timed arm so both tools read the
  physical 12 GiB source rather than inheriting the fixture-generation cache.
- Hold both 12 GiB destination trees until both have been independently hashed.
  This fits Nagatha's current free-space margin and prevents a failed cleanup
  from destroying the only result before comparison.
- During timed arms, run no build, test, review, indexing request, or other
  deliberate load on either Mac. The orchestrating session only waits for the
  active process.
- Persist raw stdout, stderr, `/usr/bin/time -p`, hashes, route/link facts,
  free-space snapshots, exact commands, and exit codes before interpretation.
  Evidence rejection never triggers a transfer rerun; repair the interpreter.

## Preflight — no payload writes

1. Prove the product artifact and both executable hashes above; copy only the
   two exact executables needed for the run.
2. Re-prove both Macs' model, memory, macOS build, APFS/internal-solid-state
   identity, and free space. Require at least 30 GiB free on Q before fixture
   creation and 48 GiB free on Nagatha before either destination write.
3. Re-prove `172.31.254.1/30` and `.2/30`, both routes on `bridge0`, MTU 1500,
   peer ARP identity, direct 40 Gb/s negotiation, and a source-bound ping each
   way. Do not rerun the 60-second iperf suite while the cable remains stable.
4. Prove Q has passwordless authorization only for `/usr/sbin/purge`; do not
   broaden sudo rights. Prove no stale listeners occupy Blit or rsync ports.
5. Record the current branch, exact candidate identity, source seed identity,
   clock/timezone, filesystem free blocks, and active high-CPU processes.
6. Create the evidence directory and empty session-unique source/destination
   roots. Any failed identity, space, route, permission, or emptiness gate
   stops before fixture creation.

## Fixture construction — 12 GiB Q SSD write

1. Copy the retained 1 GiB seed normally—not with `cp -c`—into twelve files.
   Use an operation that forces ordinary file writes, such as one bounded
   `dd` from the seed into each fresh destination file.
2. `sync`, record Q free blocks, and require the 11.5–12.5 GiB allocation delta.
3. Record size and SHA-256 for all twelve source files. Any mismatch stops.
4. Do not create a second fixture. The same immutable tree feeds both tools.

## Timed arm A — Blit, 12 GiB Nagatha SSD write

1. Start exact-candidate `blit-daemon` on Nagatha, bound only to
   `172.31.254.1:19031`, with its default root set to the fresh Blit
   destination. Disable mDNS; otherwise retain production defaults.
2. Prove readiness and exported-root identity without moving data.
3. On Q: `sync`, run `/usr/sbin/purge`, recheck the route, then time exactly
   one `blit copy SOURCE/ 172.31.254.1:19031:/default/ --yes --json`.
4. Require exit 0, twelve files, exactly 12,884,901,888 transferred bytes, and
   `tcp_fallback: false`. Stop the daemon cleanly.
5. Hash and size every Blit destination file against the source list. Preserve
   raw logs and the complete destination tree.

## Timed arm B — openrsync, 12 GiB Nagatha SSD write

1. Use a session-local rsync daemon config naming only the fresh rsync
   destination, `read only = false`, `use chroot = false`, one connection, a
   lock file inside the session root, and `hosts allow = 172.31.254.2`.
2. Bind the daemon only to `172.31.254.1:18730`; prove the sole expected module
   is visible from Q before timing.
3. On Q: `sync`, run `/usr/sbin/purge`, recheck the route, then time exactly
   one `/usr/bin/rsync -a --whole-file --inplace --stats --port=18730
   SOURCE/ rsync://172.31.254.1:18730/tb/`. Do not pass the unsupported
   `--no-compress`; compression is already off without `-z`.
4. Require exit 0, twelve transferred files, exactly 12,884,901,888 bytes of
   unmatched/transferred data, and no partials. Stop the daemon cleanly.
5. Hash and size every rsync destination file against the source list.

## Analysis and verdict

Calculate throughput from the exact 12,884,901,888-byte payload and external
wall time. Report, without rounding away uncertainty:

- Blit and rsync seconds, GB/s, and Gb/s;
- Blit/rsync elapsed-time ratio;
- Blit SSD throughput versus the prior 28.6 Gb/s RAM result;
- the 37.9 Gb/s same-direction TCP ceiling as context, not a pass threshold;
- exact source and destination hashes; and
- planned versus observed allocation/write accounting.

Do not grade a pass/fail performance threshold and do not infer a code cause
from timing alone. A material SSD penalty or remaining wire gap becomes input
to a separate read-only attribution and tuning plan.

## Cleanup

Cleanup occurs only after hashes, raw logs, interpretation, and the evidence
commit are complete. Stop all temporary listeners. Validate each generated
root contains only the twelve named 1 GiB payloads plus plan-declared tiny
config/log files. Unlink those exact files and remove the now-empty roots; do
not use a broad recursive target or touch the retained 1 GiB seed. Remove
temporary candidate binaries after rechecking their hashes. Retain the static
Thunderbolt addresses while the cable stays connected.

## Acceptance criteria

- [x] D1 is approved and the plan is Active before the first fixture byte.
- [x] All preflight identities and free-space gates pass.
- [x] Physical source allocation and total benchmark payload writes stay
      within the approved 36 GiB / 38.7 GB hard cap.
- [x] Exactly one Blit and one rsync data-moving arm run; no retry or repeat.
- [x] Both tools transfer exactly twelve files / 12,884,901,888 bytes and every
      destination hash matches its source.
- [x] Raw evidence and limitations are committed before generated data is
      removed.
- [x] All listeners and generated data are removed with the seed and unrelated
      paths untouched.
- [x] No product code, release candidate, tag, remote ref, or publication state
      changes as part of this plan.
