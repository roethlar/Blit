# Design-coherence audit — Phase C synthesis (ranked workstreams)

**Status**: Active (Phase C deliverable of `docs/plan/DESIGN_COHERENCE_REVIEW.md`, D-2026-06-11-1)
**Created**: 2026-06-11
**Inputs**: `DESIGN_MAP_2026-06-11.md` (Phase A, 2 errata), `DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (70 adversarially-verified findings: 4 H / 40 M / 26 L), filed findings design-1/2/3, queued slice-2 transport work (STATE.md Queue).
**Disposition rule** (owner decision, D-2026-06-11-1): nothing below enters `REVIEW.md` until the owner ratifies it by slice ID. Severity/evidence per finding lives in the Phase B doc — this report deduplicates and sequences; it does not restate evidence.

## How the 70 findings collapse

The 70 confirmed findings (plus 3 filed, plus the queued transport slice) reduce
to **10 workstreams / 33 slices**. The compression is real, not cosmetic: e.g.
the dead contradictory retry classifier was confirmed under four dimensions
(boundaries, duplication, errors, deadcode) and is **one** deletion slice; the
buffer-formula, tuning-ladder, and pull-discards-tuning findings are three
facets of **one** ownership fix.

One caveat carried forward: Phase B had no progress-reporting dimension, so the
Phase A map's §1.6 risks beyond the verified contract findings (delegated
transfers show zero live progress; daemon byte counters broadcast 0 for 3 of 4
kinds; no denominator anywhere) are **map-sourced, not adversarially verified**
— slice W6.2 below starts with verification.

---

## W1 — Transport & liveness policy (extends queued slice-2)

- **W1.1** *(already queued — STATE.md item 2, not re-ratified here)*: shared
  client channel builder; client HTTP/2 keepalive; explicit
  `max_decoding_message_size`; adaptive flow-control windows; tonic-Status →
  eyre chain preservation; delete inert sink `chunk_bytes`.
- **W1.2** (M, medium): one `configure_data_socket(stream, tcp_buffer_size)`
  helper in blit-core; call from the pull client connect and all three daemon
  accept paths. Today NODELAY/keepalive/tuned-buffers exist on push sockets
  only; the tuner's `tcp_buffer_size` is computed and discarded for every pull.
  [boundaries-pull-direction-bypasses-socket-policy] Coordinates with filed
  design-3 (same call site, missing connect timeout).
- **W1.3** (M, small): TCP keepalive honesty — either configure real
  `TcpKeepalive` timing at both `set_keepalive(true)` sites or rewrite both
  comments to state OS-default (~2 h) behavior; make the daemon copy log its
  failure. [drift-set-keepalive-comments-oversell-liveness]
- **W1.4** (L, small): one shared accept(30s)/token(15s) constant pair replacing
  the four local declarations. [duplication-accept-token-timeout-quadruple,
  constants-accept-token-timeout-quadruplication]

## W2 — Pull-direction parity (the FAST flagship)

Pull is structurally slower than push today: single stream, pool=4, literal
prefetch 8, untuned sockets, while push runs up to 32 tuned streams.

- **W2.1** (M, small): delete the dead warmup machinery
  (`analyze_warmup_result`, `determine_tuning`'s warmup branches) so the tuning
  table is honestly static. [constants-dead-warmup-adaptive-path,
  deadcode-core-warmup-machinery] (The *real* adaptive probe remains H10b-class
  future work — separate plan if the owner wants it.)
- **W2.2** (M, medium): single stream-count owner — move the ladder into
  `determine_remote_tuning` (taking `file_count` so push's signal survives),
  delete `pull_stream_count` and `desired_streams` private ladders, make
  `transfer_plan` take `chunk_bytes` as input instead of embedding its own
  disagreeing ladder. [boundaries-stream-count-policy-minted-three-times,
  constants-three-disagreeing-stream-ladders, duplication-byte-total-tuning-ladders]
- **W2.3** (H, large — **plan doc required first**): multi-stream pull-sync.
  Make the live pull path consume the tuning it already computes (streams,
  prefetch_count, tcp_buffer_size, pooled buffers). The only existing
  multi-stream pull implementation is inside the deprecated Pull RPC — harvest
  its pattern before W2.4 deletes it.
  [constants-pull-sync-discards-tuning + the pull half of
  duplication-buffer-pool-sizing-formula]
- **W2.4** (H, medium — owner decision on wire compat, proto not frozen per
  D-2026-06-11-1): delete the deprecated Pull RPC (client method has zero
  callers; ~500 unreachable daemon lines) **after** W2.3 harvests it; keep
  `scan_remote_files`' metadata path on PullSync or port it.
  [deadcode-pull-rpc-half-migration]

## W3 — Memory-aware buffering

- **W3.1** (H, medium): `BufferPool::for_data_plane(tuning, streams)` —
  one constructor owning the pool formula and the 64 KiB floor, with a budget
  cap derived from available memory (the local-copy `BufferSizer` already
  proves the pattern at `buffer.rs:84`). Replaces the streams×2+4 formula
  pasted at three sites; fixes OOM-by-constant on small-RAM hosts.
  [constants-network-pool-ignores-memory, duplication-buffer-pool-sizing-formula,
  constants-receive-chunk-1mib-asymmetry]

## W4 — Async & cancellation correctness

- **W4.1** (H+M, medium): the AbortOnDrop family — hoist `AbortOnDrop` to
  `blit-core::remote::transfer`; wrap the daemon's three bare data-plane
  handles (filed **design-2**), the push client pipeline + response forwarder,
  and convert the daemon's per-stream worker Vec to a `JoinSet` with
  abort-on-first-error. One pattern, five sites, one regression-test family.
  [design-2, async-push-client-pipeline-detach-on-drop,
  async-daemon-push-stream-workers-detach-on-first-error]
- **W4.2** (M, small): delete the push upload channel (262,144-slot
  drain-and-discard plumbing) — also fixes a real silent wedge on gRPC-fallback
  pushes with >262,144 changed files. [async-push-upload-channel-fallback-wedge,
  deadcode-daemon-push-upload-channel]
- **W4.3** (M, medium): daemon handlers race `tx.closed()` + cancel token
  (hoist delegated_pull's select helper to all three spawn closures); fix the
  false `active_jobs.rs:156` comment. Follow-up slice if wanted: make the
  checksum-collect phase abortable between rayon batches.
  [async-daemon-handlers-blind-to-disconnect-in-compute-phases]
- **W4.4** (M, medium): blocking work off the runtime — batch the per-manifest-
  entry stat/canonicalize into chunked `spawn_blocking` (or cache the
  canonicalized root + lexical containment), and move the single-file checksum
  branch into the same `spawn_blocking` the directory branch already uses 18
  lines below. [async-daemon-push-manifest-blocking-stat-per-entry,
  async-daemon-single-file-checksum-blocks-runtime]

## W5 — Errors & failure text (RELIABLE)

- **W5.1** (M, small, **do first — it's the cheapest high-value slice in this
  report**): install a stderr log backend (warn level) in all four binaries.
  Today every `log::warn!`/`log::error!` in the workspace — including security-
  degradation warnings and "surfaced via log::warn so the failure is visible"
  comments — is formatted and discarded. [errors-log-facade-has-no-backend]
  Pair with one stderr prefix convention [errors-stderr-prefix-babel].
- **W5.2** (M, small): delete `blit-core/src/errors.rs` (dead, contradicts the
  live classifier on three error kinds, name-collides with the proto
  `TransferError`); move `is_retryable`/`is_retryable_io_kind` into blit-core
  next to the conversion sites with a contract test; re-export from blit-app.
  Unblocks testing of the queued W1.1 chain-preservation work in-crate.
  [errors-dead-classifier-contradicts-live, boundaries-retry-policy-split-dead-classifier,
  duplication-retry-classifier-dead-twin, deadcode-core-errors-contradictory-classifier]
- **W5.3** (M, medium): daemon error-boundary helpers — `internal_err(ctx, &Report)`
  using `{:#}` (chain-preserving) + an `io_to_status` that maps NotFound/
  PermissionDenied to proper codes; mechanical sweep of the ~69 plain-format
  and 116 `Status::internal` sites. [errors-daemon-eyre-to-status-chain-amputation,
  errors-daemon-status-internal-collapse]
- **W5.4** (M, medium): one mpsc send-failure vocabulary ("response channel
  closed (peer disconnected or pipeline failed): <context>"); prefer joining
  the exited task and surfacing *its* error where the handle is available.
  [errors-mpsc-sendfail-fixed-strings]
- **W5.5** (L, small): Logger-trait cleanup (permanently-noop error channel).
  [errors-logger-trait-permanently-noop]

## W6 — Progress contract

- **W6.1** (M, medium): define `ProgressEvent` semantics in blit-core
  (bytes ride `Payload` only; `FileComplete` carries bytes:0 and counts files),
  normalize all producers, add producer-side tests, and collapse the TUI's
  three folding rules + the CLI's wrong one into a shared accumulator. This is
  the structural fix that closes filed **design-1**'s class.
  [boundaries-progress-event-contract-lives-in-consumers,
  duplication-progress-folding-rules, design-1]
- **W6.2** (unverified-map-claims, medium): verify-then-fix the Phase A §1.6
  residue: delegated transfers' zero live progress (BytesProgress has consumers
  but no producer), daemon byte counters broadcasting 0 for push/pull/pull_sync,
  no denominator end-to-end. Verification is step 1; each confirmed item
  becomes its own follow-on slice.

## W7 — Shared-helper consolidation

- **W7.1** (M, medium): one mirror/purge deletion executor in blit-core
  (containment + Windows readonly-clearing + dir pruning + consistent stats);
  all four call sites use it. Also moves `enumerate_local_manifest` (the rayon
  version) to blit-core so daemon delegation stops running the sequential twin.
  This duplication already produced one security divergence (R58-F3).
  [duplication-mirror-purge-executors, boundaries-mirror-apply-logic-duplicated-daemon-vs-app]
- **W7.2** (M, small): make `filter_from_spec` pub; push handler uses the
  validated chokepoint instead of its hand-rolled copy (malformed peer globs
  currently silently dropped on the path that scopes mirror deletion).
  [boundaries-push-filter-bypasses-validated-chokepoint]
- **W7.3** (M, small): wire metadata + path helpers into blit-core
  (`permissions_mode`, mtime-seconds with ONE error convention,
  `normalize_for_request`); delete per-crate twins.
  [boundaries-wire-path-metadata-helpers-duplicated, duplication-wire-metadata-helpers]
- **W7.4** (M, small): `checksum::hash_reader(reader, ty)` owning the 256 KiB
  loop; daemon `build_file_header` calls it. [duplication-file-hash-read-loop]
- **W7.5** (M, small): presenter formatting — add `format_bps` to
  `blit_app::display` (binary units), switch `jobs.rs` and the five TUI copies.
  [boundaries-presenter-formatting-fragmented]
- **W7.6** (L, small): `RemoteEndpoint::DEFAULT_PORT` pub; delete the 9031
  literals. [boundaries-private-default-port-literal-duplication]

## W8 — Dead-code removal

- **W8.1** (M, medium): foundation-module sweep — delete `tar_stream.rs`,
  `delete.rs`, `copy/parallel.rs`, `copy/stats.rs`, `chunked_copy_file`, four
  fs_enum helpers + exports (~800 lines, no wire impact). **Owner decision
  embedded**: `zero_copy.rs` (219 lines, Linux splice) — delete, or keep and
  file a FAST plan to actually wire it? [deadcode-core-abandoned-foundation-modules]
- **W8.2** (M, small): delete `transfer_payloads_via_control_plane` (self-
  admitted zero-caller duplicate carrying maintained defensive code); sequence
  with W1.1's chunk_bytes deletion. [deadcode-core-control-plane-payload-duplicate]
- **W8.3** (L, small): hygiene sweep — `--interval-ms` flag (parsed, documented,
  never read; help text actively wrong), blit-cli unused deps
  (walkdir/rayon/sysinfo, tonic placement), blit-app stubs (empty
  remote_remote_direct, dead perf::query), stale `#[allow(dead_code)]`
  annotations masking real dead items. [deadcode-cli-interval-ms-flag,
  deadcode-cli-unused-deps, deadcode-app-stub-module-and-perf-query,
  deadcode-daemon-allow-deadcode-masking]

## W9 — Test infrastructure

- **W9.1** (H, small-medium): un-gate the remote transfer tests — remove
  blanket `#[cfg(unix)]` from suites with nothing unix-specific; gate only the
  genuinely platform-specific assertions. This is most of "Windows parity is
  untestable". [tests-cfg-unix-gating-blocks-windows-transfer-coverage]
- **W9.2** (M, medium): revive the dead workspace-root `tests/` — relocate
  into `crates/blit-core/tests/` (MirrorPlanner's only semantic tests live
  there, never compiled); delete the can't-run `connection.rs`; fix AGENTS.md §4.
  [tests-dead-workspace-root-test-suite]
- **W9.3** (M, medium): harness consolidation — `TestContext::builder()` with
  the knobs the five clones exist for (extra daemon args, second daemon,
  delegation, read_only), shared `cli_bin()`, OnceLock'd daemon build (replaces
  ~75 nested cargo invocations), fake-server config matching production
  keepalive. [tests-five-daemon-harness-clones, tests-per-test-cargo-build-subprocess,
  duplication-cli-test-daemon-harness, tests-fake-server-config-skew]
