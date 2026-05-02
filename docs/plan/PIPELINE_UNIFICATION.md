# Pipeline Unification: One pipeline, three roles

**Status:** Planning. Drafted 2026-05-01 in response to GPT-5.5's review of
the universal-filter work and ensuing architectural discussion.
**Supersedes** the implicit "push and pull are different" model.

## Premise

A transfer involves three roles. They aren't always on the same machine, and
the protocol shouldn't pretend they are.

| Role | Responsibility |
|---|---|
| **Initiator** | Parses CLI args, builds a normalized operation spec, coordinates start/end. Does not handle bulk data. |
| **Origin** | Holds the source data. Enumerates, plans, prepares payloads, sends bytes. |
| **Target** | Holds the destination filesystem. Receives bytes, writes files, optionally enforces mirror deletion. |

This is the architecture. Whichever side a CLI invocation happens on, the
appropriate side does its appropriate job. Push, pull, local-local, and
remote-remote stop being separate code paths and become **direction-agnostic
arrangements of the same three roles**.

## Where each role lives in each operation

| Operation | Initiator | Origin | Target |
|---|---|---|---|
| local copy/mirror | CLI | CLI | CLI |
| push (local→remote) | CLI | CLI | daemon |
| pull (remote→local) | CLI | daemon | CLI |
| remote→remote | CLI | daemon A | daemon B |

The CLI's job in pull is **not** to do enumeration or comparison work — it's
to ship a normalized operation spec to daemon A and then stand by as a
receive-side sink. Daemon A runs its source pipeline and streams to daemon B
(or back to the CLI when daemon B *is* the CLI).

## The pipeline (same on every origin)

```
TransferSource → DiffPlanner → execute_sink_pipeline_streaming → TransferSink
                  ─────────                                       ─────────
                   origin                                          target
```

Every role plays its part:

- **TransferSource** enumerates the origin's filesystem. Already abstracted
  (`FsTransferSource`, `RemoteTransferSource`).
- **DiffPlanner** *(new — extraction work)* takes:
  - The source header stream
  - The user filter (`FileFilter`)
  - The target's destination manifest stream (what the target already has)
  - Comparison mode (size+mtime / hash / size-only / ignore-times)
  - Capabilities (resume support, server checksums, etc.)

  And emits a stream of `TransferPayload` variants:
  - Drop (filtered out, or unchanged on target) → no payload
  - Full file (missing on target) → `TransferPayload::File`
  - Block-level delta (changed but resume-eligible) → `FileBlock` + `FileBlockComplete`
  - Small file batch → `TarShard`

- **execute_sink_pipeline_streaming** dispatches payloads to one or more
  sinks. Already exists.
- **TransferSink** writes to the target. Already abstracted (`FsTransferSink`,
  `DataPlaneSink`, `GrpcFallbackSink`, `NullSink`).

The crucial correction: **the unified stage is `DiffPlanner`, not
`FilteredSource`**. `FilteredSource` is a special case that handles only
the user-filter input. The full diff requires destination-manifest +
comparison-mode + capabilities to produce the right payload types,
particularly for resume.

## The wire contract: `TransferOperationSpec`

The initiator → origin handshake carries a normalized intent message,
not a flag bag. Sketch:

```protobuf
message TransferOperationSpec {
  // Source path on the origin (relative to the origin's exported root).
  string source_path = 1;

  // User-supplied filter rules. Identical shape used by push, pull,
  // local-local, and remote-remote.
  FilterSpec filter = 2;

  // How to decide if a target-existing file matches: size+mtime
  // (default), checksum, size-only, ignore-times.
  ComparisonMode compare_mode = 3;

  // Mirror behavior: off | filtered_subset | all (delete scope).
  MirrorMode mirror_mode = 4;

  // Resume settings: enabled, block size, hash algorithm.
  ResumeSettings resume = 5;

  // Negotiated capabilities of both peers (server checksums available,
  // tar shard support, data-plane TCP support, etc.).
  PeerCapabilities capabilities = 6;

  // Protocol/spec version for forward compatibility.
  uint32 spec_version = 7;
}

message FilterSpec {
  repeated string include = 1;
  repeated string exclude = 2;
  optional uint64 min_size = 3;
  optional uint64 max_size = 4;
  optional uint64 min_age_secs = 5;
  optional uint64 max_age_secs = 6;
  // files_from list streamed separately if non-empty (large lists
  // shouldn't bloat the spec message).
}
```

The destination manifest is **streamed separately** as part of the
operation, not packed into this message — manifests can be enormous.

## How each operation maps onto this model

### Local copy/mirror

Initiator builds the spec, hands it to the origin (which is itself).
Origin runs `FsTransferSource → DiffPlanner → pipeline → FsTransferSink`.
No protocol involved.

### Push (local→remote)

Initiator builds spec. Initiator is also origin: it enumerates locally,
runs DiffPlanner against the destination manifest the daemon ships up,
streams payloads via `DataPlaneSink` to the daemon. Daemon is target:
runs the receive pipeline writing into `FsTransferSink`.

### Pull (remote→local)

Initiator builds spec, sends it to daemon over the new
`ExecuteOriginJob` RPC (or extension of an existing one). Daemon is
origin: enumerates locally, receives the destination manifest the
initiator streams up, runs DiffPlanner, streams payloads back via
`DataPlaneSink` to the initiator. Initiator is target: runs the
receive pipeline writing into `FsTransferSink`.

