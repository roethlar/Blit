# otp12-pf1-p2-observer — aggregate small-file attribution probe

**Slice**: OTP12 performance-finding pf-1 instrumentation, P2 only.
This slice is descriptive instrumentation. It does not grade H6/H7, run a
historical control, implement a counterfactual, or claim a performance result.

## What

P2 says the unified small-file TCP path is about 10–20% slower than the pinned
old push on both performance rigs, while the gRPC control did not regress in
the same way. The current tree had no high-volume observer that could separate:

- H7's common manifest-map work, synchronization wait, per-need channel send,
  queued hop, and handler work;
- H6's per-member TCP need claims from the in-stream shard-batched claim;
- tar-record receive time from destination sink time; or
- tar parse/validation, blocking-pool delay, parallel member wall time, and
  per-member mkdir/open/write/descriptor-drop/metadata work.

Without those boundaries, a later old/new comparison could only restate wall
time and could not say where the small-file cost accumulated.

## Approach

- Add a separate `SmallFileProbe`, disabled by default. Production activation
  requires both `BLIT_TRACE_SMALL_FILE_PROBE=1` and a non-empty
  `BLIT_TRACE_RUN_ID`. Tests can inject an in-memory emitter.
- Bind one observer to each semantic SOURCE/DESTINATION endpoint after session
  establishment. TCP reports correlate with a safe digest of the existing
  session token. In-stream reports use the shared run ID, which is therefore a
  one-observed-session correlation scope (see Known gaps). No frame, token,
  transfer ordering, carrier choice, connection topology, or worker policy
  changes.
- Emit one schema-1 aggregate report per endpoint, only after that endpoint's
  successful summary path. Reports contain semantic endpoint/initiator roles,
  carrier, zero-valued fields, monotonic/Unix timing, and bounded shard records.
  They never contain paths, tokens, or per-file log lines.
- Give H7 backportable names. Common operations are
  `manifest_insert_map_op`, `need_resolve_map_op`, and `need_handler_work`;
  current-only costs remain separate as synchronization wait, channel-send,
  and enqueue-to-handler-hop spans. A task-local/inline historical path can
  record its common work without pretending it held a lock or processed an
  event.
- Keep the normal event type unchanged. Probe-on enqueue timestamps live in a
  probe-owned path-keyed table; probe-off changes neither `SourceEvent` layout
  nor event construction and takes no clocks or table locks.
- Record H6 claim members, successful removals, lock acquisitions, wait, and
  critical-section time separately for TCP and in-stream. TCP drops the real
  outstanding-needs mutex before updating observer atomics.
- Join concurrent receive and sink records with a keyed BLAKE3 digest of the
  ordered, length-delimited shard membership. The digest is opaque and paths
  are not emitted. Correlation hashing is isolated from the reported carrier
  receive and sink spans.
- Bound receive and sink records at 16,384 each and report dropped counts.
  Member work is aggregated per shard, so report size scales with shards, not
  files.
- Preserve the original probe-off filesystem branch. Probe-on expands
  `std::fs::write` into its equivalent create/write-all/descriptor-drop steps
  solely to time them honestly, then applies the same metadata policy. `close`
  means descriptor drop, not fsync or a durability guarantee.
- Explicitly disable this remote P2 observer for local-apply sessions; local
  payload bytes do not ride either observed remote carrier.

## Files changed

- `crates/blit-core/src/remote/transfer/small_file_probe.rs`
- `crates/blit-core/src/remote/transfer/mod.rs`
- `crates/blit-core/src/remote/transfer/pipeline.rs`
- `crates/blit-core/src/remote/transfer/sink.rs`
- `crates/blit-core/src/remote/transfer/session_client.rs`
- `crates/blit-core/src/transfer_session/data_plane.rs`
- `crates/blit-core/src/transfer_session/local.rs`
- `crates/blit-core/src/transfer_session/mod.rs`
- `crates/blit-core/tests/transfer_session_roles.rs`
- `docs/STATE.md`
- `REVIEW.md`

## Guard

`small_file_probe_is_complete_and_inert_across_roles_and_carriers` runs the
same 256 × 4 KiB tar fixture through eight successful real sessions:
SOURCE-initiator and DESTINATION-initiator, TCP and in-stream, probe OFF and
ON. It asserts:

- OFF emits nothing;
- OFF/ON and cross-initiator summary, need inventory, carrier, stream count,
  final bytes, mtimes, and Unix permissions are identical;
- exactly one report per semantic endpoint, with role-exclusive zero state;
- SOURCE has exactly 256 manifest inserts, need resolutions, sends, queued
  hops, and handler samples, with non-vacuous timing aggregates;
- the planner accounts for all 256 members without treating planned shard
  count as the actual emitted count;
- TCP claims 256 members under 256 lock acquisitions, while in-stream claims
  all members under one acquisition per actually received shard;
