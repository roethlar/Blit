# `blit bench` — benchmarking verb (deferred to 0.2.0)

**Status**: Historical

**Status:** Draft. Not in 0.1.0 scope. Captured here so it doesn't
get lost; ties to `RELEASE_PLAN_v2_2026-05-04.md` §2.6 (live
remote benchmark capture).

Author: mcoelho. Reviewed against GPT's framing of "destination
null sink alone doesn't isolate the network — for pure data
plane you also need a synthetic source."

---

## 1. Why a verb, not `copy --null`

R54-F1 narrowed `--null` to local-copy only because the flag was
silently broken or destructive elsewhere. The cleaner long-term
move is to remove `--null` and replace it with a dedicated
`blit bench` verb. Reasons:

- **Honest naming.** `copy` implies a write happens; `bench`
  implies it doesn't. The current `--null` confuses operators
  who reach for "why isn't the destination file there?"
- **Different output shape.** A copy emits a transfer summary;
  a benchmark emits structured timing/throughput metrics.
  Overloading copy means consumers parsing `--json` have to
  branch on a flag.
- **Different safety surface.** `blit bench` can refuse
  `mirror`/`move` semantics at the CLI grammar level (they're
  not bench subcommands) instead of as runtime args-rejection
  guards.
- **Different predictor channel.** Bench records feed a
  separate profile so they don't bias the predictor used by
  real transfers (see §6).

`--null` is removed in this work. The CLI rejections added in
R54-F1 disappear with the flag.

---

## 2. Subcommands

Two modes per GPT's framing:

### 2.1 `blit bench transfer <SRC> <DST>`

Real source reads, real network, destination uses `NullSink`.
Measures: source-side read pipeline + planning + network
(+ on remote paths, daemon-side pipeline) — but **not** the
destination disk write cost.

**Use case:** "Is the destination disk the bottleneck?"
Compare against a real `copy` of the same workload; the gap
is destination-write cost.

**Direction matrix:**

| SRC | DST | Behavior |
|---|---|---|
| local | local | FsTransferSource → NullSink locally. Trivial. |
| local | remote | Push the manifest as usual; daemon swaps `FsTransferSink` for `NullSink` on receive based on a new wire flag. |
| remote | local | Pull as usual; CLI swaps its `FsTransferSink` for `NullSink` locally. |
| remote | remote | Direct delegated pull (no `--relay-via-cli`); destination daemon receives via `NullSink`. |

Mirror/move semantics: not applicable. `bench transfer` is a
copy-shaped operation; the subcommand grammar doesn't expose
those modes.

### 2.2 `blit bench wire [--size=N] [--files=N] <SRC-HOST> <DST-HOST>`

Synthetic source bytes generated on the source daemon, streamed
through the normal data plane to the destination daemon's null
sink. Measures: pure daemon-to-daemon throughput, no filesystem
on either end.

**Use case:** "Can the data plane saturate this network link?"
Isolates §2.6's interesting question from any filesystem
variance.

Required: source-daemon synthetic generator (new minor feature).
Producer is bounded by an arithmetic byte counter; payloads are
zeroed buffers or a cheap PRNG-fill — enough to keep the wire
honest without disk involvement. Manifest is synthetic too
(generated `FileHeader` entries with the right sizes).

`--size` controls total bytes; `--files` controls payload count
(so the user can simulate "1×10 GiB" vs "10 000 × 1 MiB"
distributions, which exercise different code paths).

**Local-only?** No. `bench wire` only makes sense between two
daemons (or arguably localhost-to-localhost-on-two-daemons for
loopback testing). Reject the local→local case at the CLI with
a pointer at `bench transfer` for that direction.

---

## 3. Wire protocol changes

### 3.1 `TransferOperationSpec.discard_writes: bool`

New field on `TransferOperationSpec`. Bump
`SUPPORTED_SPEC_VERSION` from 2 → 3 so old daemons fail closed
instead of silently writing real data when a benchmark client
expects them to discard.

Honored by:
- `pull_sync` handler: the daemon is the *source* in pull — this
  field doesn't change its behavior. But carries through to
  `DelegatedPull` so the destination daemon honors it.
- Push handler: receives data from the CLI; when set, instantiates
  `NullSink` instead of `FsTransferSink` for receive.
- `DelegatedPull` handler on destination daemon: instantiates
  `NullSink` instead of `FsTransferSink` for its local writes
  during delegated remote→remote benchmark.

### 3.2 Push control header `discard_writes`

Push uses its own control protocol (`ClientPushRequest`). Add a
`discard_writes: bool` to the push start message (or another
existing init message — wherever target capabilities get
negotiated). Same semantic as the spec field.

