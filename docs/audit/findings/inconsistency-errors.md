# Inconsistency Findings: Error wrapping, propagation, user-facing messages
**Generated**: 2026-06-04
**Findings**: 12 (4 H / 5 M / 3 L)

---

## High severity

### errors-1 — `--metrics`-disabled vs `--metrics`-enabled token rejection uses different Status codes
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-daemon/src/service/push/data_plane.rs:185` — push data-plane invalid token → `Status::permission_denied("invalid data plane token")`
2. `crates/blit-daemon/src/service/pull.rs:740` — pull data-plane invalid token → `Status::permission_denied("invalid pull data plane token")`
3. `crates/blit-daemon/src/service/pull_sync.rs:632` — pull_sync data-plane invalid token → `Status::unauthenticated("invalid data plane token")`
4. `crates/blit-daemon/src/service/pull_sync.rs:754` — pull_sync_resume data-plane invalid token → `Status::unauthenticated("invalid data plane token")`

**Canonical**: All four are the same logical event (peer presented wrong handshake token). All four should be the same gRPC code. `Status::unauthenticated` is the semantically correct one (token = bearer credential failure), but the code surface is inconsistent across just the daemon. Pick one — operator-facing parity matters because client retry logic and metrics may dispatch on `status.code()`.

Also note message-text divergence: pull writes `"invalid pull data plane token"` (qualified), push and pull_sync write `"invalid data plane token"` (unqualified) — a logger filtering by message text will see two flavors.

**Recommendation**: Single helper `fn reject_invalid_token() -> Status` that returns one code (Unauthenticated) and one literal string for all four sites.

---

### errors-2 — Admin RPC clients drop Status code; jobs RPC clients preserve it — caller sees same daemon error in two shapes
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-app/src/admin/rm.rs:30` — `purge`: `.map_err(|status| eyre::eyre!(status.message().to_string()))` (code stripped)
2. `crates/blit-app/src/admin/du.rs:48,54` — `du::query`/`stream`: same pattern (code stripped)
3. `crates/blit-app/src/admin/df.rs:33` — same pattern
4. `crates/blit-app/src/admin/list_modules.rs:32` — same pattern
5. `crates/blit-app/src/admin/ls.rs:74` — same pattern
6. `crates/blit-app/src/admin/find.rs:55,61` — same pattern
7. `crates/blit-app/src/admin/jobs.rs:91-95` — `cancel`: preserves code: `"CancelJob failed ({code}): {msg}"`
8. `crates/blit-app/src/admin/jobs.rs:114-119` — `clear_recent`: preserves code: `"ClearRecent failed ({code}): {msg}"`
9. `crates/blit-cli/src/completions.rs:84` — `complete_path`: drops code via `eyre::eyre!(status.message().to_string())`

**Canonical**: jobs::cancel / jobs::clear_recent style ("RPC failed ({code}): {message}") — preserves the gRPC code so the operator can distinguish PermissionDenied vs NotFound vs Unavailable.

**Recommendation**: Single helper `fn status_to_eyre(rpc_name: &str, status: Status) -> eyre::Report` used everywhere. Without it, `blit rm /server:/module/foo` against a read-only module shows `"module 'foo' is read-only"` (no code) while `blit jobs cancel ...` against same-condition daemon shows `"CancelJob failed (FailedPrecondition): ..."`. The shape divergence also blocks retry classification: see errors-3.

---

### errors-3 — `is_retryable` can never trigger on Status-class errors because admin clients erase the io::Error chain
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-app/src/transfers/retry.rs:27-46` — `is_retryable` only returns true if the eyre chain contains a `std::io::Error` of a transient kind (TimedOut / ConnectionReset / etc).
2. `crates/blit-app/src/admin/{rm,du,df,list_modules,ls,find}.rs` — wrap tonic `Status` via `eyre::eyre!(status.message().to_string())` — NO io::Error in the chain, even when the underlying status is `Code::Unavailable` (transport-class) or `Code::DeadlineExceeded`.
3. `crates/blit-app/src/transfers/remote.rs:709-725, 738-751` — preserve `status.code()` but still build the eyre via `eyre!(...)` — no io::Error source.

**Canonical**: `Code::Unavailable` and `Code::DeadlineExceeded` from a tonic Status are equivalent to transport failures and should be retryable. Either retry the Status directly via a code-aware classifier, OR wrap status into an eyre with an io::Error source for transient codes.

**Recommendation**: Extend `is_retryable` to also walk `eyre::chain()` for `tonic::Status` and return true on `Unavailable`/`DeadlineExceeded`/`Aborted`. Today `run_with_retries` is silently a no-op for the most common transient remote-failure class.

---

### errors-4 — Unreadable-paths refusal message differs between CLI move, TUI move, daemon push, daemon pull_sync — four flavors of the same data-loss guard
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:463-479` — CLI local move: detailed message, quotes first **5** unreadable paths, full explanation ("Files we couldn't read were skipped during the copy — deleting the source now would lose them. Resolve the scan errors (typically permissions) and re-run.")
2. `crates/blit-tui/src/main.rs:4167-4179` — TUI local move (perform_local_move): same family, quotes first **3** paths, shorter explanation ("Resolve the scan errors (typically permissions) and re-run.")
3. `crates/blit-daemon/src/service/pull_sync.rs:143-160` — daemon pull_sync (mirror or move): quotes first **5**, says "Resolve the scan errors (typically permissions) on the daemon side."
4. `crates/blit-daemon/src/service/push/control.rs:328-332` — daemon push (mirror): **does not quote any paths**, message is just "source scan was incomplete (unreadable paths); refusing to purge destination to prevent data loss. Resolve the unreadable source path(s) and retry."

