# Codebase Review - 2026-05-01

## Scope

This review covers the current repository state, including the recent universal filter pipeline, `blit check` command, and daemon metrics endpoint work.

Primary areas reviewed:

- CLI transfer routing and path semantics.
- Core local orchestration, enumeration, filtering, planning, source/sink abstractions, and data-plane receive pipeline.
- Daemon push, pull, pull-sync, admin RPCs, config loading, and metrics.
- Remote parity documentation, workflow status documents, and saved project context.

Verification performed:

- `cargo check --workspace` passes.
- `cargo test --workspace` passes when run outside the sandbox with local port binding allowed: 185 tests passed, 1 doc test ignored.
- The sandboxed test run failed first because admin integration tests could not bind probe listeners: `Operation not permitted`.
- Warnings remain for deprecated macOS FSEvents API usage in `crates/blit-core/src/change_journal/snapshot.rs:85` and `crates/blit-core/src/change_journal/snapshot.rs:111`, plus an unused test variable in `crates/blit-core/src/fs_capability/macos.rs:165`.

## Executive Summary

The codebase has a solid core shape: the newer `TransferSource` / `TransferSink` / shared pipeline design is the right abstraction, local and remote transfer paths are converging, and the current test suite has broad integration coverage for admin verbs, remote push/pull, fallback, resume, remote-to-remote, and path semantics.

The main issues are boundary issues rather than architecture issues. Receive-side path handling is not centralized and still has unsafe joins. The daemon exposes `use_chroot` as a config/documented feature but appears not to enforce it. Filtered mirror delete semantics need an explicit product decision. The in-progress metrics and `check` features compile and test, but need lifecycle, endpoint, and semantics hardening before being treated as production-grade.

## Findings

### F1. Receive-side path sanitization is incomplete and inconsistent

Severity: High

The receive path accepts relative paths from the wire and writes them by joining directly under the destination root in several places:

- `crates/blit-core/src/remote/transfer/sink.rs:218` to `crates/blit-core/src/remote/transfer/sink.rs:222` builds `dst_root.join(&header.relative_path)` for streamed files.
- `crates/blit-core/src/remote/transfer/sink.rs:426` builds `dst_root.join(&p.rel)` for tar entries.
- `crates/blit-core/src/remote/transfer/sink.rs:462` builds `dst_root.join(relative_path)` for resume block writes.
- `crates/blit-core/src/remote/transfer/sink.rs:498` builds `dst_root.join(relative_path)` for block completion.
- `crates/blit-core/src/remote/transfer/pipeline.rs:348` to `crates/blit-core/src/remote/transfer/pipeline.rs:356` constructs `FileHeader` directly from a wire string without semantic path validation.

The tar-shard path check is too weak:

- `crates/blit-core/src/remote/transfer/sink.rs:403` to `crates/blit-core/src/remote/transfer/sink.rs:407` rejects any string containing `..`, but does not reject absolute paths or Windows prefixes via path components.

Impact:

A malicious or buggy peer can potentially cause writes outside the intended destination root via absolute paths, Windows prefixes, or other component-level edge cases. The current `rel.contains("..")` check also rejects legitimate filenames containing `..`, while missing other escape forms. This is a trust-boundary issue because the same sink code is used by remote push, pull, remote-to-remote relay, and receive-side resume.

The codebase already has stronger logic in nearby modules:

- `crates/blit-core/src/remote/pull.rs:944` to `crates/blit-core/src/remote/pull.rs:965` has a component-aware `sanitize_relative_path`.
- `crates/blit-daemon/src/service/util.rs:38` to `crates/blit-daemon/src/service/util.rs:86` has daemon-side relative path validation.

Recommendation:

Move a single component-aware sanitizer into shared core code and use it before every receive-side filesystem join. It should reject absolute paths, `..`, Windows prefixes, root components, and platform-specific absolute-style strings. It should preserve the existing empty-path behavior only for single-file transfers where the destination root is already the final file path. Add tests for streamed files, tar shards, block writes, block completion, Unix absolute paths, Windows drive prefixes, UNC-style paths, and valid filenames containing literal `..`.

### F2. `use_chroot` is documented and configured but not enforced

Severity: High

The daemon exposes chroot-style configuration:

- `docs/DAEMON_CONFIG.md:82` documents `root_use_chroot`.
- `docs/DAEMON_CONFIG.md:96` documents per-module `use_chroot`.
- `docs/plan/WORKFLOW_PHASE_3.md:15`, `docs/plan/WORKFLOW_PHASE_3.md:44`, and `docs/plan/WORKFLOW_PHASE_3.md:57` claim read-only/chroot enforcement.

The runtime loads the field, but stores it in an underscore-prefixed field:

- `crates/blit-daemon/src/runtime.rs:193` to `crates/blit-daemon/src/runtime.rs:199` stores module `_use_chroot`.
- `crates/blit-daemon/src/service/util.rs:16` to `crates/blit-daemon/src/service/util.rs:22` carries root `use_chroot` into `_use_chroot`.

Search results show `use_chroot` is displayed or stored, but there is no enforcement path. Most daemon handlers validate textual `..` paths and then use `module.path.join(...)`, for example `crates/blit-daemon/src/service/core.rs:191` to `crates/blit-daemon/src/service/core.rs:194`.

Impact:

Users can configure or read docs suggesting a chroot boundary exists when the implementation does not appear to enforce an OS chroot or a canonicalized containment check. The existing `resolve_relative_path` prevents obvious `..` traversal, but it does not by itself prevent symlink-based escape from a module root. That matters for list/read/write/delete paths that call `metadata`, open files, or create directories under joined paths.

Recommendation:

Either implement the promised boundary or remove/rename the option until it exists. If the intended model is "chroot-like containment" rather than OS `chroot(2)`, canonicalize the target and enforce `target.starts_with(module_root)` for operations that follow symlinks, with care for paths that do not exist yet during writes. For write paths, validate each existing parent component before creating the final file. Add symlink escape tests for list, pull, push, purge, find, du, and completions with `use_chroot = true`.

### F3. Daemon default exposure is broad for an unauthenticated protocol

Severity: Medium

The daemon defaults to binding on all interfaces:

- `crates/blit-daemon/src/runtime.rs:150` to `crates/blit-daemon/src/runtime.rs:155` defaults `bind_host` to `0.0.0.0`.
- `docs/DAEMON_CONFIG.md:75` documents the same default.

The project documentation explicitly says remote transfers rely on operator-provided secure networks or tunnels, which is a valid product stance. The risk is that the default binding still exposes the unauthenticated daemon to the whole LAN when an operator starts it without reading the security docs.

Impact:

With no built-in TLS or authentication, a broad default bind increases the blast radius of misconfiguration. mDNS advertising can make the daemon easier to discover on the local network.

Recommendation:

Consider defaulting to `127.0.0.1` and requiring `--bind 0.0.0.0` or config opt-in for LAN exposure. If LAN-by-default is intentional, emit a startup warning when binding a non-loopback address without an explicit acknowledgement. Keep the current docs, but make the runtime behavior harder to expose accidentally.

### F4. Filtered mirror delete semantics are ambiguous and easy to misuse

Severity: Medium

Local mirror delete logic applies the same filter to the destination enumeration:

- `crates/blit-core/src/orchestrator/orchestrator.rs:508` to `crates/blit-core/src/orchestrator/orchestrator.rs:519` creates `FileEnumerator::new(filter.clone_without_cache())` for destination entries.
- `crates/blit-core/src/orchestrator/orchestrator.rs:524` to `crates/blit-core/src/orchestrator/orchestrator.rs:532` deletes destination entries that are not in the filtered source path set.

Impact:

With a command such as `blit mirror src dst --include '*.log'`, the operation likely only considers `.log` entries for both source and destination. That means stray non-log destination files remain. This may be the desired "filtered mirror" behavior, but it conflicts with an intuitive "destination exactly mirrors source" reading.

The CLI text in `crates/blit-cli/src/cli.rs:169` to `crates/blit-cli/src/cli.rs:172` says filters apply identically across source/destination combinations, but does not define delete scope.

Recommendation:

Make the semantics explicit. Either document "filtered mirror only mirrors and deletes within the filtered subset" or add a delete-scope option such as `--delete-scope filtered|all`. Add tests that cover include-only, exclude-only, `--files-from`, and size/age filters for mirror deletions.

### F5. Metrics `active_transfers` can leak on task panic or cancellation

