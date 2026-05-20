# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend

- `[ ]` Open — coder may pick up
- `[~]` In progress / pending review — sentinel in `.review/ready/`
- `[x]` Verified — verdict in `.review/results/<id>.verified.json`

## Currently pending review

| ID                | Severity | Title                                       | Status | Branch      | Commit    |
|-------------------|----------|---------------------------------------------|--------|-------------|-----------|
| d-51-f3-select-all | Feature | `a` selects/clears all visible F3 rows | `[~]` | `phase5/a1` | `e914084` |

## Open findings

| ID         | Severity | Title                                                    | Branch |
|------------|----------|----------------------------------------------------------|--------|
| B          | Feature  | `GetState` RPC + `ActiveJobs` table + recent ring        | `phase5/getstate` |
| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  | `phase5/m-jobs` |
| C          | Feature  | `Subscribe` RPC + byte-level instrumentation             | `phase5/c` |
| A.1        | Feature  | TUI implementation                                       |        |
| D          | Feature  | Verify + diagnostics screens                             |        |
| E          | Feature  | Polish (themes, refresh rates, config)                   |        |
| P0-§2.6    | Feature  | Live remote benchmark capture (hardware-bound)           |        |

## Verified history

Phase 5 A.0 complete. Per-finding audit trails in
`.review/findings/a0-*.md` and `.review/results/a0-*.verified.json`.
Sub-slices on branch `phase5/blit-app-extract`:

- `b5d2414` Crate scaffold + endpoints
- `4800cfc` df / du / find / list-modules / rm
- `009583c` ls (initial)
- `af436b2` ls — LocalListing enum fix
- `39966df` scan
- `d6ee06a` profile
- `334a684` diagnostics (perf + dump)
- `2626f9b` diagnostics — perf best-effort fix
- `e807f46` check
- `44a4f8c` util.rs split
- `2a37a3e` transfers/local
- `8c4174a` transfers/filter
- `3639159` transfers/resolution
- `65f6031` transfers/resolution — followups (`a0-resolution-fixup`)
- `b2d6c9c` transfers/remote — pull-flow helpers (`a0-remote-helpers`)
- `e25707e` transfers/remote — pull entry-point (`a0-pull-execution`)
- `cb96331` transfers/remote — push entry-point (`a0-push-execution`)
- `1879019` transfers/remote_remote_direct (`a0-delegated-execution`)
- `8860cac` transfers/dispatch (`a0-dispatch`)
- `6eeb214` endpoints — support gates (`a0-endpoints-gates`)
- `29a2026` final cleanup — drop CLI shim re-exports (`a0-final-cleanup`)

Phase 5 B sub-slices on branch `phase5/getstate`:

- `10259ec` ActiveJobs table on `BlitService` (`b-1-active-jobs`)
- `ef46631` Streaming RPCs populate ActiveJobs rows (`b-2-set-endpoint`)
- `eab1a17` TransferRecord ring + outcome capture (`b-3-recent-ring`)
- `b6b6bb2` GetState RPC + DaemonState handler (`b-4-getstate`)
- `5f8ca5b` `blit jobs list <remote>` consumes GetState (`b-5-jobs-list`)

Phase 5 M-Jobs sub-slices on branch `phase5/m-jobs`:

- `1e493c0` Per-row CancellationToken + delegated_pull race (`m-jobs-1-cancel-token`)
- `66df256` CancelJob RPC + `blit jobs cancel` CLI (`m-jobs-2-cancel-rpc`)
- `1221d60` detach field + fire-and-forget CLI path (`m-jobs-3-detach`)
- `09cffbb` `blit jobs watch` polling surface (`m-jobs-6-watch`)

Phase 5 C sub-slices on branch `phase5/c`:

- `234d2c6` Byte counter API in `blit-core` + ActiveJobs row wiring (`c-1a-byte-counter-api`)
- `14eeda7` Delegated-pull data-plane byte reporting, including tar-shard/block paths (`c-1b-byte-counter-wiring`)
- `0ffaac7` Subscribe RPC + DaemonEvent + TransferStarted (`c-2-subscribe-skeleton`)
- `df8249d` SubscribeRequest reserved-tag order aligned with `TUI_DESIGN.md` (`c-2-subscribe-skeleton` round 2)
- `5be5f10` TransferComplete + TransferError terminal events (`c-3-transfer-finished-events`)
- `7d4fd28` Terminal events broadcast after ActiveJobs/GetState drain (`c-3-transfer-finished-events` round 2)
- `69224e0` Periodic TransferProgress events (`c-4-transfer-progress`)
- `5b88f3a` Progress events serialized before terminal events (`c-4-transfer-progress` round 2)
- `6330a7d` SubscribeRequest transfer_id_filter (`c-5a-transfer-id-filter`)
- `7587b46` Per-subscriber forwarder drains broadcast before filtering (`c-5a-transfer-id-filter` round 2)
- `d901656` Forwarder exits on client disconnect (`c-5a-transfer-id-filter` round 3)
- `b71fd6d` Per-job event ring + `replay_recent` (`c-5b-event-ring`)
- `ea7a8d7` `blit jobs watch` consumes Subscribe stream (`c-6-jobs-watch-stream`)
- `f7edcc9` Subscribe-first watch race fix + stable terminal JSON (`c-6-jobs-watch-stream` round 2)
- `7d3ff3f` `blit jobs watch` enables `replay_recent` (`c-7-watch-replay`)

Phase 5 A.1 sub-slices on branch `phase5/a1`:

- `2237521` `blit-tui` crate scaffold + terminal lifecycle (`a1-1-tui-scaffold`)
- `1176331` F2 Transfers pane with live Subscribe stream (`a1-2-f2-transfers`)
- `1fcee97` F1 Daemons pane with Local row, viewport-aware table, and rescan clamp (`a1-3-f1-daemons`)
- `3ac1cb2` F1 detail block populated by `GetState` (`a1-3b-f1-getstate-detail` round 2)
- `2abc71b` F3 Browse pane with modules + directory tree (`a1-4-f3-browse` round 2)
- `d33fedc` F4 Profile pane with read-only perf history + predictor (`a1-5-f4-profile`)
- `72d67ed` F-key screen router with router-owned input task (`a1-6-screen-router` round 2)
- `8719925` AppState unified loop with generation-guarded F2 setup (`a1-6b-state-preservation` round 3)
- `0607c70` F4 profile lifecycle actions preserve mutation-error banners (`d-1-f4-profile-lifecycle` round 2)
- `62eef1e` F4 Verify pane with generation-safe in-flight edits (`d-2-f4-verify` round 2)
- `d26ca9c` F4 diagnostics dump mirrors CLI JSON shape (`d-3-f4-diagnostics` round 2)
- `aba7394` `?` help overlay global from Verify edit mode (`e-1-help-overlay` round 2)
- `470630e` F4 local copy/mirror triggers with mirror confirmation (`d-4-f4-local-transfers` round 2)
- `5b51ee0` Responsive tab-strip counts include F4 local transfers (`e-2-tab-strip-counts` round 2)
- `dc03872` F4 local move trigger with source-delete confirmation (`d-5-f4-local-move`)
- `368dd4b` F4 Verify checksum toggle (`d-6-f4-verify-checksum-toggle`)
- `428cd22` F4 Verify one-way toggle (`d-7-f4-verify-one-way-toggle`)
- `9b01340` F4 Verify/Transfer Done durations (`d-8-f4-elapsed-time`)
- `f011cb3` F4 live elapsed ticker (`d-9-live-tick` round 2)
- `b625ca1` F4 transfer Done throughput (`d-10-transfer-throughput`)
- `fa18813` F1/F3/F4 freshness live tick (`d-11-freshness-tick` round 2)
- `ec8695f` F4 Esc cancels mirror/move confirm (`d-12-esc-cancels-confirm` round 2)
- `8787b5f` F2 footer last-event age (`d-13-f2-freshness-footer`)
- `f1217b7` F2 active-row age column (`d-14-f2-active-row-age`)
- `31541a7` F2 active-row percent complete (`d-15-f2-active-row-progress`)
- `d471f1e` `?` help overlay documents active-pane refresh (`d-16-help-overlay-keymap-sync` round 2)
- `873757b` F4 Verify Done preview lines (`d-17-verify-result-preview`)
- `a4416c8` Ctrl-U clears focused Verify field (`d-18-verify-form-clear`)
- `aac0b22` Digit aliases for F1-F4 pane navigation (`d-19-digit-tab-shortcuts`)
- `2e11732` F2 recent throughput column + layout doc sync (`d-20-f2-recent-throughput` round 2)
- `5e8856f` F2 active-row cursor anchored by transfer_id (`d-21-f2-active-cursor` round 2)
- `852fe10` F2 cancel-selected action + layout doc sync (`d-22-f2-cancel-selected` round 2)
- `94f556a` F2 cancel-status fragment auto-hide TTL (`d-23-cancel-status-auto-clear`)
- `db779dc` Configurable F2 cancel TTL drives sleep budget (`d-24-config-cancel-ttl` round 2)
- `7bc5e57` F2 TiB/TiB/s formatter tiers aligned with F4 (`d-25-f2-tib-tier`)
- `a89dd48` F3 substring filter via `/` (`d-26-f3-filter` round 2)
- `d3ba561` F3 stable sort with deterministic case-variant tiebreak (`d-27-f3-sort` round 2)
- `0fbaad6` F3 no-match filter empty-state message (`d-28-f3-no-matches-msg`)
- `b7b08a9` Opt-in F2 cancel confirmation prompt (`d-29-confirm-cancel`)
- `d778f76` F2 batch cancel freezes confirmed transfer ids (`d-30-batch-cancel` round 2)
- `13459b4` Scrollable help overlay for small terminals (`d-31-help-scroll`)
- `614e58d` Help overlay scrollbar indicator (`d-32-help-scrollbar`)
- `c89d278` TUI config loader warning ordering + Verify defaults (`e-3-config-scaffold` round 2)
- `94a30c2` Configurable tab-strip counts (`e-4-config-tab-strip-counts`)
- `7b5310e` Configurable live-tick interval + source-doc sync (`e-5-config-live-tick-interval` round 2)
- `72b742b` Verify path prefill config + schema doc sync (`e-6-verify-prefill` round 2)
- `12ee960` Configurable tab-strip accent with contrasting active-tab foreground (`e-7-config-theme` round 2)
- `7baf2a4` F3 pull-source spec preview with bracketed IPv6 authority (`d-33-f3-pull-source` round 2)
- `eb1ee45` F3 pull-source preview derived via `RemoteEndpoint` (`d-34-f3-pull-endpoint`)
- `beb5d9e` F3 pull destination prompt + execution with resolved local destinations (`d-35-f3-pull-execute` round 2)
- `27a8005` Ctrl+R hot-reload of tui.toml with parse-error keep-current semantics (`d-36-hot-reload-config`)
- `cd1751a` F3 pull live byte/file footer with pull-receive accumulator semantics (`d-37-f3-pull-progress` round 2)
- `7c523e8` F3 pull Done/Error footer auto-hide TTL (`d-38-f3-pull-ttl`)
- `43258ab` F3 pull progress footer average throughput (`d-39-f3-pull-throughput`)