- **W9.4** (M, small): read-only module enforcement tests (3 gates, zero
  coverage today, mirror-deletion blast radius). [tests-readonly-module-enforcement-untested]
- **W9.5** (M, medium): jobs/detach lifecycle e2e (Subscribe, watch fallback,
  cancel exit codes) — the regression net W4 needs before changing cancellation.
  [tests-jobs-lifecycle-no-e2e]
- **W9.6** (L, small): stderr capture in harness; tuning-tier unit tests.
  [tests-harness-stderr-blackhole, tests-tuning-tiers-never-exercised]

## W10 — Docs & contract repairs (all small; can land as one batch slice)

- **W10.1**: AGENTS.md §4/§6 real symbol names (`transfer_engine`/`PLAN_OPTIONS`
  are ghosts). [drift-agents-md-ghost-identifiers — map erratum #2 already applied]
- **W10.2**: WORKFLOW_PHASE_2.md re-status to Historical with dated erratum
  (Shipped status certifies unbuilt machinery; contradicts STATE.md queue/H10b).
  [drift-workflow-phase2-shipped-ghost-machinery]
- **W10.3**: scope --resume/--retry help + manpage + retry.rs doc to what push
  actually does (whole-file retry, no block resume); optional warning when
  --resume is passed on push. House rule: help + manpage + README in one slice.
  [drift-resume-retry-help-overstates-push-coverage]
- **W10.4**: comment-truth sweep — pipeline.rs FileStream ghost variant,
  pull_sync false "reuse" comment, manpage heartbeat claim, roadmap-audit
  FileStream row. [drift-pipeline-filestream-ghost-variant-comment,
  drift-pull-sync-reuse-comment-over-redefined-constants,
  drift-manpage-verbose-claims-heartbeat-messages]

---

## Proposed queue order (rationale: cheap-and-load-bearing first, then the FAST program, with test nets landing before the behavior they protect)

1. **W5.1** log backend (smallest slice, unmutes ~20 existing failure reports)
2. **W4.2** delete push upload channel (small; removes a real wedge)
3. **W5.2** classifier consolidation (small; unblocks W1.1 testing)
4. **design-2 + W4.1** AbortOnDrop family (filed High + same-pattern confirmed siblings)
5. **W9.5 then W4.3** jobs e2e net, then disconnect-racing (net before behavior)
6. **W1.1 (queued) + W1.2 + design-3** transport policy bundle
7. **W2.1 → W2.2 → W2.3 → W2.4** the pull-parity program (W2.3 needs a plan doc; W2.4 needs the owner's wire call)
8. **W3.1** memory-aware pools (after W2.2 settles the tuning owner)
9. **W6.1 (+ design-1) then W6.2** progress contract, then verify-the-rest
10. **W9.1, W9.2, W9.3, W9.4** test infrastructure (W9.1 early if Windows parity matters near-term)
11. **W7.x** consolidations (each independent, fill gaps between larger slices)
12. **W8.x** dead-code sweeps (anytime; W8.2 sequences with W1.1)
13. **W10.x** docs batch (anytime, cheap)

## Ratification checklist (owner)

Per D-2026-06-11-1 each slice enters REVIEW.md only on explicit ratification.
Compact options: "ratify all as proposed", "ratify all except …", or name
slice IDs. Three embedded owner decisions need explicit answers regardless:

- [ ] **W2.4**: delete the deprecated Pull RPC (wire-breaking; proto not frozen)?
- [ ] **W8.1**: `zero_copy.rs` — delete, or keep + plan to wire splice (FAST)?
- [ ] **W2.3**: ratify writing the multi-stream-pull plan doc (no code until Active)?
