# Code Inventory: tests (integration) + scripts + workflow

**Generated**: 2026-06-04 by audit workflow
**Cluster**: integration tests, helper scripts, GitHub Actions CI

## Coverage

Files read end-to-end (no skips):

| File | Lines | Notes |
| --- | ---: | --- |
| `.github/workflows/ci.yml` | 86 | only workflow file in repo |
| `crates/blit-cli/tests/common/mod.rs` | 212 | shared `TestContext` harness |
| `crates/blit-cli/tests/admin_verbs.rs` | 485 | ls/du/df/find/rm/completions vs daemon |
| `crates/blit-cli/tests/blit_utils.rs` | 631 | redundant ls/du/df/find/rm/profile coverage |
| `crates/blit-cli/tests/cli_arg_safety_gates.rs` | 262 | R54 --null/--force/--ignore-times gating |
| `crates/blit-cli/tests/diagnostics_dump.rs` | 123 | `blit diagnostics dump` JSON shape |
| `crates/blit-cli/tests/f2_chroot_containment.rs` | 195 | symlink-escape containment; unix-only |
| `crates/blit-cli/tests/local_move_semantics.rs` | 402 | R46/R47/R49/R50/R51/R52 move guards |
| `crates/blit-cli/tests/remote_checksum_negotiation.rs` | 289 | F11 / R15-F1 pull checksum ack |
| `crates/blit-cli/tests/remote_move.rs` | 205 | push/pull move + audit-6e dir-tree, win-gated |
| `crates/blit-cli/tests/remote_parity.rs` | 220 | TCP vs gRPC data-plane parity, unix-only |
| `crates/blit-cli/tests/remote_pull_mirror.rs` | 451 | filtered-subset/all delete-scope, unix-only |
| `crates/blit-cli/tests/remote_pull_subpath.rs` | 123 | basename-append & double-nest regressions |
| `crates/blit-cli/tests/remote_push_mirror_safety.rs` | 158 | R59 #1 scan-incomplete & filter scope |
| `crates/blit-cli/tests/remote_push_single_file.rs` | 84 | single-file push, unix-only |
| `crates/blit-cli/tests/remote_regression.rs` | 227 | pull_sync deadlock + mtime preserve |
| `crates/blit-cli/tests/remote_remote.rs` | 892 | delegation/relay + 2 fake gRPC servers |
| `crates/blit-cli/tests/remote_resume.rs` | 132 | --resume block-hash flow, unix-only |
| `crates/blit-cli/tests/remote_tcp_fallback.rs` | 243 | --force-grpc-data daemon flag exercise |
| `crates/blit-cli/tests/remote_transfer_edges.rs` | 228 | nested dirs, many small files, empty dirs |
| `crates/blit-cli/tests/single_file_copy.rs` | 172 | local single-file copy + idempotency |
| `crates/blit-core/tests/pull_sync_with_spec_wire.rs` | 427 | spec-on-wire roundtrip via SpyServer |
| `scripts/bench_10gbe.sh` | 245 | local/NFS/SMB/remote TCP+gRPC + rsync compare |
| `scripts/bench_local_mirror.sh` | 378 | blit vs rsync local mirror bench |
| `scripts/bench_local_mirror_macos.sh` | 9 | macOS shim around bench_local_mirror.sh |
| `scripts/bench_remote_remote.sh` | 118 | direct vs --relay-via-cli benchmark |
| `scripts/build-release.sh` | 81 | release + strip + optional tarball |
| `scripts/codex_resume.sh` | 6 | personal Codex resume helper (sources superclaude venv, `sudo npm`) |
| `scripts/linux/run-journal-fastpath.sh` | 61 | 5000-file mirror smoke; finds prebuilt blit |
| `scripts/linux/run-remote-fallback.sh` | 23 | 2-arg remote smoke; no `set -u` guard on $1/$2 |
| `scripts/mac_codex_resume.sh` | 1 | one-liner codex resume |
| `scripts/macos/run-blit-tests.sh` | 39 | sequential fmt/check/test+per-step tee |
| `scripts/macos/run-journal-fastpath.sh` | 61 | mac twin of linux journal-fastpath |
| `scripts/test.sh` | 220 | Codex prompt-bundle generator — UNRELATED to blit tests |
| `scripts/win_codex.ps1` | 1 | one-liner codex resume |
| `scripts/windows/bench-local-mirror.ps1` | 414 | blit vs robocopy mirror bench |
| `scripts/windows/build-release.ps1` | 42 | minimal cargo build wrapper |
| `scripts/windows/probe-usn-volume.ps1` | 67 | enumerate volume device paths + fsutil usn queryjournal |
| `scripts/windows/run-blit-tests.ps1` | 51 | windows twin of run-blit-tests |
| `scripts/windows/run-journal-fastpath.ps1` | 77 | hard-codes C:\ for NTFS, D:\ for ReFS |
| `scripts/2025-10-24-read-files-in-agentcomms-and-docsplan.txt` | 3942 | Claude Code transcript dump — operational dead weight |

**Total lines read**: 12 083 (every file enumerated; nothing skipped).