This is the case that today goes through `pull_sync.rs` with custom
manifest-diff/compare/resume code. That custom code is what gets
extracted into `DiffPlanner`.

### Remote→remote

Initiator builds spec, hands it to daemon A: "you're the origin,
your target is daemon B." Daemon A enumerates, requests daemon B's
destination manifest directly (no relay through the initiator), runs
DiffPlanner, streams payloads to daemon B via `DataPlaneSink`. Daemon B
is target: runs receive pipeline. Initiator just observes progress.

This is currently CLI-mediated relay (CLI runs `RemoteTransferSource`
to pull from A and re-pushes to B). The role-pure model takes the CLI
out of the data path; bytes go A → B directly. Future work.

## Why this matters

- **Filter parity becomes free.** All filtering goes through `DiffPlanner`,
  on whatever side is the origin. No special pull-side limitation.
- **One diff implementation.** Today's diff logic lives in three places
  (orchestrator local mirror, push client, pull_sync handler). Extracting
  `DiffPlanner` collapses them.
- **Resume becomes universal.** Block-level resume is currently a feature
  of `pull_sync` only. Once it's a `DiffPlanner` capability, push gets it
  for free.
- **Path safety has fewer entry points.** F1's safe-join helper protects
  the receive sink path; with one receive pipeline (instead of pull_sync's
  parallel one), there are fewer sites to instrument.
- **Remote→remote becomes natural.** The role-pure model lets daemon A
  talk to daemon B without the initiator in the loop. Significant
  performance and simplicity win for cross-server workloads.

## Priority sequence

1. **F1: Receive-side path safety** *(prereq, independent)*
   - Shared `safe_join` helper in `blit-core`.
   - Apply at every receive-sink path-join site.
   - Migrate existing sanitizers (`pull.rs::sanitize_relative_path`,
     `service/util.rs` validators) into the shared module.
   - Adversarial-path tests: `..`, absolute Unix, Windows drive,
     UNC, root, valid `..`-containing filenames.
   - Independent of the unification refactor; protects every receive
     path now and after.

2. **TransferOperationSpec: define the data shape**
   - Proto messages: `TransferOperationSpec`, `FilterSpec`,
     `ComparisonMode`, `MirrorMode`, `ResumeSettings`,
     `PeerCapabilities`.
   - Rust types in `blit-core` mirroring proto.
   - No behavior change yet — just the contract.

3. **DiffPlanner: extract from pull_sync.rs into shared core**
   - New module `blit-core::remote::transfer::diff_planner`.
   - Inputs: source header stream, FilterSpec, dest manifest stream,
     ComparisonMode, ResumeSettings, capabilities.
   - Output: `Stream<TransferPayload>`.
   - Push-origin code (orchestrator + push client) starts using it.
   - Behavior preserved; just relocated and unified.

4. **Refactor pull_sync.rs to use the unified pipeline**
   - Daemon side reads `TransferOperationSpec` off the wire.
   - Constructs `FsTransferSource` + `DiffPlanner` + `DataPlaneSink`.
   - Calls `execute_sink_pipeline_streaming`.
   - Delete the parallel manifest-diff / streaming logic that's now
     redundant.
   - Old `PullSync` protocol stays as a deprecated path for rollout
     compatibility (marked behind a capability flag).

5. **Revisit remote→remote** *(architectural decision)*
   - Decide whether daemon A → daemon B should bypass the initiator.
   - If yes: define daemon-to-daemon `ExecuteOriginJob` invocation,
     keep initiator as the spec-shipper and progress observer only.
   - Probably yes long-term, but evaluate after pull is unified.

## What this preserves

- The receive pipeline's symmetry (push receive, pull receive, remote→remote
  receive all share code) is already in place — keep it.
- `FilteredSource` decorator from the recent commit becomes one ingredient
  fed into `DiffPlanner`, not the diff itself.
- All existing path-safety findings (F1) apply here; F1 is sequenced first
  for that reason.

## What this replaces

- The current "filter parity" workaround that bails on pull when filter
  args are passed (CLI side).
- The custom `service/pull_sync.rs` enumeration + diff + stream code.
- CLI-mediated remote→remote relay (eventually).

## What we're not doing yet

- Web/HTTP exposure of metrics — already removed; counters stay internal
  until a future GUI/TUI consumer needs them.
- Daemon-side authentication/TLS — operator's responsibility per existing
  docs; out of scope for pipeline unification.
- Backwards-compat layer for old daemons. Versioning via `spec_version`
  and capability flags should let new clients fall back, but spec-out
  the upgrade path before deletion.

## Open questions for later steps

- **`spec_version` rollout strategy.** When does the new spec become
  required? Does the daemon advertise support via mDNS TXT?
- **Destination manifest streaming.** Current `pull_sync` streams the
  client's manifest piecemeal. Keep that contract for `DiffPlanner`'s
  manifest input, or buffer-and-batch?
- **Capability negotiation.** Currently implicit (try TCP, fall back to
  gRPC). Should `PeerCapabilities` become explicit + negotiated upfront?
- **Resume payload prep on remote source.** `RemoteTransferSource` can't
  emit `FileBlock` payloads today (would require fetching specific byte
  ranges from the daemon). Either teach `RemoteTransferSource` to do
  ranged reads, or accept that resume from a remote source falls back
  to whole-file transfer.

These get resolved during their respective implementation steps.
