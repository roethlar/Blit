# Drift Findings: CLI surface (manpage, README, CHANGELOG)
**Generated**: 2026-06-04
**Claims audited**: ~80 (verb shapes, flags, behaviors, principles, shipped/deferred items)
**Findings**: 14 (H: 2 / M: 8 / L: 4)

## High severity

### manpage-omits-shipped-verbs-jobs-check — Two shipped top-level verbs (`jobs`, `check`) are absent from manpage and CHANGELOG
**Plan says**: blit.1.md `SYNOPSIS` enumerates the user-facing verbs (`copy`, `mirror`, `move`, `scan`, `list`, `list-modules`, `ls`, `du`, `df`, `rm`, `find`, `completions`, `profile`, `diagnostics perf`, `diagnostics dump`) — no `jobs`, no `check`. CHANGELOG.md:26 enumerates "Commands: `copy`, `mirror`, `move`, `scan`, `list`, `du`, `df`, `rm`, `find`" — also omits `jobs` and `check`. Both docs hold themselves out as the authoritative user-facing surface.
**Code does**: `crates/blit-cli/src/cli.rs:76` defines `Check(CheckArgs)` ("Compare two trees by size+mtime or hash"). `cli.rs:82-86` defines `Jobs { command: JobsCommand }` with `List`, `Cancel`, `Watch` subcommands. Both are wired in `main.rs:76` (`Commands::Check` → run_check with semantic exit codes) and `main.rs:86` (`Commands::Jobs` → run_jobs with semantic exit codes). `blit check` and `blit jobs list|cancel|watch` are real, advertised features with semantic exit codes (0/1/2 and 0/1/2/3 respectively), held up by reviewer-tracked work-items (§6.5 of TUI_DESIGN cited in main.rs:82).
**Evidence**:
- `crates/blit-cli/src/cli.rs:76` — `Check(CheckArgs)`
- `crates/blit-cli/src/cli.rs:82-86` — `Jobs { command: JobsCommand }`
- `crates/blit-cli/src/main.rs:76,86` — exit-code propagation paths
- `docs/cli/blit.1.md:8-24` — SYNOPSIS without `jobs`/`check`
- `CHANGELOG.md:26` — "Commands:" list without them
**Notes**: Both verbs ship with detach/watch flows that users are expected to operate; `--detach`'s very help text in `cli.rs:317-326` directs users to `blit jobs cancel` and `blit jobs list` — yet neither is documented. This is silent feature drift: shipped surface > documented surface, and the docs cannot help users discover or script the most operationally important verb (`jobs watch` for completion polling). Remediate: add `SYNOPSIS` entries, OPTIONS subsections, and a brief CHANGELOG line.