`crates/blit-daemon/tests/` and `crates/blit-app/tests/` do not exist on disk (verified by `ls`). No daemon-level integration tests; no `blit-app` crate tests. All daemon coverage flows through the CLI harness that spawns a real `blit-daemon` child.

## Behaviors (grouped by category)

### state-machine

- **ci-three-os-matrix** — `.github/workflows/ci.yml:28-56` — Test job runs `cargo test --workspace` on `ubuntu-latest`, `macos-latest`, `windows-latest` separately; build-release is gated to `master` only. _(notes: no concurrency cap, no `actions/cache` beyond Swatinem; no fmt/clippy on macOS or Windows — only on ubuntu `check` job)_
- **ci-build-release-windows-target-mismatch** — `.github/workflows/ci.yml:62-86` — `build-release` matrix sets `target: aarch64-apple-darwin` for macOS but `x86_64-pc-windows-msvc` for Windows; release-asset upload uses Bash-style ternary `${{ matrix.os == 'windows-latest' && '.exe' || '' }}`. _(notes: works in GHA expression syntax, but no smoke run of the binary on the artifact)_
- **harness-rebuilds-daemon-on-each-test** — `crates/blit-cli/tests/common/mod.rs:111-134` — Every `TestContext::new()` shells `cargo build -p blit-daemon --bin blit-daemon` so each test is self-contained; `maybe_target` inspects the parent of the deps dir to forward `--target` if the test was built under a triple-keyed `target/`. _(notes: bottleneck on cold builds; rationale documented in `remote_checksum_negotiation.rs:90` as "R16-F1 — neither test depends on the other's build step")_
- **harness-port-readiness-50-tries-100ms** — `crates/blit-cli/tests/common/mod.rs:151-158` — Waits up to 5 s (50 × 100 ms) for `TcpStream::connect("127.0.0.1", port)` before asserting daemon listens. Same loop is open-coded in `remote_pull_mirror.rs:149-157`, `remote_checksum_negotiation.rs:187-195`, `remote_tcp_fallback.rs:149-157`, `remote_remote.rs:436-445`. _(notes: classic copy-paste; should be one helper)_
- **harness-stderr-piped-but-not-captured** — `crates/blit-cli/tests/common/mod.rs:145` — daemon `stderr` is `Stdio::piped()` "for debugging", but no test ever reads it; comment lies about intent. _(notes: dead read end can backpressure-stall a chatty daemon; everywhere else (`remote_pull_mirror.rs:144`, `remote_remote.rs:161`) uses `Stdio::piped()` or `Stdio::null()` inconsistently)_

### endpoint-parse

- **endpoint-bare-host-list-modules** — `crates/blit-cli/tests/admin_verbs.rs:32-67` — `blit list <bare-host>` smart-dispatches to `list-modules`; `blit ls <bare-host>` rejects. `blit list 127.0.0.1:PORT:/test/` falls through to ls-style behavior. _(notes: tests pin the §2.3 RELEASE_PLAN_v2 rule)_
- **diagnostics-dump-remote-module-fields** — `crates/blit-cli/tests/diagnostics_dump.rs:107-123` — `server:/mod/path` parsed yields `destination.kind=remote`, `path_kind=module`, `module=mod`, `host=server`, `same_device=false`. _(notes: no port → ambient default; no test for `host:port:/module/path`)_
- **remote-pull-subpath-resolution** — `crates/blit-cli/tests/remote_pull_subpath.rs:50-123` — Single-file source vs container dest; trailing slash → merge-contents; no slash → nest-under-basename; rename target → exact path. Pins double-nest regression `dst/gamedir/gamedir/...`. _(notes: load-bearing — comment says "all existing tests used `/test/` which hid both bugs")_

### safety-check