**Canonical**: The data-loss guard is the same logical event ("source scan didn't see every file; can't safely delete based on that view"). The four messages must agree on (a) preview length, (b) wording of the rationale, (c) remediation phrasing. The CLI/daemon-pull pair already quotes 5; TUI quotes 3; daemon-push quotes 0 (a worse user experience because the operator can't act).

**Recommendation**: Build a single `format_incomplete_scan_refusal(operation: &str, unreadable: &[String], side: Side)` helper in `blit-app` or `blit-core` and call it from all four sites. Today an operator sees three different stories for the same failure mode depending on which path they hit.

---

## Medium severity

### errors-5 — "Refusing to delete module root" differs between CLI rm and daemon Purge handler
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-cli/src/rm.rs:27-31, 39-43` — CLI message: `"refusing to delete entire module '{module}'; specify a sub-path"` (qualifies with module name)
2. `crates/blit-daemon/src/service/admin.rs:36-39` — daemon-side: `"refusing to delete module root; specify a sub-path"` (no module name)

**Canonical**: The CLI version is more useful (names the module). Whichever wins should be the only thing the operator ever sees. Today the daemon message is the only one that ever surfaces to a remote caller bypassing the CLI's pre-check (e.g., the TUI's `build_delete_request`, future SDK callers).

**Recommendation**: Pick CLI form, port to daemon. Update the daemon Status to include the module name.

---

### errors-6 — Source-delete-failed message uses three different past-tense verbs across move paths
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-tui/src/main.rs:3339` — F3 pull-then-delete (TUI): `"received but failed to delete remote source: {err:#}"`
2. `crates/blit-tui/src/main.rs:3423` — F1 push-then-delete (TUI): `"pushed but failed to delete local source: {err:#}"`
3. `crates/blit-tui/src/main.rs:3523` — F1 delegated-then-delete (TUI): `"delegated but failed to delete remote source: {err:#}"`
4. `crates/blit-app/src/transfers/remote.rs:250` — remote-side delete inside transfer pipeline: `"failed to delete {target}: {e}"` — no "received/pushed/delegated but ..." prefix

**Canonical**: All four are the same logical event: copy succeeded, source-delete step failed → user MUST be told the source still exists. The TUI variants follow a pattern; the deeper-layer site (250) doesn't. Sub-issue: CLI move's local-local case in `crates/blit-cli/src/transfers/mod.rs:484-487` uses `.with_context(|| format!("removing {}", src_path.display()))` — yet another phrasing ("removing X") for the same data-loss-class signal.

**Recommendation**: One helper `format_post_transfer_delete_failure(operation: TransferKind, side: Side, err) -> String` so the operator always sees the same shape. Critically, the failure is data-loss-adjacent (operator must KNOW the copy succeeded so they don't re-run and double-write) — it cannot be a passing message that gets lost in the noise.

---

### errors-7 — TUI strips `Status::code()`, only keeps `.message()` — operator loses ability to distinguish transport from authz errors in the footer
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-tui/src/main.rs:5717-5720` — DaemonEvent subscription stream loss: `format!("stream: {}", status.message())` — code discarded
2. `crates/blit-tui/src/main.rs:5711` — clean stream end: `"stream ended"` — no diagnostic
3. CLI counterpart `crates/blit-cli/src/jobs.rs:393-411` — uses `WatchSnapshot::Active(... "still active after stream loss")` and exits with code 3, distinguishing the failure modes via exit codes

**Canonical**: An `Unavailable` status (daemon went down) is operationally very different from a `Cancelled` status (user-side close) or `Internal` (daemon bug). Today the TUI cannot tell them apart in the banner.

**Recommendation**: When forwarding a Status into an Error fragment, format as `"stream: {code}: {message}"` so the operator can correlate with daemon logs.

---

### errors-8 — `MirrorMode::Unspecified|Off` ambiguity surfaces as different errors on the two purge paths
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-daemon/src/service/push/control.rs:343-345` — push handler treats `MirrorMode::Unspecified | Off` (when `mirror_mode=true`) as "use user's filter" (back-compat shape for older clients).
2. `crates/blit-daemon/src/service/pull_sync.rs:430-462` — `scope_deletions` treats `Off | Unspecified` as `Vec::new()` (delete nothing).

**Canonical**: Same enum value triggers two different behaviors. If a client lands on a daemon-push path with this enum, mirror semantics happen; if it lands on pull_sync, they don't. There's no user-visible error here — the issue is that no error is raised when the caller's intent is ambiguous.

**Recommendation**: Either make `Unspecified` a hard reject (InvalidArgument "mirror_kind must be explicitly set when mirror_mode=true") OR normalize both sites to the same fallback. Today this is silent semantic drift, not loud miscommunication.

---

### errors-9 — Local move's source-not-exist message duplicated in 4+ places with slight drift
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:208` — `bail!("source path does not exist: {}", src.display())` (run_transfer)
2. `crates/blit-cli/src/transfers/mod.rs:216` — same message (different code path)
3. `crates/blit-cli/src/transfers/mod.rs:433` — same message (run_move)
4. `crates/blit-cli/src/transfers/mod.rs:536` — same message (different move path)
5. `crates/blit-cli/src/transfers/local.rs:82` — `bail!("source path does not exist: {}", src_path.display())` (one layer down)
6. `crates/blit-cli/src/check.rs:25-28` — `check` uses same string + a `destination path does not exist` companion

**Canonical**: Convergent today, divergent tomorrow. One helper.

**Recommendation**: `fn require_path_exists(side: &str, p: &Path) -> Result<()>` shared in blit-app. Same fix removes the destination-side gap (CLI transfer doesn't pre-check destination existence the way `check` does).

---

## Low severity

### errors-10 — `with_context` vs `eyre!` vs `bail!` are mixed inconsistently for the same kind of failure
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-app/src/transfers/remote.rs:336, 351, 468` — connection failures use `.with_context(|| format!("connecting to {}", ...))` (preserves source chain)
2. `crates/blit-app/src/transfers/remote.rs:709-725` — RPC failures use `eyre!(...)` (no source chain)
3. `crates/blit-app/src/admin/*.rs` — RPC failures use `eyre::eyre!(status.message().to_string())` (no source chain, no rpc name in message)
4. `crates/blit-cli/src/transfers/mod.rs` — most CLI bails are flat strings via `bail!`

**Canonical**: `with_context` is correct for any RPC site because it preserves the chain (essential for `is_retryable` per errors-3 and for `{err:#}` formatting to show the underlying cause).

**Recommendation**: Audit-pass replacing `eyre!(...)` with `.with_context(...)` wherever a real source error exists.

---

### errors-11 — "starting copy" banner appears on CLI but TUI doesn't surface it
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:194-203` — "starting copy SRC -> DST" on stderr (CLI move at :420-428)
2. `crates/blit-tui/src/main.rs` — no equivalent banner; TUI shows only final status

**Canonical**: Not strictly an error but a UX expectation drift — CLI operators see "starting", TUI operators don't. Cosmetic, but the divergence is real.

**Recommendation**: TUI could emit an equivalent "starting X" status line; or both could be silent in non-verbose mode. Pick one expectation.

---

### errors-12 — Three different past-tense verbs ("received", "pushed", "delegated") used as adjectives in error prefixes
**Dimension**: Error wrapping, propagation, user-facing messages
**Instances**:
1. `crates/blit-tui/src/main.rs:3339,3344` — "received but ..."
2. `crates/blit-tui/src/main.rs:3423` — "pushed but ..."
3. `crates/blit-tui/src/main.rs:3523,3528` — "delegated but ..."

(These cluster with errors-6 but are flagged separately as a *style* drift — "delegated" is awkward as a past-tense main verb of a sentence in a way "received" and "pushed" are not.)

**Canonical**: "copy succeeded but source-delete failed" reads cleaner than "delegated but failed to delete remote source".

**Recommendation**: Re-phrase as "copy succeeded but {side}-side source delete failed: {err}" uniformly.

---

## Coverage attestation
- Read every code-*.md inventory file.
- Grepped: `bail!`, `eyre!`, `Status::`, `map_err`, `with_context`, `wrap_err`, "containment", "read-only", "module root", "failed to delete", "stream:", "but failed to" across `crates/blit-{cli,tui,daemon,app,core}`.
- Read source at: `blit-cli/src/{rm,jobs,transfers/mod,transfers/local,check,completions}.rs`; `blit-daemon/src/service/{core,admin,pull,pull_sync,push/control,push/data_plane,util,delegated_pull,delegation_gate}.rs`; `blit-tui/src/main.rs` (forward-step, prepare_local_transfer, perform_local_move); `blit-app/src/admin/{rm,jobs,du,df,find,ls,list_modules}.rs`; `blit-app/src/transfers/{retry,remote}.rs`.