### manpage-omits-data-loss-relevant-flags — Eight transfer flags including data-loss-class `--null`, `--detach`, `--delete-scope`, `--force`, `--ignore-existing` are absent from manpage
**Plan says**: blit.1.md `OPTIONS` "Transfer Options" enumerates the documented flag surface: `--dry-run`, `--checksum`, `--resume`, `--verbose`, `--progress`, `--force-grpc`, `--relay-via-cli`, `--yes`. Verbatim from manpage:103-132. CHANGELOG.md:29 enumerates "`--dry-run`, `--checksum`, `--force-grpc`, `--workers`". Both purport to describe the live user surface.
**Code does**: `crates/blit-cli/src/cli.rs:188-364` defines `TransferArgs` with: `--null` (data-loss-class, plan §"--null + local-source push silently safe"), `--detach` (data-loss-class for move; see lines 314-335), `--delete-scope` (data-loss-class `all` vs `subset` default; line 247), `--force` (line 240, marked "dangerous"), `--ignore-existing` (line 237), `--size-only`, `--ignore-times`, `--exclude`/`--include`/`--files-from`/`--min-size`/`--max-size`/`--min-age`/`--max-age` (filtering; lines 279-299), `--retry`/`--wait` (reliability; lines 264-272), `--json` (transfer summary; line 223). The manpage documents none of these. The reject-gates in `transfers/mod.rs:131-406` explicitly call these out as data-loss-critical for `move` — so users hitting the gate get an error referring to flags the manpage doesn't mention.
**Evidence**:
- `crates/blit-cli/src/cli.rs:204-364` — full TransferArgs flag inventory
- `crates/blit-cli/src/transfers/mod.rs:131-154` — `--null` reject gates citing data-loss
- `crates/blit-cli/src/transfers/mod.rs:255-269` — `--detach` reject gate (move)
- `crates/blit-cli/src/transfers/mod.rs:280-296` — filter reject gates (move)
- `crates/blit-cli/src/transfers/mod.rs:306-314,323-331` — `--ignore-existing`, `--null` move-gates
- `docs/cli/blit.1.md:102-132` — manpage's complete Transfer Options coverage
- `CHANGELOG.md:29` — CHANGELOG's complete CLI options coverage
**Notes**: This is the highest-impact doc drift in the cluster: flags exist in shipped builds, have dedicated CLI-layer guard-rails to prevent data loss, are tested (R47/R49/R51/R52/R53/R54 prefixes in source comments), and are user-discoverable via `--help` — but the canonical reference docs say none of them exist. A user reading the manpage will not know `--null`, `--exclude`, or `--delete-scope all` are supported, and won't know that `move --exclude` is explicitly forbidden. Remediate: regenerate manpage from clap definition or hand-add an "Advanced Transfer Options" subsection.

## Medium severity

### manpage-stale-publication-date — Manpage dated 2025-11-21, predates v0.1.0 changelog (2026-05-31) by ~6 months
**Plan says**: docs/cli/blit.1.md:3 header `% 2025-11-21`. CHANGELOG.md:5 records the documented release at `[0.1.0] - 2026-05-31`. The manpage is the SYNOPSIS-of-record for v0.1.0.
**Code does**: Between Nov 2025 and May 2026 the CLI gained at minimum: `jobs` (list/cancel/watch), `check`, `--null`, `--detach`, `--delete-scope`, `--retry`/`--wait`, full filter suite (`--exclude` etc.), `--json` for transfers, `--config-dir` global, `--mdns-name`, `--no-server-checksums`, `--metrics` daemon flags. The manpage was not refreshed to match. This is contradiction #7 of the plan inventory.
**Evidence**:
- `docs/cli/blit.1.md:3` — `% 2025-11-21`
- `CHANGELOG.md:5` — `## [0.1.0] - 2026-05-31`
- `crates/blit-cli/src/cli.rs:36-86,188-364` — diff vs documented surface
**Notes**: The date stamp signals to maintainers and ops teams "this is current" when it isn't. Remediate: bump the date stamp at the same time the SYNOPSIS is regenerated. Consider auto-generating the date via build script.

