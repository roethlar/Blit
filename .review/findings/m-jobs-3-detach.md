# m-jobs-3-detach: detach field + spawn-closure disarm + CLI gates

**Severity**: Feature (new wire field, new CLI flag, new
behavior on the daemon)
**Status**: In progress / pending review
**Branch**: `phase5/m-jobs`
**Commit**: filled by the sentinel commit

## What

Third M-Jobs sub-slice (`docs/plan/TUI_DESIGN.md` §6.5).
m-jobs-1/-2 made the daemon side cancellable; this slice makes
it self-owning. After this slice an operator can fire
`blit copy --detach src:/m/ dst:/m/`, exit their shell, and
the destination daemon completes the transfer without them.
Cancel via `blit jobs cancel <remote> <transfer_id>` if
plans change.

## Approach

### Wire

`bool detach = 32;` on `DelegatedPullRequest`. Documented as
"only valid on DelegatedPull" — push / pull / pull_sync put
the CLI in the byte path, so detach is meaningless there.
Defaults to `false` (proto3), preserving the historical
behavior for older clients.

### Daemon

The existing `tokio::select!` in `delegated_pull` had three
arms: `tx.closed()` (R30-F2 client-hangup race),
`cancel_token.cancelled()` (m-jobs-1), and the handler
future. This slice adds an `if !detach` guard to the
`tx.closed()` arm. When detached:

- `tx.closed()` is disabled — disconnect doesn't drop the
  transfer.
- `cancel_token.cancelled()` is still live — `CancelJob`
  still works.
- The handler runs to completion or failure.

The outcome-mapping match below the select is unchanged.
`None` still resolves through the same client-cancel vs
CancelJob distinction via `cancel_token.is_cancelled()`. On
a detached transfer the only way `None` lands is via
CancelJob (because the tx.closed arm is gated off), so the
inferred cause is always correct.

### CLI

- `TransferArgs::detach` clap flag (visible in `--help`).
- `run_transfer` adds an up-front gate that bails before any
  RPC fires when the route can't honor detach:
  - Local-source or local-destination → CLI is in the byte
    path; disconnect drops bytes.
  - Remote→remote + `--relay-via-cli` → relay puts CLI in
    the byte path; same problem.
  - Remote→remote without relay → accepted; flows to the
    existing delegated dispatch path.
- `run_move` rejects `--detach` outright. The source-delete
  step is CLI-side; with detach the CLI exits as soon as
  the daemon starts, and the delete never fires. Silent
  move-becomes-copy. Error message points users at
  `blit copy --detach + blit rm` as the workaround.

### Plumbing

- `DelegatedPullExecution::detach` field in `blit-app`
  forwards into `DelegatedPullRequest`.
- CLI's `transfers/remote_remote_direct.rs` initializes
  `detach: args.detach` on the execution struct.

## Files changed

- `proto/blit.proto` — `+bool detach = 32;` on
  `DelegatedPullRequest`.
- `crates/blit-daemon/src/service/core.rs`:
  - `+let detach = req.detach;` before move.
  - `_ = tx.closed(), if !detach => …` guard on the
    select arm.
- `crates/blit-daemon/src/service/delegated_pull.rs`:
  - Test-only `DelegatedPullRequest` literal grew
    `detach: false`.
- `crates/blit-app/src/transfers/remote.rs`:
  - `+DelegatedPullExecution::detach: bool` field.
  - `run_delegated_pull` threads it into the
    `DelegatedPullRequest`.
- `crates/blit-cli/src/cli.rs`:
  - `+TransferArgs::detach: bool` (doc + `#[arg(long)]`).
- `crates/blit-cli/src/transfers/mod.rs`:
  - `run_transfer` up-front `--detach` rejection gate
    (3 cases: local endpoint, relay-via-cli, OK path).
  - `run_move` `--detach` rejection.
  - Test helpers `TransferArgs` literals initialize
    `detach: false`.
- `crates/blit-cli/src/transfers/remote_remote_direct.rs`:
  - `DelegatedPullExecution` literal grew
    `detach: args.detach`.

## Tests added

- `detach_rejected_for_local_to_local` — runs `run_transfer`
  with local src+dst + `--detach`, asserts the
  "remote→remote only" message.
- `detach_rejected_with_relay_via_cli` — remote→remote +
  `--detach` + `--relay-via-cli`, asserts the
  "incompatible with --relay-via-cli" message.
- `detach_rejected_on_blit_move` — `run_move` + `--detach`,
  asserts the "move does not support --detach" message.

Workspace: 536 passed (was 533; +3).

## Known gaps

1. **No happy-path test on the daemon side.** The select
   guard's `if !detach` arm is exercised structurally — the
   handler tests for `cancel_job_ok_for_delegated_pull`
   (m-jobs-2) verify the cancel-via-token path still works
   under both detach values, but I haven't added a focused
   test asserting "tx.closed() does NOT drop the handler
   when detach=true." That would require an in-process
   tonic server and a client that closes its receiver mid-
   transfer — bigger than the slice should be. M-Jobs
   integration suite can add it once the events ring and
   subscribe wire land.

2. **CLI exit-after-Started isn't implemented.** The flag
   only changes daemon-side behavior; the CLI still
   awaits the handler stream by default. M-Jobs-4 (per-job
   event ring + Subscribe filter) and m-jobs-6 (watch CLI)
   are the natural place to expose "exit after Started"
   semantics. For now an operator running `blit copy
   --detach` still waits for the transfer to finish on
   their terminal — they get the daemon-side guarantee
   that hanging up doesn't drop the transfer, but no
   CLI-side fire-and-forget yet. Documented as a gap in
   the `--detach` help text would be appropriate; can be
   added in a docs follow-up.

3. **Forward-version daemons emitting `detach=true` to
   older daemons.** Proto3 defaults the field to `false`,
   so an older daemon that doesn't know about `detach`
   simply ignores it. Behavior on that daemon is unchanged
   (tx.closed race always armed). No deserialization
   error, no surprise — but a `--detach` user against an
   older daemon will get the "client-cancel kills transfer"
   semantic. Diagnostics could surface this in a future
   slice that checks daemon version before honoring the
   flag client-side; out of scope here.

## Reviewer comments

(empty — pending grade)