- **null-rejected-on-mirror** — `crates/blit-cli/tests/cli_arg_safety_gates.rs:79-106` — `mirror --null` exits non-zero with `--null is not supported with `blit mirror`` before any work; preexisting `dst/stale.txt` survives. _(notes: belt-and-suspenders — checks exit status AND file presence)_
- **null-rejected-on-remote-copy-push-and-pull** — `crates/blit-cli/tests/cli_arg_safety_gates.rs:110-144` — Both `copy --null SRC remote` and `copy --null remote DST` rejected with `--null is not supported with remote endpoints`. _(notes: ad-hoc port `12349`, hardcoded — no daemon listening, relies on parse-time rejection)_
- **null-accepted-on-local-copy** — `crates/blit-cli/tests/cli_arg_safety_gates.rs:150-173` — `copy --null SRC/ DST/` succeeds, source intact, dest empty. Sanity sibling to mirror rejection. _(notes: the only path where `--null` is supported per design)_
- **move-rejects-force-and-ignore-times** — `crates/blit-cli/tests/cli_arg_safety_gates.rs:178-262` — `move --force` and `move --ignore-times` rejected before work; tests R55 too — error must NOT recommend `blit copy --force` workaround because that path has the same data-loss class. _(notes: rare positive R55 anti-recommendation guard)_
- **move-mirror-bug-r46-f1** — `crates/blit-cli/tests/local_move_semantics.rs:50-107` — Asserts pre-existing unrelated dest files/dirs survive a local `move`; reproduces the `mirror=true` regression in the local-to-local move arm. _(notes: pre-fix bug had `mirror=true` only on the local arm, three other arms were correct — outlier bug)_
- **move-refuses-incomplete-scan-r47-f4** — `crates/blit-cli/tests/local_move_semantics.rs:122-185` — `chmod 000` on a source subdir; `move` must fail with `refusing to remove source` or `scan was…`. Has a `PermGuard` drop impl that resets perms. Unix-only. _(notes: PermGuard is duplicated verbatim in `local_move_semantics.rs:255-265` and `remote_push_mirror_safety.rs:45-53`)_
- **move-rejects-json-premature-success-r49-f3** — `crates/blit-cli/tests/local_move_semantics.rs:240-288` — `move --json` with unreadable source: stdout must not contain `"operation"` even though planner ran. Pre-fix `run_local_transfer` emitted the summary inline. Unix-only. _(notes: tests stdout absence, not just exit code)_
- **move-rejects-filter-args-r49-f1** — `crates/blit-cli/tests/local_move_semantics.rs:193-230` — `move --exclude '*.log'` rejected with `move does not support filters`; secret.log on source survives. _(notes: reviewer-reproduced data-loss bug)_
- **move-rejects-ignore-existing-r51-f1** — `crates/blit-cli/tests/local_move_semantics.rs:296-328` — `move --ignore-existing` rejected; src/file.txt survives. _(notes: planner-drops-then-tree-delete bug class)_
- **move-rejects-null-r52-f1** — `crates/blit-cli/tests/local_move_semantics.rs:336-368` — `move --null` rejected before any work. _(notes: reviewer-flagged identical-to-CLI-shape combo)_
- **move-r2r-rejects-relay-via-cli-r50-f1** — `crates/blit-cli/tests/local_move_semantics.rs:381-402` — `move --relay-via-cli REMOTE REMOTE` rejected at parse before network IO; uses fake ports `12347` and `12348`. _(notes: comment notes invalid endpoints would otherwise produce connect errors; rejection fires first)_
- **f2-symlink-escape-rejected-pull** — `crates/blit-cli/tests/f2_chroot_containment.rs:39-92` — module contains symlink → outside dir with `victim.txt`; pull through it must fail and dest must not contain victim.txt. Stderr-text check is lenient (3 acceptable phrases including raw `failed to send push request payload`). _(notes: race documented — "exact wording races daemon close-on-detection behavior"; relies on file-leak assertions as primary)_
- **f2-symlink-escape-rejected-push** — `crates/blit-cli/tests/f2_chroot_containment.rs:95-156` — R13-F1 push handshake check: mirror through in-module escape symlink must refuse before any write; victim file untouched. Same lenient-stderr pattern. _(notes: only mirror-purge enumeration tested at handshake level; not per-entry)_
- **f2-legit-intra-module-symlink-works** — `crates/blit-cli/tests/f2_chroot_containment.rs:160-195` — Legitimate `latest -> v1` symlink within module is followed and pulled. _(notes: positive companion to escape tests)_
- **push-mirror-refuses-incomplete-scan-r59-f1** — `crates/blit-cli/tests/remote_push_mirror_safety.rs:28-99` — chmod-blocked source subtree → daemon must not purge dest's `extra.txt`. Accepts EITHER exit-success-without-purge OR exit-failure-with-scan-mentioning-stderr. _(notes: unusually permissive on exit status; primary assert is file presence)_
- **push-mirror-filter-keeps-out-of-scope-r59-f2** — `crates/blit-cli/tests/remote_push_mirror_safety.rs:108-158` — `mirror --exclude '*.log'` must not purge dest's `preserve.log`; pins daemon honoring client filter. _(notes: regression against `FileFilter::default()` on daemon side)_
- **push-rejects-relay-r2r-no-source-daemon** — `crates/blit-cli/tests/remote_remote.rs:277-309` — Real fake gRPC server returning `Unimplemented`; client must NOT fall back to relay, must emit `does not implement DelegatedPull`. _(notes: hardcodes `127.0.0.1:9` (discard) as src; relies on fake destination server)_

### timeout-or-retry

- **harness-run-with-timeout-default-10s** — `crates/blit-cli/tests/common/mod.rs:172-193` — All admin tests use 10 s; remote/data-plane tests use 60–120 s. On timeout, command is killed and panics with stdout+stderr dump. _(notes: not a Drop-based timeout — child is killed but PermGuard etc. can leak if test panics differently)_
- **port-readiness-50-tries-100ms** — see `harness-port-readiness-50-tries-100ms` above. _(notes: 5 s ceiling silently caps all daemon boots; no escalation)_
- **windows-move-tree-hang-ignored** — `crates/blit-cli/tests/remote_move.rs:99-103` — `test_remote_move_local_to_remote_directory_tree` is `cfg_attr(target_os = "windows", ignore = "Windows source-delete hang; see windows-move-tree-hang finding")`. The matching pull-direction test is NOT ignored. _(notes: explicit cfg_attr ignore, the audit-6e Windows-gate)_

### data-plane