### 3.3 New RPC: `BenchSynthesize` (for `bench wire`)

Source daemon-side endpoint. Request: `{ size_bytes,
file_count, payload_pattern }`. Stream of synthetic
`FileHeader` + payload bytes through the normal data-plane
machinery. Authorization: gated behind a `[bench]` config
section on the daemon (off by default; operator must opt in
since it lets remote clients consume CPU/network with no
filesystem read).

Alternative shape: piggyback on `pull_sync` with a new
"synthetic source" flag on the spec. Decide during impl —
piggyback is simpler if the existing flow accommodates fake
manifests cleanly.

---

## 4. Implementation tasks (per crate)

### blit-core

- [ ] Add `discard_writes: bool` to `TransferOperationSpec`
  proto + regenerate. Bump `SUPPORTED_SPEC_VERSION` → 3.
- [ ] Plumb `discard_writes` through
  `NormalizedTransferOperation::from_spec`.
- [ ] Tag `PerformanceRecord` with `kind: TransferKind` where
  `TransferKind = { Copy, Mirror, BenchTransfer, BenchWire }`.
  Bump the JSONL schema version, migrate older records via
  `migrate_record`.
- [ ] Synthetic source (for `bench wire`): a new
  `SyntheticTransferSource: TransferSource` that emits N file
  headers totaling M bytes of zero-fill (or a cheap PRNG fill
  if zeros compress trivially over wire) without touching the
  filesystem. Lives in `blit-core::remote::transfer::source`.

### blit-daemon

- [ ] Push receive path: honor push-control `discard_writes`
  by selecting `NullSink` for the receive pipeline.
- [ ] `DelegatedPull` handler: honor spec's `discard_writes`
  for its local sink choice.
- [ ] New config section `[bench]` with `allow_synthetic:
  bool` (default false) so operators must opt in to letting
  remote clients consume CPU + network for synthetic
  benchmarks.
- [ ] `BenchSynthesize` RPC or `pull_sync` synthetic-source
  flag — implementation TBD; honor `[bench].allow_synthetic`
  in either case.

### blit-cli

- [ ] New `Bench { command: BenchCommand }` clap subcommand:
  - `BenchCommand::Transfer { src, dst, ... }`
  - `BenchCommand::Wire { src_host, dst_host, size, files }`
- [ ] Remove `--null` flag entirely. Remove all R54-F1 CLI
  guards (they describe a flag that no longer exists).
- [ ] Update `cli_arg_safety_gates.rs` regression tests:
  remove `--null` cases, add `bench transfer` and
  `bench wire` happy-path + invalid-direction tests.
- [ ] Implement bench output formatting (human + `--json`),
  see §5.
- [ ] Update README.md and `docs/cli/blit.1.md`.
- [ ] Update `BENCHMARK_10GBE_PLAN.md` so the §2.6 playbook
  uses `bench transfer` and `bench wire` instead of any
  prior `--null` invocations (the doc currently doesn't
  reference `--null` so likely no change, but verify).

---

## 5. Output format

Both subcommands emit a structured benchmark report. Human
form is compact, single-block; JSON form is the same data
machine-readable.

**Common fields:**

```json
{
  "operation": "bench-transfer",     // or "bench-wire"
  "src": "<endpoint or 'synthetic'>",
  "dst": "<endpoint>",
  "files": 12345,                    // total entries processed
  "bytes_read": 17179869184,         // bytes read from source side
                                     //   (real or synthetic generation)
  "bytes_sent_wire": 17179869184,    // bytes transmitted on the wire
                                     //   (only differs from bytes_read for
                                     //   remote arms with tar-shard
                                     //   batching overhead)
  "bytes_received_dst": 17179869184, // bytes the destination receive
                                     //   pipeline saw
  "bytes_discarded": 17179869184,    // bytes the destination null-sink
                                     //   discarded (should equal received)
  "streams": 4,                       // data-plane stream count negotiated
  "tcp_fallback_used": false,         // whether the data plane fell back
                                     //   to gRPC
  "durations_ms": {
    "scan": 12,
    "plan": 8,
    "transfer": 4521,
    "total": 4541
  },
  "throughput_mibps": {
    "read": 3812.4,
    "wire": 3812.4,
    "received": 3812.4
  },
  "kind": "bench-transfer"           // for predictor tagging
}
```

Human form is a 6-line block:

```
benchmark: bench-transfer
  source:       /data/large-dataset (real reads)
  destination:  truenas:9031:/bench/ (null sink)
  workload:     12,345 files / 16.00 GiB
  duration:     4.54s (scan 12ms / plan 8ms / transfer 4521ms)
  throughput:   3.72 GiB/s wire (4 streams, TCP data plane)
```

