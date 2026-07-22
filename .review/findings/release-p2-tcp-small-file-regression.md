# release-p2-tcp-small-file-regression — restore TCP small-file overlap

**Severity**: HIGH — retained same-session evidence measured unified TCP
small-file push 10–20% slower than the old path on both rigs.
**Status**: Fixed locally
**Branch**: `master`
**Commit**: This rel-2 implementation commit

## Evidence and cause

The retained `push_tcp_small` medians were 2080/1811 ms (1.149) on
netwatch-01 and 1975/1644 ms (1.201) in 12c-win; zoey measured 1.105. The
same gRPC cell ranged from 0.801 to 1.001, so the regression signature was
specific to the TCP small-file path.

Exact historical code at `0f922de` handled each `NeedBatch` during the open
manifest and immediately queued the authorized TCP payload. The unified
source instead awaited the next scan entry independently of its receive-half
events and deferred all payload planning until after `ManifestComplete`. A
NeedBatch could therefore be available while the TCP workers remained idle;
scan, destination diff, and transfer no longer overlapped.

The retained observer also proved a second executed TCP-only delta. Unified
TCP tar receive claimed the need-list mutex once per member, while the
in-stream path claimed once per shard and the old served-push receive path
bypassed `NeedListSink` entirely.

## Fix

The source scan loop now services control events while waiting for the next
manifest entry. For an ordinary TCP copy, it plans and queues each authorized
batch immediately. In-stream payload, mirror operations, and
`require_complete_scan` operations still wait for `ManifestComplete`; their
pre-write refusal and deletion guarantees are unchanged. The early and
post-manifest TCP paths share one planning/queue helper.

TCP tar receive now claims every member in one shard under one outstanding-set
lock, preserving the prior ordered rejection behavior while eliminating the
per-file lock/unlock cycle.

## Guard proof

`tcp_payload_overlaps_open_manifest_under_either_initiator` pauses enumeration
after 128 of 129 one-byte files and observes a destination file complete before
the gate permits `ManifestComplete`. It passes with SOURCE and DESTINATION
initiating. Disabling the overlap condition made the guard fail with “TCP
payload waited for ManifestComplete”; restoration returned it green.

`small_file_probe_is_complete_and_inert_across_roles_and_carriers` now requires
TCP claim-lock acquisitions to equal tar-shard count. The retained fixture
observed 5 locks for 256 members. Restoring the per-member loop made the guard
fail at 256 versus 5; restoration returned it green.

No hardware throughput matrix or large-write test was run. The numerical
ceiling remains unremeasured; release closure rests on the exact old/new code
delta and mutation-sensitive ordering/operation-count proofs, as required by
D-2026-07-22-3.

Final local validation passed formatting, strict workspace clippy, all
workspace tests, and the docs gate.