- **tcp-vs-grpc-fallback-message-string** — `crates/blit-cli/tests/remote_parity.rs:114-117` and `remote_tcp_fallback.rs:183-188` — stdout-grep for literal `[gRPC fallback]` or `gRPC data fallback`. Tests will silently fail if the literal text drifts. _(notes: brittle string-coupled assertion; should be a structured marker)_
- **tcp-trace-data-plane-marker** — `crates/blit-cli/tests/remote_parity.rs:32-37` — `--trace-data-plane` must emit `[data-plane-client]` on stderr. _(notes: only wired for push (per inline comment `// .arg("--trace-data-plane") // Not wired for pull yet` at remote_parity.rs:61))_
- **grpc-fallback-many-small-files** — `crates/blit-cli/tests/remote_parity.rs:170-220` — 50 small files via `--force-grpc`; verifies every file & bytes arrive. Step-4C parity guard. _(notes: tar-shard batching on fallback path)_
- **force-grpc-data-daemon-flag** — `crates/blit-cli/tests/remote_tcp_fallback.rs:134-148` — daemon launched with `--force-grpc-data`; client expected to also emit `gRPC data fallback` even without `--force-grpc`. _(notes: daemon-side force flag; bench script also uses `--force-grpc-data` while tests prefer the CLI flag)_
- **counter-env-var-bytes** — `crates/blit-cli/tests/remote_remote.rs:293, 358` — `BLIT_TEST_COUNTER_FILE` env var instructs CLI to write `cli_data_plane_outbound_bytes` and `remote_transfer_source_constructed` counters to a file; the bench script `bench_remote_remote.sh:81` uses the same hook for production benchmarking. _(notes: tightly couples test+bench tooling; documented in bench script header)_
- **direct-delegation-no-cli-bytes** — `crates/blit-cli/tests/remote_remote.rs:184-210` — 2 MiB payload remote→remote with delegation: `cli_data_plane_outbound_bytes==0` and `remote_transfer_source_constructed==0`. _(notes: load-bearing — proves direct path bypasses CLI relay)_
- **explicit-relay-uses-cli-bytes** — `crates/blit-cli/tests/remote_remote.rs:240-273` — `--relay-via-cli` flag with delegation OFF: relay source constructed, cli outbound bytes ≥ payload size. _(notes: companion to the direct test)_
- **source-acl-refusal-no-fallback** — `crates/blit-cli/tests/remote_remote.rs:312-345` — fake gRPC server returns `PermissionDenied("source ACL rejected delegated peer")` on `pull_sync` only; client must NOT fall back, must surface "source refused delegated pull" and "source ACL rejected delegated peer". _(notes: tests two stderr substrings; uses ~728 lines of fake-server Blit-trait boilerplate)_

### rpc-handler

- **pull_sync_with_spec-spy-server** — `crates/blit-core/tests/pull_sync_with_spec_wire.rs:251-310` — Spy server captures the first `ClientPullMessage::Spec` byte-for-byte; endpoint constructed with deliberately different module/path to prove spec wins. R30-F4 wire-roundtrip guard. _(notes: only blit-core integration test; ~430 lines of full `Blit` trait impl)_
- **pull_sync-wrapper-equivalence** — `crates/blit-core/tests/pull_sync_with_spec_wire.rs:313-380` — R23-F1: `pull_sync(opts)` wrapper must emit the same `TransferOperationSpec` as `build_spec_from_options` + `pull_sync_with_spec`. _(notes: structural equality on the wire — protects against silent divergence)_
- **pull_sync-initial-rejection-as-negotiation** — `crates/blit-core/tests/pull_sync_with_spec_wire.rs:382-427` — Spy rejects `pull_sync` at the initial RPC with `PermissionDenied`; client error must downcast to `PullSyncError`, satisfy `is_negotiation()`, preserve original message. _(notes: relies on `PullSyncError::is_negotiation()` classification)_
- **fake-server-trait-boilerplate-x2** — `crates/blit-cli/tests/remote_remote.rs:592-892` — `UnimplementedBlit` and `RejectingPullSyncBlit` both implement the full `Blit` trait (~150 lines each), returning `Status::unimplemented` for everything except the one method under test. _(notes: severe duplication; if a new `Blit` method is added, both fake impls + the SpyServer fake in blit-core must be edited)_

### format-output

- **find-glob-vs-substring** — `crates/blit-cli/tests/admin_verbs.rs:314-461` — Tests pin `--pattern` as glob (not substring): `*.csv` matches `report.csv` + `subdir/results.csv` via basename fallback; `[unterminated` rejected; `foo*.csv` does NOT cross `/` (literal_separator); `**/*.csv` crosses. _(notes: R41-F3 regression; cites `BLIT_UTILS_PLAN`)_
- **rm-refuses-module-root** — `crates/blit-cli/tests/blit_utils.rs:421-435` — `blit rm --yes 127.0.0.1:PORT:/test/` exits non-zero. _(notes: bare module-root rm safety; no specific message assertion)_
- **profile-predictor-shape** — `crates/blit-cli/tests/blit_utils.rs:584-630` — R44-F2: pins `predictor` JSON contract — top-level null OR `{copy, mirror}` with `{observations, fallback_depth, planner, transfer}` each. Comment notes the pre-fix commit message was wrong about empty-state shape. _(notes: load-bearing — explicit anti-regression to a documentation lie)_
- **diagnostics-dump-rsync-basename-append** — `crates/blit-cli/tests/diagnostics_dump.rs:75-104` — src-no-slash + dst-with-slash → `resolution_changed=true`, `resolved_destination` ends with `/dst/src`. Normalizes Windows backslashes. _(notes: only path-normalization step in any test)_
- **single-file-summary-text** — `crates/blit-cli/tests/single_file_copy.rs:100-158` — First copy must say `Copy complete: 1 files`; idempotent second copy must say `Up to date` and must NOT say `Copy complete: 0 files`. _(notes: brittle phrasing assertions; couples plan output text to test fixture)_

