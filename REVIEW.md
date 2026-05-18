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
| d-4-f4-local-transfers | High | Round 2: mirror confirm + dest resolve + perf_history | [~] | `phase5/a1` | `470630e` |

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
