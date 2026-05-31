# d-61-f1-trigger-push: local→remote copy push from the F1 trigger

**Severity**: Feature (designed — TUI_DESIGN §1 "between any two endpoints")
**Status**: In progress / pending review (round 3)
**Branch**: `phase5/a1`
**Commit**: `83e8675` (round 1: `68f5389`, round 2: `3508007`)

## What

The F1 trigger (d-58…d-60) handles **remote→local** transfers by
delegating to the F3 pull machine. The opposite direction —
pushing a **local** path to a remote daemon — had no path in the
TUI at all (only the CLI could push). d-61 adds it: a local→remote
**copy** push, launched from the same trigger modal.

## Approach

### Direction detection in the trigger commit

On Enter, `take()` yields `(source, dest, kind)`. The handler
classifies the source with
`blit_app::endpoints::parse_transfer_endpoint` (round 2 — see the
reviewer note; the round-1 raw `RemoteEndpoint::parse` was the
footgun):

- `Ok(Endpoint::Remote(src))` → remote→local (pull family):
  Copy launches via `start_pull`; Mirror/Move route through the
  F3 confirm gate (Move gated against module roots, d-60).
- `Ok(Endpoint::Local(path))` → local→remote push (Copy only);
  the dest must parse as `Endpoint::Remote` **and** pass
  `ensure_remote_destination_supported` (round 3 — reject a
  bare-host `Discovery` dest), else drop.
- `Err(_)` → a malformed remote-shaped (`:/`) or forward-slash
  input → **drop** (must not become a local push). Inline
  parse-error feedback is a follow-up.

### A dedicated push lifecycle (no F3 reuse)

The F3 pull machine is remote→local specific, so push gets its
own small state — `f1push::F1PushState` (Idle / Running / Done /
Error), mirroring the f3del shape:

- `begin(label) -> Option<u64>` bumps a monotonic run id,
  transitions to `Running`, returns the `request_id` (no-op while
  already running).
- `spawn_f1_push(request_id, local_src, remote, tx)` builds a
  `PushExecution` (`Endpoint::Local(src)`, `FileFilter::default()`,
  `mirror_mode=false`, `MirrorMode::Off`, no scan-gate) and runs
  `run_remote_push(.., None)` — **no live progress** this slice —
  flattening to an `F1PushReply { Ok((files, bytes)) | Err }`.
- A new event-loop select arm applies the reply via
  `apply_done` / `apply_error` (generation-guarded by
  `request_id`, so a superseded run's reply is dropped).

The push status renders on the **F1 footer** (no jump to F3 —
that's the pull direction): `push → <dest>…` (running),
`pushed N file(s) · X → <dest>` (done), `failed: <msg>` (error).
Footer priority: trigger modal > push status > discovery footer.
`PushStatusDisplay` is a screens-side type built by a bridge, so
`screens/f1.rs` stays decoupled from `f1push`.

## Files changed

- `crates/blit-tui/src/f1push.rs` (new): the push state machine
  + unit tests.
- `crates/blit-tui/src/main.rs`: `mod f1push`; `f1_push` +
  `f1_push_reply_tx` AppState fields + inits; `F1PushReply` +
  channel; the trigger-Enter push branch; `spawn_f1_push`; the
  push reply select arm; `f1_push_status` bridge; render call.
- `crates/blit-tui/src/screens/f1.rs`: `PushStatusDisplay` +
  `render_push`; `render_into` push param + footer priority.
- `crates/blit-tui/src/help.rs`: `t` keymap row reworded
  (remote↔local).

## Tests

538 total (was 529):

f1push.rs (7): idle; begin → Running with label; begin no-op
while running; apply_done / apply_error terminal states; stale
reply dropped; run ids increment.

main.rs (2): `..._enter_starts_push_for_local_source` (local src
+ remote dest + Copy → push runs, stays on F1, not a pull —
tokio test for the detached spawn);
`..._local_source_mirror_does_not_push` (copy-only — mirror with
a local source doesn't launch).

The push RPC itself needs a live daemon (manual); the direction
detection, state machine, and footer wiring are unit-tested.

## Known gaps

1. **Copy-only.** Mirror push (server-side delete-extraneous)
   and move push (delete the local source after) aren't wired —
   they need their own confirm gates. A local source with
   mirror/move selected is a no-op.
2. **No live byte progress.** `run_remote_push` is called with
   `progress = None`; the footer shows a static `push → …` until
   the terminal reply. Wiring a progress forwarder (as F3 pull
   has) is a follow-up.
3. **No terminal auto-hide.** The Done/Error footer persists
   until the next push begins (no TTL sweep). A follow-up could
   add the d-38-style auto-hide.
4. **No inline parse-error feedback** (d-58 gap).
5. **remote→remote (delegated) trigger** still pending.

## Out of scope

- Mirror/move push; live progress; remote→remote; F1 `d`
  diagnostics.

## Reviewer comments

### Round 1 (reopened)

> Malformed remote-shaped sources are misclassified as local push
> sources. The push branch is entered whenever
> `RemoteEndpoint::parse(src)` fails and kind is Copy — but that
> parser also fails for malformed remote-shaped inputs (e.g.
> `nas:9031:/home`, missing the module trailing slash). So
> `src: nas:9031:/home  dst: other:9031:/backup/` falls through to
> push, launching `spawn_f1_push(PathBuf::from("nas:9031:/home"))`
> — the same footgun the transfer parser avoids. Distinguish a
> genuine local source from a malformed remote-shaped one (reject
> `:/` / `://` on parse failure, or reuse the strict transfer
> parser). Add a regression test for that case.

**Response (3508007):** Replaced the raw `RemoteEndpoint::parse`
classification with `blit_app::endpoints::parse_transfer_endpoint`
— the exact strict parser the CLI uses, which returns `Err` for
`:/`-shaped inputs and `Local` only for genuine paths. The source
match is now `Remote → pull`, `Local → push`, `Err → drop`; the
push dest is parsed the same way. Added the reviewer's regression
case (`..._malformed_remote_source_does_not_push`): `nas:9031:/home`
→ `other:9031:/backup/` starts neither a push nor a pull. 539
tests green, fmt + clippy clean.

### Round 2 (reopened)

> Bare-host push destinations still start an invalid push. The
> push branch accepts any `Ok(Endpoint::Remote)` from
> `parse_transfer_endpoint(&dest)` before `f1_push.begin` /
> `spawn_f1_push`. A bare host (`nas` / `nas:9031`) parses as
> `RemotePath::Discovery`, which the push client later rejects
> ("missing module specification") — but the TUI already started
> a push and showed the Running footer. Require the dest to pass
> the destination-shape gate (`ensure_remote_destination_supported`)
> first. Add a regression test for `dst = "nas:9031"`.

**Response (83e8675):** Gated the parsed remote dest with
`ensure_remote_destination_supported` (the CLI's preflight)
before `f1_push.begin`/`spawn_f1_push`, so a bare-host
`Discovery` dest is dropped without starting a push. Added
`..._bare_host_dest_does_not_push` (`/tmp/src` → `nas:9031`
starts neither push nor pull); the valid `nas:9031:/home/` push
test remains the positive case. 540 tests green, fmt + clippy
clean.