- every actual receive record joins exactly one sink record on
  `(shard_id, members, archive_bytes)` despite concurrent completion;
- receive and sink spans are nonzero and internally consistent, all six member
  timing categories contain exactly 256 nonzero samples, and no bounded record
  was dropped; and
- TCP queue timings/counts are present while the in-stream TCP-queue fields are
  exactly zero.

Mutation proofs, each run against a transfer that otherwise still completed:

1. Removing the TCP `NeedListSink` probe attachment failed at 0 versus 256
   claim members.
2. Removing the filesystem-sink attachment failed at zero shard-sink records.
3. Removing the common need-handler hook failed at 0 versus 256 samples.
4. Removing the H7 enqueue stamp failed at 0 versus 256 hop samples.
5. Replacing sink shard IDs with a constant failed the exact receive/sink join.
6. Skipping probe-on mtime application failed OFF/ON metadata parity.
7. Replacing keyed, ordered shard IDs with one constant failed the direct
   cross-run/order-isolation unit guard.
8. Treating an absent production trace flag as enabled failed the environment
   activation unit guard before constructing an emitter.

Every mutation was restored. The focused guard returned green after each
restoration. The probe module also pins environment activation and single-shot
success emission with zero fields preserved. The complete role target passes
42/42.

Full workspace gate: fmt check, strict workspace clippy, and 1,493 passed / 2
ignored. `scripts/agent/check-docs.sh` passes and `docs/STATE.md` remains at
198/200 lines.

Production-path smoke: running the real one-session loopback-TCP role test with
`BLIT_TRACE_SMALL_FILE_PROBE=1` and a unique `BLIT_TRACE_RUN_ID` passed and
emitted exactly two prefixed, parseable schema-1 JSON summaries. They carried
the same token-derived correlation ID, opposite semantic endpoint roles, exact
source/destination-exclusive inventories, and a matching keyed receive/sink
shard ID. This exercises the actual environment activation and stderr writer,
not the injected test emitter.

## Known gaps

- These timings are descriptive. Under the active plan, H6/H7 can only be
  confirmed or killed by their separately reviewed wall-time counterfactuals
  on the pre-registered scale. This slice supplies no causal grade.
- No local, rig, or historical measurement is part of this slice. In
  particular, it does not change the current P1/P2 status or make Mac↔Mac an
  acceptance run.
- In-stream has no existing wire-shared session token. Its correlation ID is
  derived from `BLIT_TRACE_RUN_ID`, so that ID must be identical at both ends
  and unique to exactly one observed in-stream session. The existing
  block-level rig harness reuses one run ID across many sessions and cannot
  consume this probe unchanged; a missing/mismatched exact two-report
  inventory voids the run.
- The pinned `0f922de` control needs an adapted probe, not a cherry-pick. It
  must initialize source stats from the unique shared run ID before old push's
  early task-local manifest work, define its own role type, and hook both the
  old TCP direct sink and old gRPC `TarShardExecutor` path. The neutral common
  map/handler fields above are the comparison contract; synchronization and
  channel fields are honestly zero where the old path has no such mechanism.
- `record_receive_ns` is intentionally not a CPU-decode claim. It includes
  carrier wait/allocation/framing after that carrier's discriminator: TCP
  includes its tar header and archive body; in-stream begins after
  `TarShardHeader` validation/claim and covers the chunk frames. Claim time is
  reported separately.
- Planned tar-shard count precedes in-stream header-size bounding, so long
  paths can split a planned shard. `planned_tar_shards` is descriptive;
  receive/sink records are the authoritative actual shard inventory.
- Output is best-effort stderr. Serialization/write failure, a probe-task join
  failure, nonzero dropped counts, or any missing endpoint summary invalidates
  an observed run; the future analyzer must fail closed on that inventory.
- Probe-on necessarily adds clocks, atomics, bounded record locks, correlation
  hashing, and explicit member timing. Every measured configuration therefore
  needs its plan-required OFF/ON pair to bound observer overhead.

## Reviewer comments

Claude Fable 5 via Claude Code 2.1.211, `--effort max`, accepted exact range
`f01d662351039a4153ee9a64ee6c59c10d29b9b7..713526e8f4cbe61f881a24d1f19cc481d0a8b188`
with no material finding and `guard_confirmed=true`. It independently passed
the three probe units and the complete role/carrier guard, replaced production
shard IDs with one constant, observed the predicted key-isolation failure,
restored the exact reviewed blob, and reran both commands green. The retained
review worktree is clean at the reviewed SHA. Raw output and adjudication:
`.review/results/otp12-pf1-p2-observer-r1.claude.json` and
`.review/results/otp12-pf1-p2-observer-r1.claude-verdict.md`. The interrupted
non-authoritative first attempt is retained separately as
`.review/results/otp12-pf1-p2-observer-r1.claude-attempt1-error.json`.