Severity: Medium

The new metrics code increments `active_transfers` before spawning transfer tasks and decrements it after the awaited handler returns:

- Push: `crates/blit-daemon/src/service/core.rs:82` to `crates/blit-daemon/src/service/core.rs:96`.
- Pull: `crates/blit-daemon/src/service/core.rs:118` to `crates/blit-daemon/src/service/core.rs:130`.
- Pull sync: `crates/blit-daemon/src/service/core.rs:152` to `crates/blit-daemon/src/service/core.rs:172`.

Impact:

If a spawned task panics or is aborted before reaching the decrement, the gauge stays elevated. The error counter also only increments when the handler returns an error status, not when the task panics. This can make Prometheus output misleading during exactly the failures operators need to diagnose.

`purge_operations` is also semantically inconsistent with the metrics module comment. The metrics module says counters increment at the dispatch boundary (`crates/blit-daemon/src/metrics.rs:3` to `crates/blit-daemon/src/metrics.rs:8`), but purge increments only after successful deletion at `crates/blit-daemon/src/service/core.rs:273` to `crates/blit-daemon/src/service/core.rs:277`.

Recommendation:

Use an RAII guard that increments on construction and decrements on `Drop`, then move the guard into the spawned task. If panic accounting is required, wrap the handler or inspect join handles where practical. Decide whether operation counters mean "attempts" or "successes"; if attempts, increment purge before validation/deletion and add separate success/error counters. If successes, rename metrics accordingly.

### F6. Metrics HTTP server needs basic hardening before production use

Severity: Medium

The metrics endpoint is intentionally hand-rolled:

- `crates/blit-daemon/src/metrics.rs:84` to `crates/blit-daemon/src/metrics.rs:111` accepts connections and spawns one task per connection.
- `crates/blit-daemon/src/metrics.rs:113` to `crates/blit-daemon/src/metrics.rs:148` reads one buffer and dispatches by path prefix.

Impact:

For an opt-in internal endpoint this is acceptable as a starting point, but it has no read timeout, no connection concurrency limit, no method validation, and path matching accepts any path starting with `/metrics` or `/health`. If an operator binds this to a non-loopback interface, it is another unauthenticated endpoint to harden and document.

Recommendation:

Add a read timeout, validate `GET`, enforce exact path or documented prefix behavior, and document that `--metrics-addr` should normally be loopback or behind a trusted scrape network. Add an async test that starts the endpoint, performs `/metrics`, `/health`, unsupported method, and not-found requests.

### F7. Remote-source tar shard preparation can double-buffer and over-allocate

Severity: Medium

`RemoteTransferSource::prepare_payload` builds tar shards by reading each remote file into a per-file `Vec`, then appending it into a tar builder backed by another `Vec`:

- `crates/blit-core/src/remote/transfer/source.rs:160` to `crates/blit-core/src/remote/transfer/source.rs:180`.
- The TODO at `crates/blit-core/src/remote/transfer/source.rs:164` acknowledges the double-buffering.

Impact:

Planner policy should keep tar shards small, so this is not automatically catastrophic in the happy path. The risk is that remote-source metadata is a trust boundary: a malicious or buggy source daemon can advertise unexpectedly large files or enough files to create memory pressure. This code also makes remote-to-remote small-file workloads pay extra memory copy overhead.

Recommendation:

Enforce a strict tar-shard byte budget in `RemoteTransferSource` before allocating, not only in the planner. Prefer streaming tar construction when practical, or fall back to individual file payloads when advertised headers exceed the safe tar-shard budget. Add tests with oversized header metadata from a fake source.

### F8. Wire receive limits still allow large per-stream allocations

Severity: Medium

The receive parser has useful upper bounds:

- `crates/blit-core/src/remote/transfer/pipeline.rs:315` to `crates/blit-core/src/remote/transfer/pipeline.rs:329` defines path, tar count, tar bytes, and block byte caps.

However, the tar-shard cap is still 1 GiB, and the parser allocates the whole tar shard at once:

- `crates/blit-core/src/remote/transfer/pipeline.rs:382` to `crates/blit-core/src/remote/transfer/pipeline.rs:390`.

Impact:

A single accepted tar shard can allocate 1 GiB before extraction. With multiple streams, that can exceed the host memory budget. The limits prevent unbounded `u64::MAX` allocations, but they are still high enough to create operational instability.

