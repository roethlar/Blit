# Blit v2 TODO

This is the master checklist. Execute the first unchecked item in
the "Current Review Follow-up" section, or pick up other unchecked
work from later sections. After completion, check the box and add
an entry to `DEVLOG.md`. Items in "Deferred design calls" below
are intentionally not next-actionable — skip them unless the
prerequisite (typically benchmark data) has changed.

## Current Review Follow-up

See `docs/reviews/codebase_review_2026-05-01.md` for the original
codebase review and `docs/reviews/followup_review_2026-05-02.md`
for the followup review series. Pipeline architecture
clarification in `docs/plan/PIPELINE_UNIFICATION.md`.

Status: **14 of 15 baseline findings closed**, every followup
review round to date closed. The only remaining open item is the
explicitly-deferred logging epic (F15).

### Pipeline unification sequence

- [x] **F1 — P0** Centralize receive-side path sanitization. Shared
      `safe_join` helper in `blit-core`. Applied at every receive-sink
      path-join site. Migrated `pull.rs::sanitize_relative_path` and
      daemon `service/util.rs` validators into the shared module.
      Adversarial tests in place. *(Landed `cc77074`. Followup
      review rounds 1, 5–6 closed R1-F1, R1-F3, R5-F1, R5-F2,
      R6-F1, R6-F2, R6-F3.)*
- [x] **TransferOperationSpec** Proto messages
      (`TransferOperationSpec`, `FilterSpec`, `ComparisonMode`,
      `MirrorMode`, `ResumeSettings`, `PeerCapabilities`) defined and
      wired through. Step 4A landed; daemon normalizes via
      `NormalizedTransferOperation::from_spec` (R2-F2, R4-F1).
- [x] **DiffPlanner extraction** Diff/comparison/payload-planning
      split out into `blit-core::remote::transfer::diff_planner`.
      Local-mirror, push, and pull all route through it now (Steps
      3a, 3b, 4).
- [x] **pull_sync.rs refactor** Pull side runs the unified pipeline
      via `FsTransferSource → DiffPlanner → execute_sink_pipeline →
      sink`. Filter parity is real; CLI bail-on-filter-args removed
      (Steps 4B + R4-F3).

### Original baseline findings (`codebase_review_2026-05-01.md`)

- [x] **F1 — P0** Receive-side path sanitization. *(See above.)*
- [x] **F2 — P0** Daemon canonical containment. `path_safety::
      contained_join` / `verify_contained` is the always-on
      chokepoint at every daemon read/write site. F2 integration
      tests cover symlink-escape rejection across pull, push
      destination_path, and mirror purge. R13-F1 closure added the
      handshake-level rejection on push (commit `0d4d2fb`).
- [x] **F3 — Medium** Reframed as docs-only: `0.0.0.0` default is
      intentional for a network file daemon. Trust model + exposure
      expectations now documented in `docs/DAEMON_CONFIG.md` (commit
      `35068b8`).
- [x] **F4 — P1** Filtered mirror delete semantics. Daemon ships an
      authoritative `DeleteList`; `MirrorMode::FilteredSubset`
      (default) vs `MirrorMode::All` (`--delete-scope all` opt-in).
      Step 4B.
- [x] **F5 — Medium** `TransferMetrics::enter_transfer()` returns
      an `ActiveGuard` RAII handle that releases the gauge on Drop
      (panic/cancel-safe). `inc_purge` moved to dispatch boundary so
      counter semantics are consistent across push/pull/purge.
- [x] **F6 — Medium** Metrics HTTP server removed entirely; counters
      are opt-in via `--metrics` and live in-process for a future
      GUI/TUI consumer.
- [x] **F7 — Medium** `RemoteTransferSource::prepare_payload` size
      validation + bounded read via `take(size + 1)` so a hostile
      remote source can't grow the relay's allocation past
      `MAX_TAR_SHARD_BYTES` (closes the R6-F1 mirror on the send
      side; R11-F1 closed the read-bound issue).
- [x] **F8 — Medium** `MAX_WIRE_TAR_SHARD_BYTES` derived from
      `tar_safety::MAX_TAR_SHARD_BYTES` — single source of truth.
- [x] **F9 — Medium** `execute_local_mirror_async` exposed for
      async callers; `execute_local_mirror` is the sync wrapper.
      No more nested-runtime hazard.
- [x] **F10 — Medium** Filter parity on remote pull — daemon honors
      `FilterSpec` via `FileEnumerator`. CLI no longer bails on
      filter args for remote-source transfers (Step 4B).
- [x] **F11 — Medium** Pull `PullSyncAck.server_checksums_enabled`
      stored on `RemotePullReport`; client errors at handshake when
      `--checksum` is requested but the daemon has it disabled.
      R15-F1 made this reachable from the CLI by splitting the
      remote-transfer gate into pull/push variants.
- [x] **F12 — Low/Medium** `blit check` equivalence model
      documented in `--help` and `CheckArgs` rustdoc. 6 unit tests
      pin the behavior (regular files, empty dirs, symlinks,
      file-vs-directory, missing-on-dest, one-way).
- [x] **F13 — Low/Medium** `use_chroot` config field removed
      entirely (containment is always-on per F2). Workflow docs
      synced. TODO.md (this section) synced.
- [x] **F14 — Low** Migrated `change_journal/snapshot.rs` from
      `fsevent_sys::FSEventsGetCurrentEventId` to
      `objc2_core_services::FSEventsGetCurrentEventId`
      (`objc2-core-services` crate, `FSEvents` feature). Same call
      shape and `u64` return — only the binding crate changed. Both
      deprecation warnings on macOS builds are gone.
- [ ] **F15 — Low** Adopt `tracing` or structured `log` across daemon
      and transfer modules. Gate noisy data-plane logs. Explicitly
      deferred per `docs/plan/PROJECT_STATE_ASSESSMENT.md`.

### New audit findings (2026-05-23)

- [x] **audit-13** Double-locking and redundant memory zeroing in `BufferPool` release/return paths.
- [x] **audit-14** Redundant `seek` system calls in sequential block-level resume copies.
- [x] **audit-15** Missing request/idle timeouts on tonic gRPC server control plane.

### New audit findings (2026-07-06)