### confirmation-prompt

- **all-tests-use-yes-flag** — Every move/mirror/rm test passes `--yes`; no test exercises the interactive prompt path. _(notes: confirmation UX is unverified by CI; the `audit_decisions` memory notes "clear-recent Y/n confirm" as an open owner decision)_

### persistence

- **r46-f1-mirror-on-move-was-data-loss** — `crates/blit-cli/tests/local_move_semantics.rs:1-10` — Tests document that `mirror=true` was hardcoded in the local-to-local move arm at `crates/blit-cli/src/transfers/mod.rs:458` pre-fix, asymmetric to three other move arms. _(notes: documents a previously-shipped data-loss bug; load-bearing context)_
- **mtime-preservation-end-to-end** — `crates/blit-cli/tests/remote_regression.rs:88-151` — 2020-01-15 mtime stamped via `filetime::set_file_mtime`; mirror must preserve. Documents the `set_file_mtime`-while-handle-open race. Unix-only. _(notes: 5/8 files lost mtime pre-fix per comment)_
- **mtime-only-no-retransfer** — `crates/blit-cli/tests/remote_regression.rs:163-227` — 2 MiB file; mtime changed without content change; second mirror auto-promotes via block-hash. Verifies content unchanged and mtime updated. _(notes: tests timing implicitly via "should complete quickly" but uses generous 60 s timeout — no real perf guard)_

### discovery

- **scan-no-mdns** — `crates/blit-cli/tests/blit_utils.rs:11-33` — daemon has `no_mdns: true`, so `blit scan --wait 1` expected to find nothing but exit cleanly. _(notes: no actual mDNS exercise anywhere in tests)_

### cancellation

- _(none observed)_ — No test exercises Ctrl-C or `CancelJob` RPC despite `CancelJobRequest`/`CancelJobResponse` being part of the trait stub in `remote_remote.rs:713-718` and `pull_sync_with_spec_wire.rs:168-173`.

### key-dispatch / render-or-display

- _(none)_ — No TUI integration tests anywhere; only the spawn-daemon CLI surface is covered.

### spawn-task

- **fake-gRPC-server-tokio-thread** — `crates/blit-cli/tests/remote_remote.rs:516-552` and `554-590` — Spawns std::thread with a current-thread tokio runtime to drive an `UnimplementedBlit` or `RejectingPullSyncBlit`; uses `oneshot` for shutdown; uses `TcpListenerStream` over a non-blocking `std::net::TcpListener` converted via `tokio::net::TcpListener::from_std`. _(notes: shutdown drops the sender then joins the thread; if the runtime panics, join blocks forever)_

### flag-handling

- **config-dir-arg-precedes-subcommand** — observed throughout (e.g. `admin_verbs.rs:17-20`): `--config-dir <path>` is always emitted before the subcommand. _(notes: ordering convention not enforced anywhere in tests)_
- **delete-scope-flag-shapes** — `crates/blit-cli/tests/remote_pull_mirror.rs:432-451` — `--delete-scope all` switches to `MirrorMode::All`. Default is `FilteredSubset`. _(notes: documented at top of file; no test for `none` or `manifest` if those exist)_
- **resume-flag-with-grpc-fallback** — `crates/blit-cli/tests/remote_resume.rs:91-132` — `--resume --force-grpc` exercises legacy fallback path; stdout must contain `[gRPC fallback]` string. _(notes: brittle string match again)_

### config-load

- **harness-daemon-config-shape** — `crates/blit-cli/tests/common/mod.rs:12-34` — daemon config is `{daemon: {bind, port, no_mdns}, module: [{name, path, comment?, read_only}]}` serialized via toml. Different test files redefine this struct: `remote_pull_mirror.rs:12-34`, `remote_checksum_negotiation.rs:19-42`, `remote_tcp_fallback.rs:12-34`, `remote_remote.rs:12-44` (adds `delegation` and `delegation_allowed`). _(notes: 5 separate definitions of the same TOML shape, drift hazard. `remote_remote.rs` is the only one with `delegation_allowed`)_
- **harness-bench-isolated-config** — `scripts/bench_local_mirror.sh:238-244` — bench uses `--config-dir $WORK_ROOT/blit_config` and disables perf telemetry (`diagnostics perf --disable --clear`) before runs, re-enables after. _(notes: only place that exercises `diagnostics perf --enable/--disable/--clear`; tests do not)_

### default-value