---

## 6. Predictor integration

Bench runs ARE useful training data — they're the cleanest
signal for "how fast can this hardware actually move bytes"
free of filesystem variance. But they're NOT representative of
typical user workloads, so they shouldn't leak into the
predictor that estimates real-transfer durations.

Design: extend `PerformancePredictor` v3 (bump schema) with a
second profile bucket keyed on `TransferKind`. The
copy/mirror profile uses records where `kind ∈ {Copy,
Mirror}`; the bench profile uses records where `kind ∈
{BenchTransfer, BenchWire}`. Both update via the existing
gradient-descent observer; queries select the right bucket
based on what the caller is doing.

Concretely:
- `predictor.observe(record)` looks at `record.kind`, routes
  to the right bucket.
- `predictor.predict(...)` adds an `(intent: TransferKind)`
  parameter; copy/mirror callers pass their actual kind,
  bench callers pass theirs.
- `blit profile --json` surfaces both buckets.

This means the §2.6 benchmark capture (still hardware-bound)
also serves as the first training data for the predictor's
bench bucket — which is the model `bench wire` queries to
say "expected wire throughput for 16 GiB on this network is
~3.7 GiB/s based on N prior runs."

`kind` lives in the `PerformanceRecord` JSONL; record schema
version bumps from current value → +1 with a `migrate_record`
default of `Copy` for pre-existing records.

---

## 7. Test plan

- Unit:
  - `SyntheticTransferSource` emits the requested file/byte
    counts; payloads are deterministic enough to checksum if
    needed (for debugging) but generation cost stays trivial.
  - `discard_writes` field round-trips through proto serialize
    + `from_spec` normalize.
  - Predictor splits Copy/Bench profiles correctly: an observe
    on Bench doesn't move the Copy profile and vice-versa.
- Integration:
  - `blit bench transfer LOCAL LOCAL` works, emits the expected
    JSON shape, leaves dst empty.
  - `blit bench transfer LOCAL REMOTE` against a test daemon —
    daemon receives the bytes but writes nothing.
  - `blit bench transfer REMOTE LOCAL` — same but reversed.
  - `blit bench transfer REMOTE REMOTE` — delegated path; both
    daemons' filesystems untouched.
  - `blit bench wire` against two daemons with `allow_synthetic
    = true` — synthetic generation, null receive, throughput
    surfaces in JSON.
  - `blit bench wire` against daemons with default config —
    refused with a config-pointer error.
  - CLI grammar rejects `blit bench mirror` / `blit bench
    move` (they're not subcommands).

---

## 8. What this replaces

Removed by this work:
- `TransferArgs::null` and its three CLI guards from R54-F1
  (since `--null` no longer exists, the guards are dead code).
- The R52-F1 move guard for `--null` (same reason).
- The R54-F1 docstring on `--null`.

The existing `NullSink` in `blit-core::remote::transfer::sink`
stays — it's now reachable through `bench transfer` instead of
through `--null`-routed `local::run_local_transfer`.

---

## 9. Out of scope for 0.2.0

- **Latency/p99 measurements.** Throughput-only for now.
  Tail-latency benchmarking needs different instrumentation
  (per-payload timestamps, histogram aggregation) and is its
  own piece of work.
- **Concurrent benchmark runs.** Single bench operation at a
  time. Concurrent stress-testing is a separate verb.
- **Disk-only `bench read` mode.** GPT's earlier framing
  mentioned a source-read-only mode (no destination at all).
  Useful but easy to add later as `bench transfer SRC
  /dev/null` once we have a "local null destination" alias —
  defer until someone asks for it.
- **Comparison against rsync/scp** built into the verb.
  Operators can wrap `bench transfer` with their own
  comparison harness; `scripts/bench_10gbe.sh` already does
  this for the §2.6 workflow.

---

## 10. Sequencing

1. Land 0.1.0 with the current `--null` narrowing intact.
2. Capture §2.6 hardware-bound numbers (the existing playbook).
3. Spec field + proto bump → 3 (small commit; gates the daemon-side
   work).
4. Daemon-side null sink wiring for push receive and
   DelegatedPull (medium commit).
5. CLI verb scaffolding (`blit bench transfer` only, no
   synthetic-source yet — that gets you the destination-disk
   isolation measurement immediately).
6. `SyntheticTransferSource` + `bench wire` (separate commit
   so the wire benchmark can go through review on its own).
7. Predictor bucket split (separate commit; can land before
   or after the bench verbs).
8. Remove `--null` and its guards; update docs.

Cost estimate: ~3-4 days of focused work, plus the §2.6
capture wall-clock.
