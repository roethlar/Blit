# w6-2 — Progress residue (design map §1.6): verify-then-file

**Source**: Design-review queue row `w6-2-progress-residue-verify`
(`docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W6.2 — an
unverified-map-claims row: "Verification is step 1; each confirmed
item becomes its own follow-on slice").
**Severity**: Medium (RELIABLE — three classes of silently-dead or
zero-valued progress surfaces).
**Nature**: verification + filing slice — no code change. The three
follow-on findings are filed as `w6-2a/-2b/-2c` in `REVIEW.md`'s
filed-findings section (same entry route as `design-1..5`).

## Verification method

Claims re-derived twice at HEAD `8fd8978`: first by the w6-1 5-agent
mapping workflow (2026-07-04, producer/consumer/daemon-boundary
censuses + a gapcheck critic that re-grepped every hit), then
spot-confirmed by hand at the exact sites below. All three §1.6
residue claims are **CONFIRMED**; none refuted.

## Claim 1 — CONFIRMED: delegated transfers show zero live progress (`BytesProgress` is wire-dead)

- `proto/blit.proto:895` defines `BytesProgress` (cumulative
  files/bytes, field 3 of the `DelegatedPullProgress` oneof at :873).
- Consumers are live: `blit-app/src/transfers/remote.rs:768`
  (`DelegatedPayload::BytesProgress` arm → `report_bytes_progress`
  delta bridge, the contract's aggregate lane) + 2 unit tests; TUI
  delegated forwarder folds whatever arrives.
- **Producers: zero.** `grep -rn BytesProgress crates/` hits only the
  proto and blit-app — no code in `blit-daemon` (or anywhere)
  constructs the message. `delegated_pull.rs` sends `Started`, then
  nothing during the transfer (comment at :363-369 records the
  deliberate 0.1.0 gap), then one post-hoc
  `ManifestBatch{file_count = summary.files_transferred}` (:433) and
  the summary.
- Net: CLI delegated progress prints only a terminal
  "manifest enumerated N file(s)" line; the TUI delegated footer stays
  0/0 for the whole transfer. The destination daemon ALREADY meters
  the bytes (its row atomic is fed via `core.rs:667` →
  `pull_sync_with_spec(byte_progress)`), so the fix is a bridge, not
  new instrumentation.
- **Filed**: `w6-2a-delegated-bytesprogress-producer`.

## Claim 2 — CONFIRMED: daemon byte counters stay 0 for push and pull_sync rows

- `job.bytes_counter()` is wired exactly once in service code:
  `core.rs:667`, the delegated_pull dispatch.
- Push receive: `push/data_plane.rs` builds its `FsTransferSink`
  without `.with_byte_progress()` and calls
  `execute_receive_pipeline(.., None)` (:1086) — neither lane feeds
  the row.
- PullSync serve: `pull_sync.rs` passes `None`/no counter at all three
  send pipelines (:635 gRPC, :765 fixed, :795-801 elastic).
- Net: for 2 of 3 active transfer kinds, `GetState.bytes_completed`,
  the 10 Hz `TransferProgress` broadcasts, and
  `TransferComplete.bytes` are all 0 — `blit jobs watch` on a push
  shows `bytes=0` until a zero-valued terminal event.
- **Filed**: `w6-2b-daemon-counters-push-pullsync`.

## Claim 3 — CONFIRMED: no denominators (or file counts) end-to-end on the daemon event stream

- `core.rs:240-242`: every `TransferProgress` broadcast hardcodes
  `bytes_total: 0, files_completed: 0, files_total: 0` ("land in
  follow-up C sub-slices").
- `core.rs:322-325`: `TransferComplete.files` hardcoded 0.
- `core.rs:994-996`: `GetState` rows hardcode `bytes_total: 0`.
- Net: no consumer of the daemon event stream can render "N of M" or
  a percentage; the TUI F2 percent column and `jobs watch` totals run
  on bytes_completed alone.
- **Filed**: `w6-2c-daemon-progress-denominators`.

## Sequencing note (recorded in the filed rows)

2b is the substrate: 2a bridges the same row atomic 2b feeds, and 2c's
`files_completed` wants the same per-row counter family. Suggested
order 2b → 2a → 2c, but each is independently landable; final
sequencing is the coder's pick per queue policy unless the owner
orders otherwise. All three touch the daemon only — the client-side
contract (w6-1) needs no changes to absorb them (the delegated bridge
and `ProgressTotals` already handle the aggregate lane).

## Files

- `.review/findings/w6-2-progress-residue-verify.md` (this record).
- `REVIEW.md` — w6-2 row closed; three follow-on rows filed.

## Tests

None — no code change; the docs gate (`scripts/agent/check-docs.sh`)
is the validation surface per D-2026-07-04-1.

## Known gaps

- The three fixes themselves are deliberately NOT in this slice (the
  ratified row defines verification + filing as the deliverable).
- `TransferComplete.tcp_fallback_used` is also hardcoded `false`
  (core.rs:329-330) — same comment family but not a §1.6 claim; noted
  here so it isn't lost, folded into w6-2c's scope as the terminal
  event's honesty is one surface.