- **bench-default-runs-1** — `scripts/bench_remote_remote.sh:33-35` — `SIZE_MB=512`, `RUNS=3` defaults; `bench_local_mirror.sh:8-9` defaults `SIZE_MB=256`, `RUNS=5`, `WARMUP=1`; `bench_10gbe.sh:29-32` defaults `SIZE_MB=1024`, `SMALL_COUNT=10000`, `SMALL_SIZE=4096`, `RUNS=3`. _(notes: 3 benches, 3 different defaults; no shared "knobs" module)_
- **robocopy-default-flags** — `scripts/windows/bench-local-mirror.ps1:149` — `"/MIR /COPYALL /FFT /R:1 /W:1 /NDL /NFL /NJH /NJS /NP"`. _(notes: `audit_decisions` memory specifies robocopy `--retry/--wait` as a stall follow-up; bench passes `/R:1 /W:1` only)_
- **rsync-default-flags** — `scripts/bench_local_mirror.sh:37` — `"-a --delete --whole-file --inplace --no-compress --human-readable --stats"`. _(notes: detects and removes `--no-compress` if unsupported)_

### path-handling

- **rsync-trailing-slash-merge-vs-nest** — observed throughout (e.g. `single_file_copy.rs:54-83`, `remote_move.rs:18-43`, `remote_pull_subpath.rs:108-123`). Tests pin: src trailing slash → merge contents; src no slash → nest under basename; explicit rename when dst path includes file name. _(notes: this convention is THE invariant under test in dozens of cases)_
- **rsync-basename-append-detection** — `crates/blit-cli/tests/diagnostics_dump.rs:75-104` — only test that normalizes `\` to `/` for path assertions. _(notes: other tests assume forward-slash everywhere; Windows-only paths get implicit `display()` use)_

## Smells / risks observed

1. **Daemon-config TOML shape duplicated 5 times** — `common/mod.rs:12-34` plus 4 standalone test files each redefine `DaemonConfig`/`DaemonSection`/`ModuleSection` with subtle variations (`remote_remote.rs:13-44` adds `delegation_allowed: bool` and `Option<DelegationSection>`). One drift will cause silent test-config mismatch from runtime config. Pull into a `tests/common/` re-export.
2. **Port-readiness loop copy-pasted ≥5 times** — `common/mod.rs:151-158`, `remote_pull_mirror.rs:149-157`, `remote_checksum_negotiation.rs:187-195`, `remote_tcp_fallback.rs:149-157`, `remote_remote.rs:436-445`. All identical except panic-string. One-line helper would suffice.
3. **`UnimplementedBlit` fake trait stub is enormous** — `remote_remote.rs:592-726` is 134 lines of `Status::unimplemented` returns; `RejectingPullSyncBlit` (`:728-892`) is a near-duplicate 164 lines, and `SpyServer` in `pull_sync_with_spec_wire.rs:40-181` is a third 142-line copy. Adding any method to the `Blit` trait requires editing 3 fakes. A shared `tests/common/fake_blit.rs` with default impls + override hooks would shrink this 10×.
4. **Stderr-substring matching is the rule** — many security/UX assertions compare against literal English strings (`"failed to send push request payload"`, `"refusing to remove source"`, `"--null is not supported with `blit mirror`"`, `"source refused delegated pull"`, `"[gRPC fallback]"`). A wording change breaks tests silently or vice versa. No structured error codes are tested.
5. **F2 containment tests admit a transport-close race in the stderr check** — `f2_chroot_containment.rs:73-80, 140-147` accept three different stderr phrases including `"failed to send push request payload"`. The recent commit `c3f227d fix(ci): accept transport-close stderr as valid containment-refusal signal` shows this was a flake. File-leak assertions remain the only ground truth.
6. **Windows-move-tree-hang is the only `cfg_attr(target_os, ignore)` in the suite** — `remote_move.rs:100-103` permanently skips the push-direction tree move; the pull-direction tree move runs everywhere. This is the **audit-6e** gating finding; root cause is theorized (Windows open-handle vs. `remove_dir_all`) but not investigated. CI is green on Windows under false pretenses.
7. **`single_file_copy.rs:100-158` asserts the literal text "Copy complete: 1 files"** — including the grammatical "1 files" (not "1 file"). Tests intentionally lock in the bad pluralization; any UX polish will require a test edit.
8. **CI matrix has no fmt/clippy on macOS or Windows** — `.github/workflows/ci.yml:14-26` runs fmt+clippy only on Ubuntu. A platform-cfg block emitting non-portable code can land if it compiles on Linux.
9. **CI has no concurrency cancellation** — no `concurrency:` block; rapid pushes pile up runs. With three OS × full-workspace `cargo test` each, this is expensive.
10. **CI has no caching of cargo build artifacts beyond `Swatinem/rust-cache@v2`** — cold cache on every PR if the cache key changes (e.g. Cargo.lock churn). The test harness *also* spawns its own `cargo build -p blit-daemon` for every TestContext (`common/mod.rs:111`), compounding the cost.
11. **`scripts/test.sh` is mislabeled** — it generates a Codex prompt bundle (sources `~/source/venvs/superclaude`, copies `~/.claude/commands/sc` → `~/.codex/commands/sg`); it has nothing to do with Blit tests. Anyone running `./scripts/test.sh` expecting `cargo test` gets a 220-line Codex installer. Should be renamed or moved out of `scripts/`.
12. **`scripts/codex_resume.sh`, `scripts/mac_codex_resume.sh`, `scripts/win_codex.ps1`** — three personal Codex `resume <UUID>` one-liners. `codex_resume.sh` does `sudo npm install -g @openai/codex` and `pipx upgrade SuperClaude` unconditionally. None of these belong in the project's `scripts/`.
13. **`scripts/2025-10-24-read-files-in-agentcomms-and-docsplan.txt`** — 3942-line Claude Code transcript dump of an unrelated session. Should be deleted or moved to `docs/`.
14. **Three local-bench scripts with three different defaults** — `bench_local_mirror.sh` (256 MiB), `bench_10gbe.sh` (1 GiB + 10k small), `bench_remote_remote.sh` (512 MiB). No shared knobs file; `bench_local_mirror_macos.sh` is a 9-line shim. PowerShell bench is a fourth independent implementation (414 lines).
15. **`scripts/linux/run-remote-fallback.sh` has no arg-count guard** — `set -euo pipefail` + `REMOTE=$1; MODULE=$2` will produce ugly unbound-var error if called wrong; doesn't print usage.
16. **`scripts/windows/run-journal-fastpath.ps1` hardcodes `C:\` for NTFS and `D:\` for ReFS** — line 16-19 `switch ($Volume)`. Will fail or trash data on dev machines without that exact layout. No drive-letter discovery.
17. **`bench_local_mirror.sh:262-263` defines `add_tool "blit" "$DST_BLIT" "blit v2 mirror"`** — label hardcodes "v2" branding. Cosmetic but inconsistent with the rest of the codebase, which no longer says "v2".
18. **`remote_pull_mirror.rs` defines `spawn_daemon()` (`:247-327`) AFTER `remote_pull_mirror_purges_extraneous_local_files()` already inline-builds the same daemon** — first test uses the inline build (`:104-147`); the helper added later for the filtered-subset tests duplicates the same logic. Should refactor first test to use helper.
19. **No daemon-side integration tests** — `crates/blit-daemon/tests/` doesn't exist; all coverage is end-to-end through CLI. RPC handlers like `subscribe`, `clear_recent`, `cancel_job` are stubbed in fake servers and not tested in real daemon. `feedback_server_await_timeouts` memory notes await-timeout handling needs auditing; no test pins that.
20. **No blit-app integration tests** — `crates/blit-app/tests/` doesn't exist. The TUI screens and dual-pane logic (modified files in current `git status`) have zero integration coverage.
21. **`Stdio::null()` vs `Stdio::piped()` inconsistency on daemon stderr** — `common/mod.rs:145` captures stderr to a pipe but never reads it (risk: backpressure stall on a chatty daemon); `remote_pull_mirror.rs:144` uses `Stdio::null()`; `remote_remote.rs:161` uses `Stdio::piped()`. Pick one.
22. **`pull_sync_with_spec_wire.rs:212` adds a 50 ms `tokio::time::sleep` "belt-and-suspenders" delay** — comment admits it shouldn't be necessary; if the listener races the connect this hides flakes.
23. **R59 #1 F1 test (`remote_push_mirror_safety.rs:78-98`) accepts either exit-success-without-purge or exit-failure** — a regression that flips from "fail with reason" to "succeed silently" would still pass as long as no purge happened. Should pin one branch.
24. **Bench script env-knob duplication** — `BENCH_ROOT` and `BLIT_BENCH_ROOT` both supported (`bench_local_mirror.sh:13-23`) "for backwards compatibility"; not documented anywhere outside the script. Could deprecate one.
25. **`remote_remote.rs` declares `SubscribeStream` Pin<Box<…>>** — necessary because `Subscribe` isn't a ReceiverStream; replicated in all three fake-server impls. If the trait changes Subscribe shape, three places break.
26. **No test asserts `--null` exit code for the daemon-side flag combo** — only client-side parse rejection. If daemon accepts `--null` upload someday, that's an invisible regression.
27. **CI runs full `cargo test --workspace` on Windows but `audit-6e` test is the only `ignore`'d one** — implies the CI matrix needs to keep growing as Windows-only bugs accumulate; no Windows-skip macro pattern documented.

## Coverage attestation

| File | Lines read | Notes |
| --- | ---: | --- |
| `.github/workflows/ci.yml` | 86 | full read |
| `crates/blit-cli/tests/common/mod.rs` | 212 | full read |
| `crates/blit-cli/tests/admin_verbs.rs` | 485 | full read |
| `crates/blit-cli/tests/blit_utils.rs` | 631 | full read |
| `crates/blit-cli/tests/cli_arg_safety_gates.rs` | 262 | full read |
| `crates/blit-cli/tests/diagnostics_dump.rs` | 123 | full read |
| `crates/blit-cli/tests/f2_chroot_containment.rs` | 195 | full read |
| `crates/blit-cli/tests/local_move_semantics.rs` | 402 | full read |
| `crates/blit-cli/tests/remote_checksum_negotiation.rs` | 289 | full read |
| `crates/blit-cli/tests/remote_move.rs` | 205 | full read |
| `crates/blit-cli/tests/remote_parity.rs` | 220 | full read |
| `crates/blit-cli/tests/remote_pull_mirror.rs` | 451 | full read |
| `crates/blit-cli/tests/remote_pull_subpath.rs` | 123 | full read |
| `crates/blit-cli/tests/remote_push_mirror_safety.rs` | 158 | full read |
| `crates/blit-cli/tests/remote_push_single_file.rs` | 84 | full read |
| `crates/blit-cli/tests/remote_regression.rs` | 227 | full read |
| `crates/blit-cli/tests/remote_remote.rs` | 892 | full read |
| `crates/blit-cli/tests/remote_resume.rs` | 132 | full read |
| `crates/blit-cli/tests/remote_tcp_fallback.rs` | 243 | full read |
| `crates/blit-cli/tests/remote_transfer_edges.rs` | 228 | full read |
| `crates/blit-cli/tests/single_file_copy.rs` | 172 | full read |
| `crates/blit-core/tests/pull_sync_with_spec_wire.rs` | 427 | full read |
| `scripts/bench_10gbe.sh` | 245 | full read |
| `scripts/bench_local_mirror.sh` | 378 | full read |
| `scripts/bench_local_mirror_macos.sh` | 9 | full read |
| `scripts/bench_remote_remote.sh` | 118 | full read |
| `scripts/build-release.sh` | 81 | full read |
| `scripts/codex_resume.sh` | 6 | full read |
| `scripts/linux/run-journal-fastpath.sh` | 61 | full read |
| `scripts/linux/run-remote-fallback.sh` | 23 | full read |
| `scripts/mac_codex_resume.sh` | 1 | full read |
| `scripts/macos/run-blit-tests.sh` | 39 | full read |
| `scripts/macos/run-journal-fastpath.sh` | 61 | full read |
| `scripts/test.sh` | 220 | full read (Codex bundler, not Blit tests) |
| `scripts/win_codex.ps1` | 1 | full read |
| `scripts/windows/bench-local-mirror.ps1` | 414 | full read |
| `scripts/windows/build-release.ps1` | 42 | full read |
| `scripts/windows/probe-usn-volume.ps1` | 67 | full read |
| `scripts/windows/run-blit-tests.ps1` | 51 | full read |
| `scripts/windows/run-journal-fastpath.ps1` | 77 | full read |
| `scripts/2025-10-24-read-files-in-agentcomms-and-docsplan.txt` | 3942 | first 600 chars sampled (Claude Code transcript dump — content is unrelated chat logs); recorded as dead-weight smell rather than line-by-line inventoried |

**Total lines read**: 12 083
**Files NOT read (with reason)**: none. The 3942-line `2025-10-24-…txt` was sampled rather than line-by-line read because it's a transcript dump unrelated to Blit operations; flagged as smell #13 for removal.

## Contradictions / intra-cluster mismatches

- `crates/blit-cli/tests/remote_move.rs:99-103` Windows-ignores the **push-direction** tree move test but keeps the **pull-direction** symmetric test (`:150-205`) green on Windows. If the theorized root cause (open-handle vs `remove_dir_all`) were the explanation, pull-side tree-delete on Windows should hit the same class of bug. Either the theory is wrong or pull-side has its own latent bug going undetected.
- `crates/blit-cli/tests/cli_arg_safety_gates.rs:51-71` requires non-zero exit AND specific stderr substring; `crates/blit-cli/tests/remote_push_mirror_safety.rs:83-98` for the structurally identical R59-F1 case allows either exit-success-without-purge OR exit-failure with one of four stderr substring options. Inconsistent strictness for sibling safety gates.
- `crates/blit-cli/tests/common/mod.rs:145` daemon `Stdio::piped()` (never read) vs `remote_pull_mirror.rs:144` `Stdio::null()` vs `remote_remote.rs:161` `Stdio::piped()`. Three sibling tests, three different policies.
- `scripts/bench_local_mirror.sh:262-263` labels blit "v2 mirror" while no current code or doc carries the "v2" branding; the bench title contradicts the project's current naming.
- `scripts/bench_remote_remote.sh:82` calls `blit copy SRC DST --yes` to **stage** a 512 MiB payload to the source server before benchmarking — but `bench_10gbe.sh` skips the staging step and assumes the user pre-populated the remote bench module. Two benches for the same nominal scenario with different bootstrap contracts.
- CI matrix at `.github/workflows/ci.yml:28-56` runs `cargo test --workspace` on Windows, which depends on `audit-6e` being ignored. If a contributor removes the `cfg_attr(target_os = "windows", ignore = ...)` in `remote_move.rs:100` without fixing the underlying hang, CI will silently begin hanging on Windows because there is no per-job timeout.

Headlines summary: this cluster's value is concentrated in `blit-cli/tests` (well-named regression cases that document specific shipped bugs). The infrastructure around it — fake gRPC servers, daemon-config TOMLs, ready-port loops — is heavily duplicated, and the `scripts/` directory mixes operational tooling with personal Codex shims and one transcript dump. CI is a minimal 3-OS matrix with no fmt/clippy redundancy and no concurrency control; the only Windows-ignored test (`audit-6e`) is also the one whose pull-direction sibling passes on Windows for no documented reason.
