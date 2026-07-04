# w6-1 — ProgressEvent contract: semantics defined in blit-core, producers normalized, one shared accumulator

**Source**: Design-review queue row `w6-1-progress-event-contract`
(`docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W6.1; evidence
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`
`boundaries-progress-event-contract-lives-in-consumers` +
`duplication-progress-folding-rules`). Incorporates and closes filed
**design-1** (`design-1-cli-pull-byte-double-count`) structurally.
**Severity**: Medium (RELIABLE — user-visible wrong numbers; structural
cause of the design-1 class).

## What

`ProgressEvent` had no owner-defined semantics: three producer families
assigned three incompatible meanings to the same variants (TCP pull
receive emitted the same bytes on BOTH `Payload` and `FileComplete`;
push send put bytes on `FileComplete` only; delegated put both deltas on
`Payload`), so every consumer hard-coded per-direction folklore — the
TUI kept three folding rules, and the CLI's single generic rule
double-counted bytes on TCP pulls (design-1).

This slice defines the contract in blit-core, normalizes every producer
to it, and collapses all consumer folds into one shared accumulator:

- **Bytes ride `Payload` only.** `FileComplete`'s `bytes` field is
  **deleted from the enum** (`FileComplete { path }`), and
  `report_file_complete(path)` loses its byte parameter — the design-1
  double-count class is now unrepresentable at the type level, not just
  avoided by convention.
- **Files count exactly once via one of two lanes**: one byteless
  `FileComplete { path }` per file (per-file lane — producers that see
  individual files), or `Payload { files: delta }` (aggregate lane —
  the delegated `BytesProgress` bridge and tar-shard batch appliers).
- **`FileComplete.path` is the source-relative wire path** — the gRPC
  pull previously leaked the absolute local destination path.
- **`ManifestBatch` is the denominator only** — documented as
  direction-flavored (pull: full source manifest; push: need-list;
  delegated: post-hoc summary), never added to transferred totals.
- **`ProgressTotals`** (blit-core, next to the enum) is the one shared
  fold: `files += Payload.files + 1 per FileComplete`,
  `bytes += Payload.bytes`, `manifest_files += ManifestBatch.files`,
  saturating; `started()` gates live-totals display. The TUI's three
  per-direction accumulators are deleted; the CLI monitor and all three
  TUI forwarders fold through it.

## Producer normalization (all emit sites)

| Site | Before | After |
|---|---|---|
| `pipeline.rs` receive FILE | `Payload{0,N}` + `FileComplete{path,N}` (double) | `Payload{0,N}` + `FileComplete{path}` |
| `pipeline.rs` receive TAR_SHARD | `Payload{0,archive}` only (members never counted) | + one byteless `FileComplete` per member |
| `pipeline.rs` receive BLOCK / BLOCK_COMPLETE | nothing (TCP resume invisible) | `Payload{0,bytes_written}` / `FileComplete{path}` |
| `pipeline.rs` send worker (push TCP + gRPC fallback) | `FileComplete{path,size}` only | `Payload{0,size}` + `FileComplete{path}` per file |
| `pull.rs` gRPC FileData/TarShardChunk/BlockTransfer | `Payload` chunks (conformant) | unchanged |
| `pull.rs` `finalize_active_file` | `FileComplete{ABSOLUTE dest path, 0}` | `FileComplete{wire-relative path}` (path carried through `active_file`) |
| `pull.rs` gRPC TarShardComplete | nothing (members never counted) | `Payload{stats.files, 0}` (aggregate lane) |
| `pull.rs` gRPC BlockComplete | nothing | `FileComplete{wire path}` |
| `blit-app` delegated `report_bytes_progress` | `Payload{fΔ,bΔ}` (conformant) | unchanged; documented as the aggregate lane |
| `data_plane.rs` `send_payloads_with_progress` (dead, 0 callers) | bytes on `FileComplete` | conformed (normalize, deletion is w8 territory) |
| `payload.rs` `transfer_payloads_via_control_plane` (dead, w8-2 target) | bytes on chunk `Payload`s AND `FileComplete` (latent double-count) | conformed |

## Consumer collapse

- `blit-cli/src/transfers/remote.rs` monitor: hand-rolled three-arm fold
  → `ProgressTotals` (fixes design-1's ~2× `[progress]` bytes on TCP
  pulls). JSON `file_complete` events keep their key shape with
  `"bytes":0` for stream compatibility (per-event bytes no longer
  exist; they were already 0 on gRPC pulls).
- `blit-tui/src/progress_accum.rs`: `accumulate_pull_progress` /
  `accumulate_push_progress` / `accumulate_delegated_progress` deleted;
  `pull_throughput` / `du_total_from_entries` remain. All three
  forwarders in `main.rs` fold through `ProgressTotals`.

## Files

- `crates/blit-core/src/remote/transfer/progress.rs` — contract docs,
  enum change, reporter API, `ProgressTotals`, 6 contract tests.
- `crates/blit-core/src/remote/transfer/mod.rs` — re-export.
- `crates/blit-core/src/remote/transfer/pipeline.rs` — receive + send
  emission normalization; 4 producer emission tests + `RecordingSink`.
- `crates/blit-core/src/remote/transfer/data_plane.rs`,
  `crates/blit-core/src/remote/transfer/payload.rs` — dead-producer
  conformance.
- `crates/blit-core/src/remote/pull.rs` — wire-relative completion path,
  tar-shard/resume gap fills, 2 `finalize_active_file` tests.
- `crates/blit-app/src/transfers/remote.rs` — aggregate-lane doc.
- `crates/blit-cli/src/transfers/remote.rs` — monitor on
  `ProgressTotals`.
- `crates/blit-tui/src/progress_accum.rs`, `crates/blit-tui/src/main.rs`
  — fold collapse; 7 accumulator tests rewritten against
  `ProgressTotals` (count preserved).

## Tests

- +12 new blit-core tests (6 `ProgressTotals` contract, 4 pipeline
  emission incl. the design-1 producer pin, 2 finalize), TUI 7
  rewritten in place. Workspace suite green; count grew (see gate run
  in the verdict record).
- Mutation checks run (each reverted before commit): dropping the
  `files` increment in `ProgressTotals::apply`'s `FileComplete` arm
  fails the contract tests; dropping the receive pipeline's
  `report_payload` fails
  `receive_pipeline_reports_payload_bytes_then_byteless_file_complete`;
  re-adding bytes to `FileComplete` no longer compiles anywhere.

## User-visible changes (intentional, per contract)

- CLI `[progress]`/final lines on TCP pulls stop showing ~2× bytes
  (design-1). Live file counts now include tar-shard members and
  resumed files.
- CLI `--json` `file_complete` events report `"bytes":0` always
  (previously: real bytes on TCP pull / push, 0 on gRPC pull).
- CLI `--verbose` per-file lines on gRPC pulls print the wire-relative
  path (previously the absolute local destination path; TCP pulls
  already printed wire-relative).
- TUI push footer bytes now arrive via `Payload` (same values, same
  totals).

## Known gaps

- `data_plane.rs` `send_payloads_with_progress` emission is normalized
  but not covered by a dedicated test (zero live callers; needs a TCP
  socket harness — deletion belongs to the w8 dead-code sweeps).
- Daemon-side residue (push/pull_sync rows broadcasting 0 bytes,
  delegated `BytesProgress` wire-dead, no denominators end-to-end) is
  w6-2 by design — this slice deliberately does not touch daemon
  counters or wire messages.
- `ManifestBatch`'s three direction-flavored meanings are documented,
  not unified — unification would be a wire/UX decision beyond this
  slice's scope.
