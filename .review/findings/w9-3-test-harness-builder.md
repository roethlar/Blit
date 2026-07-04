# w9-3-test-harness-builder — one daemon-spawn harness, one cli_bin, OnceLock build, fake-server keepalive parity

**Branch**: `master` (owner-authorized branchless loop, D-2026-06-20-6)
**Commit**: _(filled at commit time)_
**Source findings**: tests-five-daemon-harness-clones,
tests-per-test-cargo-build-subprocess, duplication-cli-test-daemon-harness,
tests-fake-server-config-skew — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`;
slice spec W9.3 in `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md`.

## What

The daemon-spawn harness (config structs + port pick + `cargo build` +
spawn + readiness poll) existed in **seven** copies at HEAD, not the five
the 2026-06-11 audit counted — w9-4 (`readonly_enforcement.rs`) and w9-5
(`jobs_lifecycle.rs`) each added another clone *because* the shared
harness couldn't express delegation or a second daemon, proving the
finding's "the next one will miss at least one" prediction twice over.
`cli_bin()` was pasted in 5 more files, `run_with_timeout` in 7,
`ChildGuard` in 4, and all five in-process fake tonic servers (not
three: two in `pull_sync_with_spec_wire.rs`, one in `jobs_lifecycle.rs`,
two in `remote_remote.rs`) ran bare `Server::builder()` while production
sets HTTP/2 keepalive 30s/20s.

Now: `tests/common/mod.rs` is the single owner (builder + spawn
primitives + shared helpers + fake-server scaffold), the daemon build
runs once per test binary behind a `OnceLock`, and every gRPC server —
production and fakes — starts from one shared production-shaped builder
in blit-core.

## Approach

- **`TestContext::builder()`** (`crates/blit-cli/tests/common/mod.rs`)
  with the knobs the clones existed for: `.read_only(bool)` (w9-4),
  `.delegation(bool)` (writes the `[delegation]` table,
  `allowed_source_hosts = ["127.0.0.1"]`, IP-form per the production
  SSRF rule), `.extra_daemon_args(...)` (`--no-server-checksums`,
  `--force-grpc-data`). `TestContext::new()` / `new_read_only()` kept
  their exact signatures — the 13 pre-existing consumer files needed
  zero edits for construction.
- **Dual-daemon support**: `spawn_daemon(workspace, name, module_dir,
  opts) -> SpawnedDaemon` primitive + `TestContext::spawn_second_daemon`.
  `remote_remote::DualDaemonContext`, `jobs_lifecycle::DelegationContext`
  and the readonly delegated-pull test are now thin wrappers over it;
  their private config structs / spawn fns / `wait_for_port` /
  `binary_paths` clones are deleted.
- **Config superset**: shared `ModuleSection` gained
  `delegation_allowed: bool`, serialized explicitly `true` — the daemon
  defaults it to `true` when absent (`runtime.rs::default_true`),
  verified before choosing, so pre-w9-3 configs that omitted it are
  behavior-identical. `DaemonConfig` gained
  `delegation: Option<DelegationSection>` (skipped when `None`).
- **OnceLock build**: `ensure_daemon_built()` wraps the
  `cargo build -p blit-daemon` step (with the `--target` triple handling
  that `remote_remote.rs`'s clone had *dropped* — that drift is now
  structurally impossible). R16-F1's property is preserved per process:
  every spawn path calls it, so no test binary depends on suite
  ordering; ~75 nested cargo invocations per full run become ≤1 per test
  binary. This is also the main daemon-spawn load-flakiness fix: the
  concurrent nested cargo builds all contended for the build-dir flock
  while readiness polls (5s budget) ticked. Bonus: the
  `remote_tcp_fallback` w4-2 helper and the jobs/readonly delegation
  spawns previously ran **no** build at all (relied on ordering) — they
  now get the guarantee.
- **Keepalive parity**: new `blit_core::remote::grpc_server` owns
  `HTTP2_KEEPALIVE_INTERVAL`/`_TIMEOUT` (30s/20s, audit-1 owner decision
  2026-05-23, rationale moved there) and
  `production_server_builder()`. Consumers: daemon `main.rs`,
  `common::spawn_fake_blit_server` (generic over the service impl —
  collapses the thrice-cloned thread+runtime+oneshot-shutdown scaffold),
  and both `pull_sync_with_spec_wire.rs` spy servers. No bare
  `Server::builder()` remains anywhere in the workspace.
- **Port-collision race fixed (the daemon-spawn load-flakiness)**:
  `pick_unused_port` binds `:0` and drops the probe listener before the
  daemon binds, so two parallel tests in one binary could be handed the
  same port — the losing daemon exits on "address in use" and its test
  then silently talks to the *winner's* daemon (wrong/empty module).
  The per-test `cargo build` used to serialize bring-ups and mostly
  hide this; removing it made the race routine (caught red-handed
  during this slice's own validation: `test_admin_find` got an empty
  listing from another test's daemon). Two-layer fix: (a)
  `pick_unused_port` keeps a process-global claimed-port set — cargo
  runs test binaries sequentially, so per-process uniqueness is the
  needed scope; (b) `spawn_daemon`'s readiness poll checks
  `child.try_wait()` each tick and panics with the real reason if the
  daemon exits during startup (external port steal, config rejection)
  instead of timing out generically or proceeding against a foreign
  daemon.
- **stderr policy unified to `Stdio::null()`**: the old shared harness
  piped stderr "for debugging" but nothing ever read it — zero
  diagnostics in practice plus a latent pipe-buffer deadlock once a
  chatty daemon wrote 64 KiB (four clones already used null). Real
  capture (drain thread, dump on readiness failure) stays w9-6's slice
  (tests-harness-stderr-blackhole).
- Serialize-only `#[serde(default)]` attrs (meaningless without
  `Deserialize`) dropped from the config structs; `ChildGuard` gained
  `terminate()` (the pull-mirror/tcp-fallback clones had it, common
  didn't); `run_with_timeout`'s
  `#[cfg_attr(windows, allow(dead_code))]` is superseded by the
  module-level `#![allow(dead_code)]` (each binary uses a different
  subset of the shared harness — documented in the file header).

## Files changed

- `crates/blit-cli/tests/common/mod.rs` — rewritten (229 → ~450 lines):
  builder, spawn primitives, OnceLock build, fake-server scaffold.
- `crates/blit-core/src/remote/grpc_server.rs` — **new**: keepalive
  constants + `production_server_builder()` + value-pin test.
- `crates/blit-core/src/remote/mod.rs` — module declaration.
- `crates/blit-daemon/src/main.rs` — serves from the shared builder.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs` — both spy
  servers production-shaped.
- Ported off private clones: `remote_remote.rs`, `remote_pull_mirror.rs`,
  `remote_checksum_negotiation.rs`, `remote_tcp_fallback.rs`,
  `jobs_lifecycle.rs`, `readonly_enforcement.rs`.
- Ported onto shared `cli_bin`/`run_with_timeout`: `single_file_copy.rs`,
  `local_move_semantics.rs`, `diagnostics_dump.rs`,
  `cli_arg_safety_gates.rs`.
- Net: −1,251 lines of test-tree duplication (+55-line shared module).

## Tests

- No test deleted or weakened: every `#[test]` in the ten touched test
  files survives on the shared harness; assertions untouched except
  harness plumbing. The 270k-file `#[ignore]` w4-2 acceptance test stays
  ignored (still 2 ignored workspace-wide).
- +1 test: `grpc_server::keepalive_values_match_owner_decision` pins
  30s/20s to the audit-1 owner decision (mutation-verified: interval
  30→31 fails it, restored passes). The structural half of the parity
  guarantee is by construction — no bare `Server::builder()` remains
  anywhere in the workspace (grep-verified).
- Workspace totals, measured A/B with one aggregation (sum of every
  `test result:` line, doc-test suites included, via `git stash`):
  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
  across 37 suites. Exactly +1, nothing dropped; per-file `#[test]`
  counts in all ten touched test files are identical to HEAD.
  (STATE's recorded "1479" baseline for design-3 came from a different
  aggregation — off by one from this method against the same tree; the
  same-method A/B above is the count-never-drops evidence.)
- Full suite run twice post-fix (plus `admin_verbs` ×10 alone, the
  binary that flaked): all green — the port-collision flake that
  surfaced mid-slice (see Approach) did not recur.

## Known gaps

- Daemon stderr is now uniformly discarded (was: piped-but-never-read in
  the shared harness, null in 4 of 6 clones). Capture-and-dump on
  readiness failure is w9-6 (tests-harness-stderr-blackhole), unblocked
  by this consolidation — it now needs exactly one edit site.
- `ModuleSection.delegation_allowed` is hardcoded `true` (the daemon
  default); no test exercises `false`. Deliberate: knobs are added when
  a consumer exists (noted in the struct docs).
- The readiness poll stays 50 × 100 ms; if daemon-spawn e2e flakiness
  persists under full-parallel runs after the port registry + OnceLock
  build landed, bumping the budget (or w9-6's stderr capture showing
  *why* startup lagged) is the next lever.
- The claimed-port set only grows (ports are never released back);
  bounded by tests-per-binary (≤ ~70), irrelevant against the ~28k
  ephemeral range. A collision with an unrelated *system* process
  between probe and bind remains possible — that residual case is what
  the child-death panic reports honestly.
- Client-side gRPC channels still set no HTTP/2 keepalive anywhere —
  out of scope here (server-side parity only), noted for the transport
  backlog.
- Windows run of the touched integration suites not executed locally
  (Linux host); windows-latest CI on the next push covers it. No
  platform-specific logic changed — the `cfg!(windows)` binary-name
  branches moved verbatim.