- [ ] **audit-16** `spawn_manifest_task`'s "Enumerated N entries… (streaming
      manifest)" heartbeat (`crates/blit-core/src/remote/push/client/helpers.rs:114-183`)
      prints unconditionally on a 1s wall-clock timer — no `--verbose` gate.
      `docs/plan/LOCAL_TRANSFER_HEURISTICS.md:42` (Status: Historical)
      documents the original intent as verbose-gated ("`--verbose` shows
      real-time heartbeat messages… Default mode remains quiet unless a
      stall occurs"); the shipped code never wired that check in, so every
      `copy`/`mirror` run spams the heartbeat regardless of verbosity — local
      mirror hits this same manifest-scan path too, per
      `engine/strategy.rs:81` (mirror/checksum/force-tar always take the
      streaming path, which is unified with remote push). Fix: gate the
      print in `spawn_manifest_task` behind the caller's verbose option
      (the function has no visibility into CLI args today — thread a flag
      through `LocalMirrorOptions`/the equivalent remote-push options).
      Related, already filed, do not re-file: `--progress` (`-p`)
      auto-enabling on TTY regardless of the flag is
      `docs/audit/findings/drift-principles.md`
      (`drift-spinner-vs-quiet-default-decision-conflict`).
- [ ] **audit-17** Local `copy` aborts the entire transfer on one
      filename the destination filesystem rejects, instead of
      skipping/warning and continuing. Reported: `blit copy
      /home/michael/ /run/media/michael/8247-7E92/michael -ypv`
      failed enumerating ~88k entries in, at
      `crates/blit-core/src/remote/transfer/sink.rs:605`
      (`write_tar_shard_payload`'s parallel-write closure) —
      `std::fs::create_dir_all` on a NuGet http-cache path whose
      final component is `670c1461c...$ps:_api.nuget.org_v3_index.json`
      returned `Invalid argument (os error 22)`. The source filename
      is valid on the source (Linux/ext4) fs but contains a `:`,
      which is illegal on FAT/exFAT/NTFS-strict destinations.
      *Assumption, unverified — drive wasn't mounted to confirm:*
      the destination (`/run/media/...`, hex volume label
      `8247-7E92`, the classic Linux label format for an
      unlabeled FAT-family volume) is exFAT or FAT32. Grepped
      `crates/blit-core/src` for existing invalid-filename handling
      (`os error 22`, `sanitize_name`, `illegal.*char`, etc.) — none
      found; this is a real gap, not a regression. One bad name
      currently kills the whole run rather than being
      skipped/reported/renamed. Fix needs a design call (owner
      input required, `plan` this before coding): per-file
      skip-and-report vs. optional name-sanitization vs. fail-fast
      with a clear top-level error instead of a raw `os error 22` +
      internal path/line. Whatever is chosen must apply uniformly
      to both the local-mirror and remote tar-shard receive paths
      (same `write_tar_shard_payload` helper backs both, per
      audit-16 above). **Confirmed reproducible**: identical failure
      (same path, same error, same line, same ~88129-entry offset)
      recurred verbatim on a second run targeting a different mount
      (`/run/media/michael/USB_DRIVE/michael`) — not a one-off fluke;
      the FAT-family-destination assumption above is now corroborated
      by a second independent mount exhibiting the same `:`-rejection.
      **Second manifestation, same closure, one line down**: after
      that NuGet-cache directory issue was worked past (retried on a
      cleared destination), the identical `os error 22` recurred at
      `sink.rs:608` — this time `std::fs::write` on a regular file
      whose name is `frostfell06.dds:crc` (unrelated content, a game
      asset tree, not NuGet). Confirms the bug is general to any path
      component containing an illegal character, not create_dir_all-
      specific or NuGet-cache-specific — the fix must cover both the
      `create_dir_all` (line 605) and `write` (line 608) call sites in
      the same closure (and their mirror in the local-mirror path, per
      the note above).
- [ ] **audit-18** Non-UTF-8 source filenames are silently corrupted
      during enumeration, then fail to open, aborting the whole
      transfer. Reported: `blit copy /home/michael/
      /run/media/michael/USB_DRIVE/michael -ypv` (same host as
      audit-17, later re-run) failed ~377k entries in at
      `crates/blit-core/src/remote/transfer/payload.rs:360`
      (`build_tar_shard`'s `std::fs::File::open(&full_path)`) with
      `No such file or directory (os error 2)` opening
      `.../Claudia-by-Choice-codex_1.0<U+FFFD>` — the trailing
      replacement character is the tell. Root cause:
      `crate::path_posix::relative_path_to_posix` (`path_posix.rs:36-44`)
      builds the canonical relative-path `String` with
      `c.as_os_str().to_string_lossy()` per component — on Linux/ext4
      a filename can be any non-`/`/non-NUL byte sequence, and a
      non-UTF-8 one gets its invalid bytes replaced with U+FFFD
      *irreversibly*. `build_tar_shard` then does
      `source_root.join(rel)` on that corrupted string, which no
      longer names the real file → `ENOENT`. `FileEntry.path` itself
      (`fs_enum.rs:14`) is a real `PathBuf` (exact bytes preserved),
      so the corruption happens specifically at the `PathBuf → String`
      relative-path conversion, not at enumeration. `relative_path_to_posix`
      is the single canonical helper for this and is called from
      `engine/mirror.rs`, `mirror_planner.rs`, `remote/transfer/{payload,tar_safety}.rs`,
      `remote/endpoint.rs`, and `remote/push/client/helpers.rs` — i.e.
      local mirror and remote push both go through the lossy path, so
      this isn't a remote-only/proto-only constraint. Fix needs a
      design call (owner input required, `plan` this before coding):
      the wire `FileHeader.relative_path`/`FileBlock.relative_path`
      fields are proto3 `string`, which is UTF-8-only at the gRPC
      layer, so a full fix for the remote path needs an encoding
      scheme that round-trips arbitrary bytes through a UTF-8-safe
      string (e.g. percent-encode invalid bytes, or WTF-8) — local
      mirror has no such wire constraint and could preserve raw
      `OsString`/`PathBuf` throughout instead. Same failure class as
      audit-17 (one bad filename kills the entire run instead of
      being skipped/reported) — whatever skip/report/fail-fast
      behavior gets designed for audit-17 should likely cover this
      case too, but the root cause here is enumeration-side path
      corruption, not destination-fs charset rejection, so treat as
      a separate fix even if the error-handling policy ends up shared.
- [ ] **audit-19** `--exclude` silently matches nothing for the path
      forms users actually type (absolute paths, and bare directory
      names), so an exclude that looks correct transfers the excluded
      tree anyway. Reported: `blit mirror /home/michael/
      /run/media/michael/USB_DRIVE -pvy --force --exclude
      /home/michael/.java` still descended into `.java/` and tried to
      write `.java/fonts/1.8.0_472/fcinfo-…-en.properties` — that write
      then hit `audit-17`'s `os error 22` (`sink.rs:608`) on the
      FAT-family destination, i.e. a working `--exclude` would also have
      side-stepped that crash. The filter *is* plumbed (local mirror
      enumerates via `FileFilter`, `transfers/local.rs:191` →
      `enumerate_directory_filtered`); this is a matching-semantics bug,
      not a dropped-filter plumbing bug. Two compounding root causes in
      `FileFilter::allows_entry` (`crates/blit-core/src/fs_enum.rs:194-240`):
      (1) **excludes are matched against the source-root-relative path
      and the bare filename, never the absolute path** — `path_str` is
      `rel_path` (`fs_enum.rs:211-213`), `filename` is
      `abs_path.file_name()` (`fs_enum.rs:207-210`). The candidate
      strings for this entry are `.java/fonts/…` (relative) and
      `fcinfo-…properties` (filename); a literal `/home/michael/.java`
      glob equals neither (globset needs a whole-string match, and
      `glob_match` with no `*` falls through to exact equality,
      `fs_enum.rs:399-417`). `--exclude` maps only to `exclude_files`
      (`crates/blit-app/src/transfers/filter.rs:42`); nothing strips the
      source prefix to make an absolute pattern relative, so an absolute
      exclude under the source root is structurally unmatchable.
      (2) **a directory pattern does not prune its subtree.** Even the
      "correct" relative form `--exclude .java` only drops an entry
      whose relative path or filename is exactly `.java`; the files
      under it are `.java/fonts/…` and globset `*` does not cross `/`,
      so the whole subtree still transfers. There is **no `--exclude-dir`
      flag** (verified: no CLI arg in `blit-cli`/`blit-app`, and
      `FileFilter::exclude_dirs` / `dir_globs` / `should_include_dir`
      at `fs_enum.rs:274-295` is never assigned anywhere in `crates/`),
      so the only incantation that works today is
      `--exclude '.java/**'` (and likely `--exclude .java` too, for the
      dir entry itself). Nothing warns that a pattern matched zero of
      the configured globs — a silent no-op, same foot-gun class as the
      endpoint-parse open question in `docs/STATE.md`. Docs gap:
      `--help` says only "Exclude files matching this glob pattern"
      (`crates/blit-cli/src/cli.rs:292`) with no hint that matching is
      source-relative (not absolute) or that a directory needs `/**`;
      rsync users reasonably expect leading-`/`-anchors-to-transfer-root
      and trailing-`/`-matches-dir semantics, none of which blit
      implements. Confirmed by reading the matcher end-to-end, not run.
      Fix needs a design call (owner input required, `plan` this before
      coding): options span (a) accept absolute patterns under the
      source root by stripping the source prefix before matching;
      (b) give directory patterns rsync-like subtree semantics and/or
      add a real `--exclude-dir`; (c) at minimum, warn when a pattern is
      structurally unmatchable (absolute but not under the source, or
      literal with no possible relative/filename match) instead of
      silently transferring everything. Whatever is chosen must apply
      uniformly across local-mirror, push, pull, and remote-remote (all
      route through the one `FileFilter`/`FilterInputs` chokepoint,
      `cli.rs:288-291`) and ship `--help`/manpage/README updates in the
      same change (docs-after-behavior rule). Distinct from `audit-17`
      (destination-fs charset rejection) — that crash is only a
      *symptom* here; the exclude no-op is the reported bug.
- [ ] **CLI transfer output redesign** (owner, 2026-07-06; re-confirmed
      2026-07-09): current `blit copy`/`mirror` output "doesn't convey any
      useful information at all" — owner wants something closer to
      `rclone`/`cargo`: "a coherent info block with stats and a scrolling
      list of files in a frame below, so probably a TUI?" (owner wording,
      2026-07-09) — i.e. a persistent stat block at a static screen
      location, plus a scrolling list of in-flight/recent filenames,
      instead of what exists today. 2026-07-09 context: the owner hit this
      while settling otp-7's error-surfacing question — "the current
      progress display is absolutely useless for this". The narrow
      end-of-operation fault summary (name failed files, suggest re-run)
      ships with otp-7 (D-2026-07-09-1) and is NOT gated on this redesign.
      Confirmed by reading the actual code — there is no persistent/redraw
      rendering anywhere in the transfer output path, only plain
      scrolling `println!`/`eprintln!` lines: (1) the local/streaming-manifest
      path's spinner + `"Enumerated N entries… (streaming manifest)"`
      heartbeat (`crates/blit-core/src/remote/push/client/helpers.rs:176`,
      the same call site audit-16 already flagged for its own separate
      `--verbose`-gating bug); (2) the remote-transfer progress path's
      once-a-second `"[progress] N/M files • X MiB copied • Y MiB/s avg •
      Z MiB/s current"` line (`crates/blit-cli/src/transfers/remote.rs:33-140`,
      `spawn_progress_monitor_with_options`), which just reprints a new
      line every tick rather than redrawing in place. Neither path shows
      a file list, a static stat panel, or does any cursor
      repositioning — every line is transient and scrolls off, which
      matches the owner's complaint exactly. This is a real UX/design
      project, not a bug fix: likely needs a terminal-rendering approach
      (raw ANSI cursor save/restore, or a crate like `indicatif`), has to
      cover both the local and remote transfer paths above, has to decide
      a fallback for non-TTY/`--json`/piped output (today's plain-line
      output is presumably what scripts already parse — a redesign must
      not break `--json` consumers), and touches `blit-cli`+`blit-app`.
      Not designed here — needs its own `plan` before any code, per this
      repo's governance (code changes require an approved plan) and the
      Review policy (D-2026-07-04-1, all code through the codex loop).
      Distinct from `docs/plan/TUI_REWORK.md` (Active), which is about
      the separate interactive `blit-tui` navigation app, not this
      inline CLI progress output during a transfer.

### Deferred design calls

These are intentionally not next-actionable. Don't pick them up
without the listed prerequisite — they're tracked here so they
don't get lost, not so the next agent reimplements them on a hunch.

- [ ] **Mac↔Mac Thunderbolt Bridge ceiling/control experiment** — optional
      follow-up after the required ldt-4 `q`↔`netwatch-01` evidence; it is
      not a substitute for Mac↔Windows acceptance. macOS supports direct IP
      over Thunderbolt Bridge, making this a useful same-OS control and a way
      to measure Blit's engine/CPU ceiling above 10 GbE while removing Windows,
      NTFS, the Ethernet NICs, and the switch as variables. First prove the
      certified cable/link and exact Thunderbolt-interface routing, then record
      bidirectional single- and multi-stream `iperf3` baselines before running
      Blit. Treat Thunderbolt 4's advertised 40 Gb/s as a link rate, not expected
      TCP payload throughput. Use a sustained workload of tens of GiB (or an
      equivalent controlled long-running sink): the existing short fixtures
      may finish before the live controller has enough samples to ADD or REMOVE.
      Compare initiator layouts within the same physical byte direction and
      exact source/destination paths. Any implementation or formal benchmark
      matrix needs its own approved plan. References: [Apple IP over Thunderbolt
      setup](https://support.apple.com/guide/mac-help/mchld53dd2f5/mac),
      [Intel Thunderbolt 4 data-bandwidth summary](https://cdrdv2-public.intel.com/755295/Thunderbolt_4_One-pager-002-210119.pdf).
- [x] **Remote→remote re-evaluation** — resolved by
  `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Phase 1 (`15991ed`)
  added the `DelegatedPull` wire/gate/daemon path; Phase 2 makes
  remote→remote CLI transfers direct by default, keeps
  `--relay-via-cli` as the explicit escape hatch, and pins byte-path
  isolation plus stale-dst/gate/source-refusal no-fallback behavior
  with CLI-side counters.

## Phase 0: Workspace & Core Logic Foundation

- [x] Initialize Cargo workspace with `blit-core`, `blit-cli`, `blit-daemon`, `blit-utils`.
- [x] Port `checksum.rs` to `blit-core`.
- [x] Port `fs_enum.rs` and `enumeration.rs` to `blit-core`.
- [x] Port `mirror_planner.rs` to `blit-core`.
- [x] Port `buffer.rs` to `blit-core`.
- [x] Extract zero-copy primitives into `blit-core/src/zero_copy.rs`.
- [x] Port unit tests for ported modules.

## Phase 1: gRPC API & Service Scaffolding

- [x] Create `proto/blit.proto` with the full API definition.
- [x] Create `build.rs` in `blit-core` to compile the protocol.
- [x] Add `tonic-build` dependencies to `blit-core/Cargo.toml`.
- [x] Create `generated` module structure in `blit-core`.
- [x] Implement skeleton `blitd` server binary in `blit-daemon`.
- [x] Implement skeleton `blit` CLI binary in `blit-cli`.
- [x] Add a minimal integration test to verify client-server connection.

## Phase 2: Orchestrator & Local Operations

- [x] Create `orchestrator.rs` in `blit-core`.
- [x] Implement the `TransferOrchestrator` struct and `new` method.
- [x] Implement `execute_local_mirror` method on the orchestrator.
- [x] Port consolidated path modules (`copy`, `tar_stream`, `transfer_*`, `local_worker`, `logger`, `delete`, `win_fs`) from v1 into `blit-core`.
- [x] Wire the `blit-cli` `mirror` and `copy` commands to the orchestrator.
- [x] Refactor `TransferFacade` and planner into streaming producer with heartbeat flushes.
- [x] Implement 10 s stall detection and progress messaging in orchestrator.
- [x] Implement fast-path routing for tiny/huge manifests in orchestrator.
- [x] Add adaptive predictor + local performance history store with `blit diagnostics perf`.
- [x] Remove `--ludicrous-speed` behaviour (make no-op) and add CLI progress UI.
- [x] Update unit/integration tests to cover fast-path routing and predictor logic.

## Phase 2.5: Performance & Validation Checkpoint

- [x] Create benchmark script for local mirror performance.
- [x] Run and compare against v1. (2025-10-16: v2 ~1.93× slower; optimization needed before GO)
- [x] Analyse Windows ETW traces (wingpt-4/5.md findings logged) (`logs/blit_windows_bench.zip`) and propose copy-path optimisations.
- [x] Re-run Windows benchmark after CopyFileExW fix (512 MiB) and update docs.
- [x] Prototype large-file heuristics (>1 GiB) and rerun 1–4 GiB suites.
  - 2025-10-19: Heuristic tuned (≤512 MiB cached, 2 GiB floor). wingpt-10.md confirms 512 MiB regression fixed and 4 GiB now beats robocopy.
- [x] Refactor oversized modules (`crates/blit-core/src/copy/`, `crates/blit-core/src/orchestrator/`) into focused submodules before Phase 3 to keep AI edits manageable.
- [x] Produce CLI/manpage documentation (include debug limiter behaviour, diagnostics commands, and hybrid transport flags once available). *(2025-10-19: `docs/cli/blit.1.md` added; hybrid transport flags pending Phase 3.)*
- [x] Extend proto (`proto/blit.proto`) with DataTransferNegotiation + reserved RDMA fields and transport stats ahead of Phase 3. *(2025-10-19: control-plane negotiation message plus push summary stats in place.)*
- [x] Document CLI debug limiter mode (`--workers`) in help text and plan docs. *(2025-10-19: CLI man page + plan updates.)*

## Phase 3: Remote Operations & Admin Tooling

- [x] Hybrid transport control/data plane scaffolding (push/pull) – initial implementation complete.
- [x] Remote pull integration tests (directory + single-file, forced gRPC path, traversal errors).
- [x] Realign CLI verbs (`copy`, `mirror`, `move`, `scan`, `list`) and remove legacy `push`/`pull`. *(2025-10-23: CLI now routes remote copy/mirror via RemotePush/RemotePull; docs/manpages updated.)*
- [x] Update canonical remote URL parser to support `server:/module/...` and `server://...` syntax. *(Parser already handled the forms; CLI + remote push now accept module sub-paths.)*
- [x] Implement daemon TOML config loader (modules, root export, mDNS flags) with warnings for implicit working-directory exports. *(2025-10-20: `blit-daemon` loads `/etc/blit/config.toml`/`--config`, supports `--root`, `--bind`, `--port`, `--no-mdns`, `--mdns-name`, and warns on implicit working-directory exports.)*
- [x] Investigate small-file performance (100 k × 4 KiB); target ≥95 % of rsync baseline. *(2025-10-21: blit 2.90 s vs tuned rsync 8.56 s on Linux; macOS 10.53 s vs rsync 11.62 s; Windows 60.63 s vs robocopy 218.48 s.)*
- [x] Investigate mixed workload (512 MiB + 50 k × 2 KiB); target ≥95 % of rsync baseline. *(2025-10-22: Linux blit 2.24 s vs rsync 6.95 s; macOS 6.32 s vs 6.56 s; Windows 31.26 s vs robocopy 110.51 s.)*
- [x] Improve incremental mirror throughput (touch 2 k/delete 1 k/add 1 k); target ≥95 % of rsync baseline. *(2025-10-22: Linux baseline 0.86 s vs rsync 1.32 s, mutation 0.61 s vs 1.23 s; macOS 0.65 s vs 0.69 s; Windows 7.10 s baseline and 6.45 s mutation vs robocopy 20.72 s/6.94 s.)*
- [x] Implement filesystem journal-based change detection on Windows (USN) to avoid full re-enumeration on no-op incremental runs; re-benchmark 0-change mutation once implemented.
- [x] Re-run Windows incremental 0-change benchmark to capture USN fast-path results (<200 ms target) and log findings in `DEVLOG`. *(2025-10-25: wingpt-53.md logged 28 ms zero-change mirror after USN fast-path fix.)*
- [x] Implement filesystem journal-based change detection on macOS (FSEvents) to avoid full re-enumeration on no-op incremental runs. *(2025-10-25: `change_journal` captures FSEvents snapshot; verify via `scripts/macos/run-journal-fastpath.sh` once mac agent available.)*
- [x] Implement filesystem journal-based change detection on Linux (metadata snapshot) to avoid full re-enumeration on no-op incremental runs. *(2025-10-25: `LinuxSnapshot` tracks device/inode/ctime; verified via `scripts/linux/run-journal-fastpath.sh` with 3 ms zero-change run.)*
- [x] Enable mDNS advertising by default with opt-out flag; update `blit scan` to consume results. *(2025-10-23: `blit-daemon` advertises `_blit._tcp.local.` unless `--no-mdns`; `blit scan`/`blit-utils scan` list discovered daemons.)*
- [x] Implement admin RPCs (module list, directory list, recursive find, du/df metrics, remote remove). *(2025-10-24: `Find`, `DiskUsage`, `FilesystemStats`, and enhanced `CompletePath` wired through daemon + proto.)*
- [x] Finish `blit-utils` admin surface: implement `find`, `du`, `df`, and `completions` (scan/list/ls/profile/rm implemented; rm wired to Purge on 2025-10-23). *(2025-10-24: new subcommands stream find/du/df results, completions delegates to daemon.)*
- [x] Wire remote mirror execution to the Purge RPC so remote mirrors delete extraneous files using the daemon. *(2025-10-23: `handle_push_stream` reuses purge helpers to remove remote extras and reports `entries_deleted` in summary.)*
- [x] Support remote-to-local mirrors (pull + local purge) so `blit mirror skippy://module dest/` downloads files and removes stray local state. *(2025-11-06: CLI routes remote mirrors through pull path, `RemotePullClient` tracks downloaded paths, and extraneous local entries are purged.)*
- [x] Investigate edge-case filesystem mirror gaps (ReFS clone path delivered but still ~35% slower than robocopy; follow-up benchmark `logs/windows/bench_local_windows_4gb_clone_20251026T020337Z.log`; ZFS baseline logged at `logs/truenas/bench_local_zfs_20251026T004021Z.log`). *(2025-10-26: Captured ReFS ETW profile `logs/windows/refs_clone_profile_20251026T022401Z.etl` plus bench log `bench_local_windows_4gb_clone_profile_20251026T022401Z.log`; manual `blit mirror --workers 1` runs (~0.28 s) show remaining ~1.7× gap vs robocopy 0.17 s is not purely task fan-out.)*
- [x] Update benchmark harnesses to honour `BLIT_BENCH_ROOT` (or similar) so Windows runs stay on the intended filesystem; document TEMP/TMP requirement in workflow docs. *(2025-10-26: scripts respect BLIT_BENCH_ROOT; workflow tip added.)*
- [x] Prototype clone-only metadata fast path on Windows (skip redundant attribute sync + reduce IOCTL overhead) and compare against robocopy using ETW traces (`logs/windows/refs_clone_profile_20251026T022401Z.etl`). *(2025-11-10: `copy_file` now returns `FileCopyOutcome`, skips metadata + timestamp preservation when block clone succeeds, and instructs operators to validate with the existing ETW trace + bench harness.)*
- [x] Implement streaming manifest/need-list for remote push so arbitrarily large file sets do not exhaust RAM (see memory `manifest_streaming_plan`). *(2025-10-26: CLI streams manifests over mpsc channel with back-pressure; daemon batches need list incrementally; control-plane fallback reads files in 1 MiB chunks.)*
- [x] Bundle remote transfers (TCP data plane + gRPC fallback) into tar shards so small files aren't shipped one-by-one. *(2025-10-27: Extended proto with `TarShard{Header,Chunk,Complete}`; CLI plans shard batches, TCP data plane streams them with new record types, daemon unpacks incrementally; `remote_tcp_fallback` test now time-bounded.)*
- [x] Parallelize daemon tar-shard unpacking so the data plane doesn't stall on decode. *(2025-10-28: `TarShardExecutor` spawns up to 4 blocking workers via `JoinSet`, updating stats as shards complete; ready for throughput benchmarking on `skippy`/`mycroft`.)*
- [x] Add large-manifest stress test (≥1 M entries) to validate streaming push memory footprint, <1 s transfer start, and throughput; capture logs/metrics with CLI/daemon traces. *(2025-11-07: Added ignored `drain_pending_headers_handles_one_million_entries` test; run manually with `cargo test -p blit-core drain_pending_headers_handles_one_million_entries -- --ignored` to collect RSS data.)*
- [ ] Benchmark remote fallback + data-plane streaming on Linux/macOS/Windows to confirm sub-second first-byte timings and document results in workflows.
- [x] Ensure destructive operations prompt unless `--yes` is supplied. *(2025-01-21: `blit mirror` prompts before deleting extraneous destination files; `blit move` prompts before deleting source after transfer; `--yes`/`-y` bypasses; `blit rm` already had prompts.)*
- [x] Document that remote transfers rely on operator-provided secure networks or SSH tunnels (no built-in TLS); update CLI/daemon help text and plan docs accordingly. *(2025-01-21: Added SECURITY sections to both `blit.1.md` and `blit-daemon.1.md` documenting TLS absence and recommended secure deployment patterns: SSH tunnel, VPN, trusted network, reverse proxy.)*
- [ ] **P0** Remote transfer parity refactor (see `docs/plan/REMOTE_TRANSFER_PARITY.md`):
    - [x] Extract shared modules `remote::transfer::{payload, progress, data_plane}` and migrate push to use them, wiring the common planner through remote push. *(2025-11-10: Added `remote::transfer` with shared payload/progress/data-plane logic and refactored push to consume it; pull wiring + auto-tune hookup still pending.)*
    - [x] Extend `PullChunk` proto with negotiation + summary messages; regenerate bindings. *(2025-11-10: Added negotiation/summary variants; regenerated prost bindings.)*
    - [x] Rebuild daemon pull pipeline to reuse hybrid transport + TCP data plane (with `--force-grpc` fallback) and enlarge data-plane buffers / zero-copy paths to match v1’s 10 GbE throughput. *(2025-11-10: `service/pull.rs` now enumerates manifests, plans payloads via `remote::transfer`, and streams via TCP listener + negotiation, falling back to gRPC when forced.)*
    - [x] Rewrite CLI / `RemotePullClient` to use the shared transport, emit progress (`-p/-v`), and connect to the data plane with the auto-tuned scheduler. *(2025-11-10: `RemotePullClient` connects to the negotiated TCP stream, writes files/tar shards to the destination, records summary info, and the CLI now reuses the shared progress monitor for push/pull.)*
    - [x] Feed `auto_tune::determine_tuning` outputs into remote push/pull schedulers so stream counts and chunk sizes adapt automatically. *(2025-11-10: `RemotePushClient` now applies the tuned chunk size across both TCP and gRPC fallback paths, drives data-plane payload prefetching via the tuned stream counts, and the daemon pull path reuses those parameters when sharding + streaming payloads.)*
    - [x] Ensure manifest need-lists flush immediately so first payloads start within seconds even on multi-hundred-thousand file manifests, and surface unreadable files inline while keeping the TCP data plane active. *(2025-11-10: daemon `FileListBatcher` gained an early flush path; CLI push now logs permission/not-found entries in red and filters them before planning.)*
    - [x] Implement multiplexed TCP data-plane streams (client + daemon) driven by auto-tuned worker counts so push/pull saturate 10 GbE. *(2025-11-10: push/pull negotiations carry `stream_count`; daemon and client spawn parallel TCP workers with auto-tuned stream counts.)*
    - [x] Implement multiplexed TCP data-plane streams (client + daemon) driven by auto-tuned worker counts so push/pull saturate 10 GbE. *(2025-11-10: push/pull negotiations carry `stream_count`; daemon and client spawn parallel TCP workers with auto-tuned stream counts.)*
    - [x] Balance TCP data-plane payload scheduling across streams so negotiated workers actually receive work; `MultiStreamSender` now slices plans into 32–512 MiB batches per stream instead of routing entire manifests through a single connection. *(2025-11-15: eliminates the ~450 Mbps cap observed on push tests and unlocks true multi-stream throughput.)*
    - [x] Revert hardcoded performance optimizations and implement orchestrator-controlled configuration for TCP settings (Nagle's algorithm, buffer sizes), chunk sizes, stream counts, and payload prefetching. *(2025-11-20: Refactored `TuningParams` to include `tcp_buffer_size` and `prefetch_count`, populated by heuristics, and passed down to data plane.)*
    - [x] Add integration/perf tests proving push/pull parity (TCP + forced gRPC) and document the results. *(2025-01-29: Tests exist in `remote_parity.rs` (push/pull TCP negotiation, gRPC fallback), `remote_resume.rs` (block-level resume for partial files, identical files, gRPC fallback). All tests pass on Unix; Windows tests deferred to CI.)*
- [ ] **P0** 25GbE performance improvements:
    - [x] Implement `BufferPool` with reusable allocations, semaphore memory control, and RAII guards (no hardcoded defaults; accepts orchestrator params). *(2025-01-26: Added to `buffer.rs` with tests.)*
    - [x] Integrate `BufferPool` with `DataPlaneSession` using `TuningParams` (pass pool via orchestrator, replace `vec![0u8; buffer_len]` allocation). *(2025-01-26: Added `SessionBuffer` enum, `connect_with_pool`, `MultiStreamSender` creates shared pool from `chunk_bytes`/`stream_count`.)*
    - [x] Implement async read-ahead pipeline in `DataPlaneSession::send_file` (overlap disk reads with network writes using double-buffering from pool). *(2025-01-26: Added `send_file_double_buffered` using `tokio::join!` with two pool buffers, falls back to single-buffer when no pool.)*
    - [x] Parallel payload dispatch across TCP streams (concurrent stream workers). *(2025-01-26: Already implemented via `MultiStreamSender` - spawns N workers as concurrent tokio tasks, round-robin batch distribution 32-512 MiB. Work-stealing queue is potential future optimization.)*
    - [ ] Benchmark TCP data plane throughput targeting 10+ Gbps per stream.
    - [x] Add remote↔remote transfers (CLI + daemon support for server-to-server sync initiated from a third host) so every src/dst combination is covered. *(2025-01-21: Implementation via `RemoteTransferSource` abstraction complete; integration test `remote_remote.rs` covers dual-daemon server-to-server copy; CLI supports `blit copy/mirror server1:/mod/ server2:/mod/` syntax.)*
- [x] Diagnose TCP data-plane resets during remote push (upload_tx channel closes while streaming tar shards; see `crates/blit-daemon/src/service/push/data_plane.rs`). Reproduce with `blit-cli mirror -v -p ~/ skippy://elphaba/home`, capture daemon `[data-plane]` logs, and fix underlying disk/write/mismatch issue so the connection no longer drops mid-transfer. *(2025-11-06: Hardened client TCP sender + restored streamed-file metadata; added CLI trace flag and confirmed `source/venvs/superclaude` run completes without resets.)*
- [x] Refactor oversized sources into AI-manageable modules:
    - [x] Split `crates/blit-daemon/src/main.rs` (service wiring, data plane handlers, admin RPCs). *(2025-10-27: introduced `runtime.rs` for config/args and `service.rs` for gRPC/data plane; main now only boots the server.)*
    - [x] Break down `crates/blit-cli/src/main.rs` (argument parsing vs command execution). *(2025-10-27: extracted `cli.rs`, `context.rs`, `diagnostics.rs`, `scan.rs`, `list.rs`, and `transfers.rs`; main now wires modules only.)*
    - [x] Decompose `crates/blit-core/src/copy/` (move platform-specific helpers into submodules). *(2025-10-27: split into `compare.rs`, `file_copy.rs`, `parallel.rs`, `stats.rs`, keeping platform helpers isolated; `mod.rs` now re-exports public API.)*
    - [x] Split `crates/blit-utils/src/main.rs` (verb dispatch vs helpers). *(2025-10-27: introduced `cli.rs`, `util.rs`, and verb modules `scan/list_modules/ls/find/du/df/completions/rm/profile`; main now dispatches only.)*
    - [x] Extract helpers from `crates/blit-core/src/change_journal.rs`, `transfer_facade.rs`, and `remote/push/client.rs` below 500 lines. *(Completed via sub-items below.)*
    - [x] `change_journal` split into `types/snapshot/tracker/util` (2025-10-28).
    - [x] `transfer_facade` modularised into `types/aggregator/planner` (2025-10-28).
    - [x] `remote/push/client.rs` reorganised into `client/{mod,types,helpers}` with spawn helpers (2025-10-28).
    - [x] Break `crates/blit-cli/src/transfers.rs` into module directory (`mod.rs` + `endpoints`, `remote`, `local`, `mmap`) to keep files under 400 LOC (2025-10-28).
    - [x] Split `crates/blit-core/src/orchestrator/mod.rs` into `options.rs`, `summary.rs`, and `orchestrator.rs` alongside existing helpers (2025-10-28).
    - [x] Restructure `crates/blit-core/src/copy/file_copy.rs` into submodules (`clone`, `metadata`, `mmap`, `chunked`) so each stays <300 LOC (2025-10-28).
- [x] Wire remote `copy`/`mirror`/`move` to hybrid transport with automatic gRPC fallback. *(2025-10-25: integration test `remote_tcp_fallback` forces `--force-grpc-data` and verifies CLI output + successful transfer.)*
- [x] Add integration tests covering remote transfer + admin verbs across Linux/macOS/Windows. *(2026-03-06: Added 16 tests: ListModules, CompletePath, subdirectory list, find with pattern, recursive rm, nested push/pull, copy-vs-mirror semantics, tar shard batching, empty directory handling.)*
- [x] Investigate why local→local mirror is slower than local→remote mirror; profile orchestrator overhead, sync I/O patterns, and compare code paths. *(2026-04-07: Root cause was divergent code paths — local used separate LocalWorkerFactory pipeline, remote used TransferSource→plan→DataPlaneSession. Fix: introduced TransferSink trait with pluggable write backends (FsTransferSink for local, DataPlaneSink for TCP). All transfers now flow through unified pipeline: TransferSource→plan_transfer_payloads→execute_sink_pipeline→TransferSink. Removed 384 lines of dead code (old worker factory, planner driver). 6 new tests.)*
- [ ] Capture remote benchmark runs (TCP vs forced gRPC fallback) and log results.
- [x] Design adaptive predictor regression test suite (parsing, coefficient updates, accuracy, runtime overhead); automate as part of CI. *(2026-03-06: 9 regression tests covering convergence, coefficient clamping, profile isolation, save/load round-trip, and scaling behavior.)*
- [x] Implement performance history schema versioning/migration to handle future format changes without data loss. *(2026-03-06: Added schema_version field to PerformanceRecord with serde(default), migrate_record(), migrate_history_file(), version stamped on write, auto-migrated on read. 7 unit tests.)*

## Phase 4: Production Hardening & Packaging

- [x] **P1** Integrate resumable file copy into transfer flow:
    - [x] Implement `resume_copy_file` with block-level comparison (`copy/file_copy/resume.rs`). *(2025-01-28: Added with 5 unit tests.)*
    - [x] Add `resume: bool` field to `CopyConfig`. *(2025-01-28)*
    - [x] Modify `local_worker.rs` to use `resume_copy_file` when `config.resume` is true. *(2025-01-28)*
    - [x] Add `--resume` flag to CLI for copy/mirror commands. *(2025-01-28)*
    - [x] Extend resume logic for remote transfers (gRPC path - block hash exchange). *(2025-01-28: Protocol extended with BlockHashRequest/BlockHashList/BlockTransfer/BlockTransferComplete; daemon requests block hashes for Modified files, compares Blake3 hashes, sends only differing blocks; client computes hashes and writes blocks at offset.)*
    - [x] Extend resume logic for remote data plane (primary path). *(2025-01-28: Added DATA_PLANE_RECORD_BLOCK and DATA_PLANE_RECORD_BLOCK_COMPLETE to TCP data plane. `stream_via_data_plane_resume` uses gRPC for block hash exchange, TCP data plane for block transfer. Client handles block records with seek+write. Works with default `--resume` flag.)*
    - [x] Fix memory vulnerability and protocol inefficiency (code review). *(2025-01-29: Fixed `compute_block_hashes` and daemon-side functions to stream files in chunks instead of loading into memory. Pipelined block hash requests in data plane path to eliminate per-file RTT penalty.)*
- [x] **P1** Implement filesystem capability probes and caching (daemon idle probes + CLI profile hook) so per-mount features like reflink/sparse/xattr are detected automatically and exposed to the planner. *(2026-03-06: Added `fs_capability/probe.rs` with statfs-based FS type detection on macOS/Linux, per-FS capability mapping for 12+ filesystem types, device-keyed cache, `cached_probe()` API. 7 tests.)*
- [x] **P1** blit-utils hardening:
    - [x] Fix `edition = "2024"` typo in `crates/blit-utils/Cargo.toml` (should be `"2021"`). *(2026-03-06: Fixed.)*
    - [x] Add integration tests for all 9 commands (scan, list-modules, ls, find, du, df, rm, completions, profile). *(2026-04-07: 21 tests in `blit_utils.rs` covering all commands with text/JSON output, filters, limits, error cases.)*
    - [x] Add human-readable byte formatting to `df` output (currently raw bytes only). *(2026-04-07: `df` now shows `format_bytes()` alongside raw bytes.)*
    - [x] Produce man page (`docs/cli/blit-utils.1.md`) mirroring blit-cli coverage. *(2026-03-06: Created with full synopsis, options, examples, and exit codes.)*
- [x] ~~Explore optional AI-powered telemetry analysis~~ — **Removed from scope 2026-05-13** (`RELEASE_PLAN_v2_2026-05-04.md` §5.4, owner decision). Scoping doc at `docs/plan/AI_TELEMETRY_ANALYSIS.md` deleted. Performance history collection continues for the predictor (§2.8); no "analyze my history" feature is planned.
- [x] Produce packaging artifacts for supported platforms (Linux, macOS, Windows). *(2026-04-07: Added `scripts/build-release.sh` (Unix) and `scripts/windows/build-release.ps1` with tarball creation. Added `.github/workflows/ci.yml` with check/fmt/clippy, tri-platform tests, and release artifact uploads.)*
- [x] Document installation/configuration (config.toml, `--root`, mDNS, service setup). *(2026-04-07: Rewrote `docs/DAEMON_CONFIG.md` with accurate TOML format, correct port 9031, `[[module]]` syntax, client config directories, mDNS section, and expanded service installation for Linux/macOS/Windows.)*
- [x] Build end-to-end integration/regression suite and integrate with CI. *(2026-04-07: CI workflow runs `cargo test --workspace` on Linux/macOS/Windows. Integration tests cover admin verbs (10 tests), blit-utils (21 tests), remote transfers, transfer edges, parity, resume, and move.)*
- [x] Review logging/error output for production readiness. *(2026-04-07: Audited all crates. Removed duplicate println/eprintln debug output from block clone paths. Added expect() messages to bare unwrap() calls. No dbg!()/todo!() found. Remaining eprintln calls in orchestrator/daemon are intentional verbose output. Full migration to structured logging deferred to post-release.)*
- [x] Prepare release notes/changelog with benchmark data and support matrix. *(2026-04-07: Created `CHANGELOG.md` with full feature inventory, platform support matrix, known limitations. Benchmark data to be added when hardware runs complete.)*

## Phase 5: TUI / UI — ✅ SHIPPED (v0.1.0, 2026-05-31)

All six original milestones (A.0, B, M-Jobs, C, A.1, D, E) shipped on
the `phase5/a1` branch and merged to master at commit `85acd63`. The
v0.1.0 tag points at `b0ea588`. See `CHANGELOG.md` `[0.1.0] - 2026-05-31`
for the full surface inventory.

The original milestone list below is **retained as historical phasing
record** — all checkboxes are now closed.

- [x] **Phase 5 — Milestone A.0.** Extract `crates/blit-app` library from `blit-cli`. ✅ Shipped.
- [x] **Phase 5 — Milestone B.** `GetState` RPC + always-on `ActiveJobs` table + `recent` ring. ✅ Shipped.
- [x] **Phase 5 — Milestone M-Jobs.** Daemon-owned transfer lifecycle for remote→remote. ✅ Shipped.
- [x] **Phase 5 — Milestone C.** `Subscribe` RPC + `DaemonEvent` family + byte-level progress instrumentation. ✅ Shipped.
- [x] **Phase 5 — Milestone A.1.** The TUI itself. ✅ Shipped.
- [x] **Phase 5 — Milestone D.** F4 Verify + diagnostics dump action. ✅ Shipped.
- [x] **Phase 5 — Milestone E.** Polish: theme, configurable refresh, key remapping, optional Prometheus bridge. ✅ Shipped.

## Phase 6: TUI Rework — Pick-not-Type (active plan post-v0.1.0)

> **The active TUI plan is now `docs/plan/TUI_REWORK.md`**, not
> `TUI_DESIGN.md`. The latter shipped as v0.1.0 and is the baseline;
> the rework addresses a structural UX problem surfaced by walking
> through the four cardinal transfer workflows on the shipped TUI:
> every workflow asks the operator to type at least one path that
> isn't on screen.

The rework is six milestones (M1 → M6), each independently shippable
on a `phase6/tui-rework` branch from master. M1 (F3 picker mode) is
the first slice; M2 (LocalDaemon) unblocks Local↔Local in the TUI for
the first time; M3a+M3b together eliminate every free-text path field
in normal flows; M4 adds multi-daemon fan-out; M5 pre-fills source
from current selection; M6 is the type-it-anyway power-user hatch.

All six "open decisions" in the original rework spec were resolved
2026-05-31 by concurring reviewer sign-off (see
[`docs/plan/TUI_REWORK.md`](./docs/plan/TUI_REWORK.md) §6 for the
locked-in choices). Agents queuing TUI work should **start from the
rework spec, not from `TUI_DESIGN.md`** — the latter has a top-of-file
note redirecting to the rework.

- [ ] **Phase 6 — M1.** F3 picker mode (foundation; modal pick-mode + continuation channel).
- [ ] **Phase 6 — M2.** `LocalDaemon` pseudo-target + local-fs browsable in F3.
- [ ] **Phase 6 — M3a.** F1 trigger modal: pickers replace text fields.
- [ ] **Phase 6 — M3b.** F3 pull destination: picker replaces free-text input.
- [ ] **Phase 6 — M5.** Source pre-fill from current F3 / Local selection.
- [ ] **Phase 6 — M4.** Multi-daemon Space-mark + batch trigger.
- [ ] **Phase 6 — M6.** Polish: `:` literal-path escape hatch inside the picker.

**Decisions taken** (`TUI_DESIGN.md` §10): separate `blit-app` library + `blit-tui` binary; local-only TUI mode first-class with "local" as a sentinel endpoint in F1; foundation-first milestone order; cancellation via server-side `CancelJob`; `--detach` CLI flag ships with M-Jobs; `AppProgressEvent` is channel-based.

## Phase 3.5: RDMA Enablement (post-release)

- [ ] Track deferred RDMA/RoCE work (control-plane negotiation, transport abstraction, benchmarking) for future planning.

- [ ] Investigate SeManageVolumePrivilege requirement for ReFS block clone on dev machine; backups showing CopyFileEx fallback (~0.6 s). Need elevated shell or alternative clone mechanism to validate fast path.
