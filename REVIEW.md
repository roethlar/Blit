# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend

- `[ ]` Open — coder may pick up
- `[~]` In progress / pending review — sentinel in `.review/ready/`
- `[x]` Verified — verdict in `.review/results/<id>.verified.json`

## Currently pending review

| ID              | Severity | Title                                       | Status | Branch              | Commit    |
|-----------------|----------|---------------------------------------------|--------|---------------------|-----------|
| b-4-getstate    | Feature  | GetState RPC + DaemonState handler          | `[~]`  | `phase5/getstate`   | `b6b6bb2` |

## Open findings

| ID         | Severity | Title                                                    | Branch |
|------------|----------|----------------------------------------------------------|--------|
| B          | Feature  | `GetState` RPC + `ActiveJobs` table + recent ring        | `phase5/getstate` |
| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  |        |
| C          | Feature  | `Subscribe` RPC + byte-level instrumentation             |        |
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