### changelog-claims-blit-utils-manpage-shipped — Plan claims `docs/cli/blit-utils.1.md` was created 2026-03-06, file does not exist
**Plan says**: docs/plan/BLIT_UTILS_PLAN.md:65 "Update documentation (CLI manpages, quick-start) once commands land. *Done — `docs/cli/blit-utils.1.md` created 2026-03-06.*"
**Code does**: `docs/cli/` contains only `blit-daemon.1.md` and `blit.1.md`. No `blit-utils.1.md`. (Verified via `ls /Users/michael/Dev/Blit/docs/cli/`.) The merge into `blit` (correctly noted in the plan's supersession banner) made that manpage redundant, but the checklist item still claims it was created and (implicitly) shipped.
**Evidence**:
- `docs/plan/BLIT_UTILS_PLAN.md:65` — claim
- `docs/cli/` directory contents — `blit-daemon.1.md`, `blit.1.md` only
- `docs/plan/BLIT_UTILS_PLAN.md:1-15` — supersession banner explicitly says "no `blit-utils(1)` manpage"
**Notes**: Internal contradiction in the same doc — line 8 says "no `blit-utils(1)` manpage", line 65 says it was created. Either historical wording should be cleaned up to read "rolled into blit.1.md" or the line should be deleted.

### manpage-omits-find-files-dirs-filters — `blit find --files` / `--dirs` flags absent from manpage SYNOPSIS and OPTIONS
**Plan says**: docs/cli/blit.1.md:19 SYNOPSIS lists `blit find [--pattern <GLOB>] [--case-insensitive] [--limit <N>] [--json] <REMOTE>`. OPTIONS Admin Options sections (lines 147-154) cover `--pattern`, `--case-insensitive`, `--limit`. No mention of `--files` or `--dirs`.
**Code does**: `crates/blit-cli/src/cli.rs:466-471` defines `--files` (Include only files) and `--dirs` (Include only directories) on `FindArgs`. They drive the filter logic at `crates/blit-cli/src/find.rs:20-29`. CHANGELOG.md:45 doesn't list them either.
**Evidence**:
- `crates/blit-cli/src/cli.rs:466-471` — flags defined
- `crates/blit-cli/src/find.rs:20-29` — flags consumed
- `docs/cli/blit.1.md:19,147-154` — undocumented
**Notes**: Same shape as the larger `manpage-omits-data-loss-relevant-flags` finding but worth flagging separately because `--files`/`--dirs` are commonly-used operational filters (the equivalent of `find … -type f` / `find … -type d`).

### options-section-omits-many-json-flags — `--json` documented for du/df only; SYNOPSIS lists it on 7 commands
**Plan says**: docs/cli/blit.1.md SYNOPSIS lines 12, 13, 14, 15, 16, 17, 19 use `[--json]` on `scan`, `list`, `list-modules`, `ls`, `du`, `df`, `find`. OPTIONS Admin Options:141-142 "`--json` (du, df) — Output results as JSON."
**Code does**: `--json` is implemented on all 7 verbs listed in SYNOPSIS (verified via `cli.rs:398,406,414,425,435,447,479`) plus `profile`, `diagnostics perf`, `diagnostics dump`, `rm`, `jobs list`, `jobs cancel`, `jobs watch`, and `copy`/`mirror`/`move`. The OPTIONS section has not been updated to reflect this. This is contradiction #3 in the plan inventory.
**Evidence**:
- `crates/blit-cli/src/cli.rs:223,398,406,414,425,435,447,479,532` — `pub json: bool` on every Args struct
- `docs/cli/blit.1.md:141-142` — OPTIONS limited to (du, df)
**Notes**: CHANGELOG.md:41 makes a stronger and accurate claim — "`--json` output for all inspection commands". The manpage OPTIONS prose is the outlier. Remediate: change OPTIONS line to "(all inspection commands)" or list them.

### options-section-omits-profile-limit — `--limit` documented for `find` only; SYNOPSIS lists it on `profile` and `diagnostics perf` too
**Plan says**: docs/cli/blit.1.md:22 SYNOPSIS lists `blit profile [--limit <N>] [--json]`. Line 23 lists `blit diagnostics perf [--limit <N>] [--enable|--disable] [--clear]`. OPTIONS Admin Options:153-154 only documents `--limit <N>` (find).
**Code does**: `crates/blit-cli/src/cli.rs:533-534` defines `ProfileArgs::limit` with `default_value_t = 50`. `crates/blit-cli/src/cli.rs:170-172` defines `PerfArgs::limit` (default 50). Both are real flags. The DIAGNOSTICS section (manpage:159) does document `--limit <N>` for `diagnostics perf`, but the profile-side `--limit` is undocumented anywhere outside the SYNOPSIS line. This is contradiction #4 in the plan inventory.
**Evidence**:
- `crates/blit-cli/src/cli.rs:170-172,533-534` — both `--limit` flags
- `docs/cli/blit.1.md:22,153-154,159` — SYNOPSIS lists profile/perf, OPTIONS only describes find
**Notes**: Low-stakes drift but a precedent for the broader "SYNOPSIS lists, OPTIONS doesn't describe" pattern across this doc.

### contradiction-grpc-role — README and CHANGELOG disagree on whether gRPC is "fallback for diagnostics" or "control plane"
**Plan says**: README.md:25 "TCP data plane for high-throughput transfers (10+ Gbps), with gRPC fallback for diagnostics." CHANGELOG.md:9 "Remote push/pull via hybrid TCP data plane + gRPC control plane". Manpage:28 "hybrid TCP/gRPC transport". Three docs, three different framings. This is contradiction #1 in the plan inventory.
**Code does**: gRPC carries the **control plane** (push/pull/admin RPCs) per `crates/blit-daemon/src/service/core.rs:845,916,938,973,1148,1198,1263` (admin RPCs on the gRPC server) and `crates/blit-cli/src/transfers/remote.rs` (push/pull control). TCP carries the data-plane payload by default; gRPC also serves as a data-plane fallback via `--force-grpc` (cli.rs:304). So CHANGELOG and manpage are accurate; README's "fallback for diagnostics" is wrong on two counts: gRPC is the primary control plane, AND gRPC-as-data-plane is selectable not just diagnostic.
**Evidence**:
- `README.md:25` — "fallback for diagnostics"
- `CHANGELOG.md:9` — "TCP data plane + gRPC control plane"
- `docs/cli/blit.1.md:28` — "hybrid TCP/gRPC transport"
- `crates/blit-daemon/src/service/core.rs:845-1263` — gRPC is the primary RPC surface
- `crates/blit-cli/src/cli.rs:304-305` — `--force-grpc` for data plane
**Notes**: README is the first-impression doc for new users; mischaracterizing gRPC's role here is the most user-visible of the three. Remediate: bring README in line with CHANGELOG's framing.

### list-smart-dispatch-described-as-alias — BLIT_UTILS_PLAN calls `list` an "alias of `ls`"; live behavior is smart-dispatch
**Plan says**: docs/plan/BLIT_UTILS_PLAN.md:30 "`blit-utils list <remote[:/module/path]>` Alias of `ls` (compatible with plan v6)."
**Code does**: `crates/blit-cli/src/cli.rs:60-62` defines `Ls(ListArgs)` with `#[command(alias = "list")]`. At dispatch time, `crates/blit-cli/src/ls.rs:49-59` routes `RemotePath::Discovery` (bare host) to `list_modules_remote` instead of treating it as a path-listing — a smart-dispatcher, not a pure alias. Manpage:80-82 documents the smart-dispatch behavior accurately; CHANGELOG.md:44 also. The BLIT_UTILS_PLAN row is stale per contradiction #6 in plan inventory.
**Evidence**:
- `crates/blit-cli/src/cli.rs:61` — `#[command(alias = "list")]`
- `crates/blit-cli/src/ls.rs:49-59` — smart-dispatch routing
- `docs/cli/blit.1.md:80-82` — manpage describes smart-dispatch
- `CHANGELOG.md:44` — CHANGELOG describes smart-dispatch
- `docs/plan/BLIT_UTILS_PLAN.md:30` — stale "alias of ls" line
**Notes**: BLIT_UTILS_PLAN is marked Superseded, so its drift is lower-impact than blit.1.md drift, but the description model the doc preserves no longer matches reality. The plan's own header even says "treat any reference to `blit-utils <verb>` as historical wording for `blit <verb>`" — so this row is mostly fine, but the *semantics* described are wrong, not just the binary name.

### completions-command-shape-restructured — BLIT_UTILS_PLAN documents single-form `completions <remote>`; shipped surface splits `completions shell` vs `completions remote`
**Plan says**: docs/plan/BLIT_UTILS_PLAN.md:35 "`blit-utils completions <remote[:/path]> [--prefix <fragment>]` Fetches remote path completions for interactive shells."
**Code does**: `crates/blit-cli/src/cli.rs:489-506` defines `CompletionKind` with two subcommands: `Shell(ShellCompletionArgs)` (static clap_complete script generation) and `Remote(RemoteCompletionArgs)` (daemon-backed). Shipped form is `blit completions shell <SHELL>` and `blit completions remote <REMOTE>`. This is contradiction #5 in the plan inventory; manpage (lines 90-96) and CHANGELOG (lines 46-47) describe the live two-subcommand shape.
**Evidence**:
- `crates/blit-cli/src/cli.rs:489-506` — split subcommand structure
- `docs/cli/blit.1.md:90-96` — manpage describes both
- `CHANGELOG.md:46-47` — CHANGELOG describes both
- `docs/plan/BLIT_UTILS_PLAN.md:35` — stale single-form
**Notes**: Same shape as the previous finding — superseded plan content. Marking as M because someone re-reading the design intent would think completions is one verb with `--prefix`, then be surprised by the daemon-side script generation.

## Low severity

### transfer-shortflags-undocumented — `-v`, `-p`, `-c` short aliases shipped but not in manpage
**Plan says**: docs/cli/blit.1.md OPTIONS Transfer Options describes `--verbose`, `--progress`, `--checksum` by long form only. The only short flag the manpage acknowledges is `-y` for `--yes` (mirror, move) at line 129.
**Code does**: `crates/blit-cli/src/cli.rs:208,216,228` give `--verbose` short `-v`, `--progress` short `-p`, `--checksum` short `-c`. `cli.rs:219,571` give `--yes` and check `--checksum` short aliases too. These short forms work today but are invisible in the docs.
**Evidence**:
- `crates/blit-cli/src/cli.rs:208,216,228,571` — short aliases on transfer flags
- `docs/cli/blit.1.md:104-132` — manpage describes long form only
**Notes**: rsync/restic/etc. document short and long forms together. Low-stakes but worth a one-line manpage tweak.

### scan-default-port-9031-hardcoded — Magic port literal repeated in code and not as a documented constant
**Plan says**: docs/cli/blit.1.md:81 "a bare host (`server`, `server:9031`) routes to `list-modules`" — 9031 surfaced as the canonical default port.
**Code does**: `crates/blit-cli/src/scan.rs:63` hardcodes `if service.port == 9031` for port-elision in display. No shared constant exposed from blit-core/blit-app. If the default ever changes, the manpage's example port and the scan display will silently disagree. (Code inventory flags this same hazard.)
**Evidence**:
- `crates/blit-cli/src/scan.rs:63` — magic literal
- `docs/cli/blit.1.md:81` — documents the port value
**Notes**: Plan/doc drift hazard, not active drift today. Worth promoting `9031` to `blit_core::DEFAULT_DAEMON_PORT` so the docs and display stay in sync.

### profile-no-network-claim-also-uses-state-file — Plan claim is accurate, but predictor file path is not documented
**Plan says**: docs/cli/blit.1.md:97-100 "Reads `perf_local.jsonl` and the predictor state file; no network access." Plan inventory file: `files-perf-local` lists `${XDG_CONFIG_HOME:-$HOME/.config}/blit/perf_local.jsonl`.
**Code does**: `crates/blit-cli/src/profile.rs:6-32` confirms no network access (just calls into `blit_app::profile::query`). The predictor file path is exposed in the output (line 32 `Predictor state: {}`) but never named in the manpage FILES section (which only lists perf_local.jsonl and settings.json).
**Evidence**:
- `crates/blit-cli/src/profile.rs:6-32` — local-only
- `docs/cli/blit.1.md:175-177` — FILES section, no predictor state path
**Notes**: A documentation completeness gap rather than functional drift. Adding the predictor file path to FILES would close it.

### rust-msrv-claim-not-enforced — README says "Rust 1.56+" but `Cargo.toml` has no `rust-version` pin
**Plan says**: README.md:67 "Rust 1.56+ (edition 2021)".
**Code does**: `Cargo.toml` workspace-level (`/Users/michael/Dev/Blit/Cargo.toml:1-20`) and per-crate (`crates/blit-cli/Cargo.toml`) do not pin `rust-version`. The build will succeed only on rustc versions that support tonic / clap-complete / let-else / async-trait / 2021-edition combinations that, in practice, require much newer Rust than 1.56. README is making a claim the build does not enforce.
**Evidence**:
- `README.md:67` — claim
- `Cargo.toml:1-20` — no rust-version
- `crates/blit-cli/Cargo.toml:1-4` — no rust-version
**Notes**: Likely cargo would fail on 1.56 anyway because of dependencies. Either bump README to a realistic MSRV (e.g. 1.74 for `let … else` baseline) or set `rust-version` in workspace `[package]` so cargo enforces it.

## Claims that align well

- `verb-copy`, `verb-mirror`, `verb-move`, `verb-scan`, `verb-list-modules`, `verb-ls`, `verb-du`, `verb-df`, `verb-rm`, `verb-find`, `verb-completions-shell`, `verb-completions-remote`, `verb-profile`, `verb-diagnostics-perf`, `verb-diagnostics-dump` — all defined in `crates/blit-cli/src/cli.rs:48-86` and wired in `main.rs:45-87`.
- Endpoint syntax (`server:/module/path`, `server://path`, `server`) and forward-slash-only rule — enforced via `parse_endpoint_or_local` (called from `du.rs:9`, `df.rs:8`, `find.rs:9`, `rm.rs:14`, `completions.rs:50`, `ls.rs:10`).
- `--yes`/`-y` on mirror and move skips destructive prompts — verified at `transfers/mod.rs:181-190` (mirror) and `transfers/mod.rs:408-418` (move).
- `--yes` on rm skips prompt — verified at `rm.rs:48-58`.
- Mirror prompts unless `--yes` OR `--dry-run`; move prompts unless `--yes` (no dry-run branch since move bails on dry-run) — `transfers/mod.rs:181-190,251-253,408-418`.
- rm refuses to delete entire module / empty rel-path — `rm.rs:26-43` (defense in depth).
- `--checksum` is opt-in (metadata is the default) — `cli.rs:228-229`, `transfers/local.rs` comparison-mode selection.
- Shell completions supported: bash/zsh/fish/powershell/elvish — `cli.rs:680-694` test pins them all; `completions.rs:42-47` uses clap_complete.
- `--max-depth 0 = unlimited` for du — `cli.rs:423` (`Option<u32>`), `du.rs:20` (`unwrap_or(0)`).
- `--wait` default 2 seconds for scan — `cli.rs:394`.
- Block-level resume with Blake3 hashing — code path threads `resume: bool` through transfers; CHANGELOG line 11 accurate.
- Remote-to-remote direct (delegated) is default; `--relay-via-cli` opts into legacy relay — `cli.rs:306-313`, `transfers/mod.rs:230-241`.
- Daemon admin RPCs ListModules / List / Find / DiskUsage / FilesystemStats / CompletePath / Purge all implemented — `crates/blit-daemon/src/service/core.rs:845,916,938,973,1148,1198,1263`.
- Daemon `--root`, `--no-mdns`, `--force-grpc-data`, `--bind` shipped — `crates/blit-daemon/src/runtime.rs:83-100`.
- mDNS service type `_blit._tcp.local.` — referenced in `blit_app::scan` discovery code path.
- BLIT_UTILS_PLAN supersession (no separate binary, no separate crate, no separate manpage) — accurately reflected: `crates/` has no `blit-utils`, `docs/cli/` has no `blit-utils.1.md`.
- `--config-dir` global flag — manpage:173, code `cli.rs:42-43`, `main.rs:39-41`.
- Test file `crates/blit-cli/tests/blit_utils.rs` exists per CHANGELOG.md:63 — verified.
- `find` glob `*` does not cross `/` (literal_separator=true) — `crates/blit-daemon/src/service/admin.rs:505-507`.
- Pattern matches both relpath and basename — `crates/blit-daemon/src/service/admin.rs:546-557`.
- No TLS in core; security via SSH/VPN/reverse-proxy — consistent across manpage, CHANGELOG, README, and code (no rustls/native-tls in dependency tree for the CLI/daemon binaries beyond tonic's optional features).