Recommendation:

Tie caps to negotiated tuning and a process memory budget. Prefer streaming tar extraction or smaller bounded chunks instead of whole-shard allocation. Add tests that assert accepted maximums match documented memory policy.

### F9. Local mirror owns a nested Tokio runtime

Severity: Medium

`execute_local_mirror` creates a multi-thread Tokio runtime inside a synchronous API:

- `crates/blit-core/src/orchestrator/orchestrator.rs:333` to `crates/blit-core/src/orchestrator/orchestrator.rs:349`.

Impact:

This works for the current CLI call path, but it is brittle as a library API. If future callers invoke it from an async context, nested runtime behavior can panic or waste threads. It also makes orchestration harder to compose with daemon or async integration tests.

Recommendation:

Split the implementation into `execute_local_mirror_async` and a small synchronous wrapper for legacy callers. Let CLI async code call the async version directly.

### F10. Remote pull filter support is missing and help text overstates parity

Severity: Medium

Remote pull rejects filter flags:

- `crates/blit-cli/src/transfers/remote.rs:226` to `crates/blit-cli/src/transfers/remote.rs:240` explains that pull-side filtering needs a proto extension.

The CLI help comment says filters apply identically to all source/destination combinations:

- `crates/blit-cli/src/cli.rs:169` to `crates/blit-cli/src/cli.rs:172`.

Impact:

The explicit rejection is the correct behavior until the protocol supports daemon-side filtering. The issue is that the user-facing and code comments now overstate parity. This can confuse users and future agents who assume filters are fully universal.

Recommendation:

Update CLI help/manpage language to say filters currently apply to local sources, push, and local/remote relay paths where the source can be filtered locally; remote pull requires a protocol extension. Add a protocol field for filter expressions if remote pull filtering is in scope.

### F11. Pull checksum capability acknowledgement is discarded

Severity: Medium

The pull-sync client receives server checksum capability but drops it:

- `crates/blit-core/src/remote/pull.rs:466` to `crates/blit-core/src/remote/pull.rs:469`.

Impact:

If server-side checksums are disabled, the client currently has no stored capability bit to drive UX, comparison strategy, or warnings. This may already be compensated elsewhere, but the TODO is in the hot path and should not remain ambiguous.

Recommendation:

Store the ack capability in pull state and use it to decide comparison behavior, progress messaging, and warnings. Add a test where daemon checksums are disabled and verify the client behavior is intentional.

### F12. `blit check` compares files only and should define directory/symlink semantics

Severity: Low to Medium

The new `blit check` command is intentionally local and read-only, but it skips non-file entries:

- `crates/blit-cli/src/check.rs:126` to `crates/blit-cli/src/check.rs:129` skips non-file source entries.
- `crates/blit-cli/src/check.rs:200` to `crates/blit-cli/src/check.rs:204` skips non-file destination entries.

Impact:

Two trees that differ only by empty directories or symlinks can be reported as identical. That may match current transfer semantics for symlinks, but it does not match a general "tree comparison" mental model. Empty directories appear to matter elsewhere in remote edge tests, so this should be explicit.

Recommendation:

Decide whether `check` verifies transfer equivalence or full filesystem tree equivalence. If transfer equivalence, document that symlinks and possibly empty directory differences are ignored. If tree equivalence, include directory and symlink entries in the diff model.

### F13. Documentation state has drifted from implementation state

Severity: Low to Medium

Several docs disagree about project status:

- `docs/plan/PROJECT_STATE_ASSESSMENT.md:10` to `docs/plan/PROJECT_STATE_ASSESSMENT.md:24` says Phase 4 is done.
- `docs/plan/WORKFLOW_PHASE_4.md:5` now links this review and marks Phase 4 in progress.
- `TODO.md:90` still marks "P0 Remote transfer parity refactor" unchecked, even though every listed subtask is checked.
- `TODO.md:102` to the remaining 25GbE items still mix implementation-complete subtasks with benchmark-only follow-up.
- `docs/plan/WORKFLOW_PHASE_3.md:114` still has an unchecked exit checklist item for daemon config/read-only/chroot enforcement.

Impact:

Agents use these docs as control-plane state. Drift causes duplicate work, missed hardening, and false confidence. The chroot item is particularly important because docs imply a security boundary that code does not appear to enforce.

Recommendation:

Make `docs/plan/PROJECT_STATE_ASSESSMENT.md` the explicit source of truth or update the stale workflow docs. Separate "implemented", "tested in unit/integration", "benchmarked on hardware", and "security boundary verified" statuses.

### F14. Remaining warnings should be paid down before release

Severity: Low

Current warnings from verification:

- Deprecated FSEvents API: `crates/blit-core/src/change_journal/snapshot.rs:85` and `crates/blit-core/src/change_journal/snapshot.rs:111`.
- Unused test variable: `crates/blit-core/src/fs_capability/macos.rs:165`.

Impact:

These do not block functionality, but release builds should not train developers to ignore warnings. The FSEvents warning also points at a future dependency/API migration.

Recommendation:

Prefix the unused test variable or assert the result. Track migration from `fsevent_sys::FSEventsGetCurrentEventId` to the recommended `objc2-core-services` path.

### F15. Structured logging remains deferred

Severity: Low

The daemon and transfer paths still rely heavily on `println!` / `eprintln!` style output. `docs/plan/PROJECT_STATE_ASSESSMENT.md:88` to `docs/plan/PROJECT_STATE_ASSESSMENT.md:90` already lists structured logging as deferred.

Impact:

Unstructured output is workable during active development, but it makes high-throughput transfer diagnostics, daemon operations, and log scraping harder. It can also add stderr noise in hot paths if not gated by verbosity.

Recommendation:

Adopt `tracing` or `log` across daemon and transfer modules with clear levels. Gate noisy data-plane logs behind debug/trace. Keep CLI human output separate from daemon logs and machine-readable progress.

## Architecture Observations

The strongest design decision is the shared transfer pipeline:

- `TransferSource` and `TransferSink` give the codebase a clear seam between enumeration, planning, transport, and filesystem writes.
- `execute_sink_pipeline` and `execute_sink_pipeline_streaming` in `crates/blit-core/src/remote/transfer/pipeline.rs` make local, push, pull, and remote-to-remote behavior easier to converge.
- `FilteredSource` in `crates/blit-core/src/remote/transfer/source.rs:226` to `crates/blit-core/src/remote/transfer/source.rs:283` is the right chokepoint for source-side filter parity.
- The receive pipeline symmetry is good: `crates/blit-core/src/remote/transfer/pipeline.rs:173` onward keeps push/pull receive behavior from diverging.

The main architecture gap is that path safety is not yet treated as a first-class shared abstraction. There are multiple sanitizers and multiple direct joins. This should be pulled up into a single tested module before adding more protocol features.

## Priority Remediation Plan

1. Fix receive-side path safety first. Add shared sanitizer, apply to streamed file, tar shard, block, and block-complete writes, then add malicious-path tests.
2. Resolve `use_chroot` truthfulness. Either implement symlink/canonical containment for all daemon operations or remove the option/docs until implemented.
3. Decide filtered mirror delete semantics and encode them in CLI help, docs, and tests.
4. Harden metrics lifecycle with an active-transfer guard, clear counter semantics, endpoint tests, read timeout, and method/path validation.
5. Bound remote tar shard memory by policy, not only planner assumptions. Move toward streaming tar construction/extraction.
6. Split local mirror into async implementation plus sync wrapper to remove nested runtime coupling.
7. Update docs status and close or re-scope stale workflow checklist items.
8. Add tests for `blit check` directory/symlink behavior and remote pull filter rejection/help text.

## Suggested New Tests

- Receive path safety tests for file stream, tar shard, block write, and block completion with `../x`, `/tmp/x`, `C:\x`, UNC-style paths, and valid names containing `..`.
- Daemon chroot tests using symlinks inside an exported module that point outside the root.
- Filtered mirror tests for include, exclude, `--files-from`, size, and age filters, including destination-only files inside and outside the filter set.
- Metrics tests for active gauge decrement on handler error and panic-like paths, plus HTTP endpoint behavior.
- `blit check` tests for empty directories, symlinks, file-vs-directory mismatches, checksum mode errors, and JSON shape.
- Remote pull filter tests that assert the current rejection message, then protocol-extension tests if filtering is implemented.
