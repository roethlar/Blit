# Inconsistency Findings: Endpoint parsing & classification
**Generated**: 2026-06-04
**Findings**: 9 (H: 2, M: 4, L: 3)

## High severity

### endpoints-h1 — F1 confirm-detail silently treats parse Err as Local (violates project memory)
**Dimension**: Endpoint parsing & classification

The project memory `feedback_endpoint_parse_err.md` is explicit: "4 buckets: module/root=remote, bare-discovery & local=local, Err=reject. Reopened d-61, d-68 ×3." The reviewer has flagged this exact pattern multiple times. The TUI dispatch site `plan_f1_trigger` honors the rule (Err → `TriggerOutcome::Rejected`); the confirm-detail rendering does NOT — it falls through to "deletes the local source" on parse Err. The user pressing `y` at that prompt sees a message that lies about which side will be erased.

**Instances**:
  1. `crates/blit-tui/src/display_f1.rs:46-54` — `confirm_detail` for `PullKind::Move` does `match parse_transfer_endpoint(source) { Ok(Endpoint::Remote(_)) => "deletes the remote source", _ => "deletes the local source" }`. Err is silently classified as "local source." Inventory `code-tui-display.md` flagged this verbatim.
  2. `crates/blit-tui/src/main.rs:3578-3580` — `plan_f1_trigger` source parse: `parse_transfer_endpoint(src)` → on `Err` returns `TriggerOutcome::Rejected(format!("invalid source: {src}"))`. The trigger dispatch CANNOT reach the confirm prompt with an unparseable source, so the lying detail line is mostly unreachable today, but the divergent classifier remains.
  3. `crates/blit-tui/src/main.rs:3599-3623` — `plan_f1_trigger` destination parse: `Err(_) → TriggerOutcome::Rejected(format!("invalid destination: {dest}"))`. Same rule applied correctly here.

**Canonical**: The trigger dispatch (Err → reject). The confirm-detail classifier must apply the same 4-bucket rule, not silently fall through.
**Recommendation**: Either route the confirm-detail through a shared classifier that returns `Result<DeleteVictim, _>` and reject Err at the gate; or guarantee the trigger can never enter `confirming` state on an unparseable source and add a `debug_assert!` so a future refactor can't reintroduce the lie. The simplest fix: surface an `unreachable!` arm matching the trigger contract.

### endpoints-h2 — `is_deletable_remote_path` filters silently in batch; CLI `blit rm` bails
**Dimension**: Endpoint parsing & classification

For the same logical operation ("delete this remote path"), the CLI rejects non-deletable inputs with an explicit error message, while the TUI silently filters them out of a batch and proceeds with whatever remains. A batch of N module-root selections in the TUI would be silently dropped to zero and produce no Purge — but `build_delete_request` returns `None` only when EVERYTHING is filtered. A mixed batch of {module-root, real-path} silently drops the module-root entry without notifying the operator.

**Instances**:
  1. `crates/blit-cli/src/rm.rs:24-43` — `blit rm`: `extract_module_and_path` then a second guard: `if rel_path.as_os_str().is_empty() || rel_path == Path::new(".")` → `bail!("refusing to delete entire module '{}'; specify a sub-path", module)`. Followed by a third guard after `rel_string` is joined: same bail. **Defense in depth, error on attempted module-root deletion.**
  2. `crates/blit-tui/src/del_request.rs:31-57` — `build_delete_request` calls `endpoints.into_iter().filter(is_deletable_remote_path).collect()`. Module-root entries are silently dropped from the deletable set. Only when `rel_paths.is_empty()` does it return `None`. A mixed batch silently shrinks; the operator's `D` press appears to succeed on the remaining items with no record that some were skipped.
  3. `crates/blit-tui/src/del_request.rs:65-74` — `is_deletable_remote_path` returns `false` for `Err` (parse failure), Discovery, and empty-rel-path. CLI bails with different error text for each of these.

**Canonical**: CLI behavior — refuse, don't silently drop. Operator must know which targets were skipped.
**Recommendation**: Change `build_delete_request` to return the skipped-count or a `Vec<SkipReason>` so the TUI banner can say "deleted N, skipped M (module root)". Even better: make module-root rows visually non-selectable in the F3 view so the situation can't arise.

## Medium severity

### endpoints-m1 — `parse_endpoint_or_local` (loose) vs `parse_transfer_endpoint` (strict) usage inconsistent across admin verbs
**Dimension**: Endpoint parsing & classification

Two parse functions exist in `blit_app::endpoints`: a strict one that errors on remote-shaped typos, and a loose one that falls back to Local for any unparseable input. Admin verbs are split unpredictably between three styles: loose-then-reject-local, strict (transfer-only), and bare `RemoteEndpoint::parse`. Same operation class, three different parsers, three different error messages on the same bad input.

**Instances**:
  1. `crates/blit-cli/src/rm.rs:14`, `crates/blit-cli/src/df.rs:8`, `crates/blit-cli/src/du.rs:9`, `crates/blit-cli/src/find.rs:9`, `crates/blit-cli/src/ls.rs:10`, `crates/blit-cli/src/completions.rs:50` — all use `parse_endpoint_or_local` and then bail on `Endpoint::Local`. A bare `host:/m` typo (missing trailing slash) parses as Remote → propagates the parse error.
  2. `crates/blit-cli/src/list_modules.rs:7-8` — uses `RemoteEndpoint::parse` directly. Inventory `code-cli.md` flagged this divergence: "every other verb uses the local-or-remote helper; here a bare local path would yield a less-friendly parse error."
  3. `crates/blit-cli/src/jobs.rs:33-34, 46-47, 175-176` — `jobs list/cancel/watch` also use `RemoteEndpoint::parse` directly. Comment says intentional since these verbs are remote-only, but the user-visible error string still diverges from `rm`/`df`/`du`/`find` on the same bad input.
  4. `crates/blit-cli/src/transfers/mod.rs:102-103` and `crates/blit-cli/src/diagnostics.rs:161-162` — use `parse_transfer_endpoint` (strict). Documented choice: transfer paths should catch remote-shaped typos before silently treating them as a local copy source.

**Canonical**: Convention is documented: loose for admin (`rm`/`ls`/etc.), strict for transfer. Two outliers (`list-modules`, `jobs`) skip the helper entirely and produce a different error class for local-path inputs.
**Recommendation**: Either make `parse_endpoint_or_local` infallible for everyone and centralize the "this verb requires remote" bail (a `require_remote!` macro or `Endpoint::require_remote() -> Result<RemoteEndpoint>` adapter); or document the three-tier policy explicitly in `endpoints.rs` so each new admin verb makes a conscious choice.

### endpoints-m2 — `extract_module_and_path` vs `module_and_rel_path`: identical logic, divergent error text
**Dimension**: Endpoint parsing & classification

Two helpers with byte-for-byte identical bodies (match the three `RemotePath` variants, error on `Discovery`) live in different modules with different error wording. `endpoints.rs` even documents the split: "Different from `admin::rm::extract_module_and_path` (rm-specific error wording); kept separate so each verb can supply its own diagnostic." Result: the same bad input ("remote source must include a module") produces different text depending on which entry point the user hit.

**Instances**:
  1. `crates/blit-app/src/endpoints.rs:175-183` — `module_and_rel_path`: errors with `"remote target must include a module path"`. Used by `df`, `du`, `find`, `completions`.
  2. `crates/blit-app/src/admin/rm.rs:48-56` — `extract_module_and_path`: errors with `"remote removal requires module syntax (e.g., server:/module/path)"`. Used by `rm`, all TUI delete paths, TUI F3 pull-move source-delete, and `build_delete_request`.
  3. `crates/blit-app/src/endpoints.rs:96-103` — `ensure_remote_source_supported`: errors with `"remote source must include a module or root (e.g., server:/module/ or server://path)"`.
  4. `crates/blit-app/src/endpoints.rs:86-93` — `ensure_remote_destination_supported`: errors with `"remote destination must include a module or root (e.g., server:/module/ or server://path)"`.

**Canonical**: The variants in `endpoints.rs` (m-3 & m-4) carry the example syntax. The rm-specific one is the most user-friendly. `module_and_rel_path` is the worst (no example).
**Recommendation**: Collapse to one helper returning `(module, rel_path)`. Take the role label (`"source"`, `"destination"`, `"target"`) as a parameter so the error reads `"remote {role} must include a module or root (e.g., server:/module/ or server://path)"`. Three current divergent wordings vanish.

### endpoints-m3 — TUI `prepare_local_transfer` rejects Remote silently lumped with both directions
**Dimension**: Endpoint parsing & classification

F4's local-transfer block parses both endpoints, then rejects `(Endpoint::Remote(_), _) | (_, Endpoint::Remote(_))` with a single error message. But the F1 push trigger (handles the symmetric "local→remote" case) accepts a remote destination and dispatches it. Same Verify-form-shaped data, two screens, contradictory verdicts on a remote destination. There is no UI hint at the Verify form that says "remote dest = use F1, not F4."

**Instances**:
  1. `crates/blit-tui/src/main.rs:4041-4062` — `prepare_local_transfer`: parses both, applies `resolve_destination`, then rejects any Remote with `"F4 transfers only support local→local paths; use the CLI for remote endpoints"`. (Note: the suggested workaround points at the CLI, not F1.)
  2. `crates/blit-tui/src/main.rs:3669-3725` — F1 push branch: accepts `Local src → Remote dst` and dispatches via `f1_push.begin`. The F1 trigger is the TUI's remote-destination entry point.
  3. `crates/blit-cli/src/transfers/mod.rs:205-242` — `select_transfer_route` dispatches all four combinations from a single parse+resolve pair. No "this screen only supports local→local" gate.

**Canonical**: The CLI's single-router pattern (m-3) is the canonical "one parse, four routes" design.
**Recommendation**: F4's Verify form should either dispatch through the same router as F1 (with a "needs F1 confirm gate" hand-off for remote dst), OR the error message should explicitly route to F1: "remote destination requires F1, not the CLI". Pointing the operator at the CLI for a feature the TUI does support on a different screen is a bug.

### endpoints-m4 — TUI's `cancel-endpoint` and `daemons-endpoint-for-row` use bare `RemoteEndpoint::parse` and silently drop None
**Dimension**: Endpoint parsing & classification

For canonical "round-trip a daemon identity back to an endpoint" the TUI silently drops parse failures. The fan-out cancel iterates targets via `RemoteEndpoint::parse(daemon).ok()` — a parse failure means the cancel is silently skipped for that daemon. `endpoint_for_row` returns `Option<RemoteEndpoint>` and the `remote_endpoints()` aggregator simply `filter_map`s — daemons with unparseable addresses become invisible to F2's fan-out subscribe. The CLI's same "parse a remote arg" path errors loudly via `with_context(...)`.

**Instances**:
  1. `crates/blit-tui/src/main.rs:3841` — `cancel-endpoint`: `RemoteEndpoint::parse(daemon).ok()`. Inventory says "None on malformed identity"; caller in `spawn_cancels_for_targets` skips targets whose daemon identity won't parse. No surface to operator.
  2. `crates/blit-tui/src/daemons.rs:335-338` — `endpoint_for_row`: `.ok()` swallowed; if `127.0.0.1:9031` somehow fails to parse, Local row silently has no endpoint; same for discovered IP:port pairs.
  3. `crates/blit-cli/src/jobs.rs:33-34` — `jobs list/cancel/watch`: `RemoteEndpoint::parse(&args.remote).with_context(|| format!("parsing remote endpoint '{}'", args.remote))?` — Err propagates with the input value.
  4. `crates/blit-cli/src/list_modules.rs:7-8` — same `with_context` pattern.

**Canonical**: CLI's error-with-context. Silent drop in the TUI hides operator-visible problems.
**Recommendation**: Round-tripped daemon identities should never fail to re-parse — if they do, that's a bug to surface, not skip. Use `expect("daemon identity must re-parse")` for the round-trip cases, or thread the Err to a status banner. The `127.0.0.1:9031` literal could even be a `const RemoteEndpoint` via lazy_static if the parse function were `const`-able.

## Low severity

### endpoints-l1 — Duplicated "cannot move a module root" message at two TUI dispatch sites
**Dimension**: Endpoint parsing & classification

Same string literal repeated at two dispatch sites that handle different transfer kinds. Inventory flagged this as a smell. If the source-root deletability guard changes (e.g. allows root-but-prompt), both sites must change in lockstep — easy to miss.

**Instances**:
  1. `crates/blit-tui/src/main.rs:3649` — F3 remote→local pull-move: `if kind == PullKind::Move && !is_deletable_remote_path(&source) { return TriggerOutcome::Rejected("cannot move a module root".into()); }`
  2. `crates/blit-tui/src/main.rs:3750` — F1 remote→remote delegated-move: identical string.

**Canonical**: Either string.
**Recommendation**: Hoist to `const CANNOT_MOVE_MODULE_ROOT: &str = "cannot move a module root";` next to `is_deletable_remote_path` in `del_request.rs`, OR return the rejection from `is_deletable_remote_path` as a `Result<(), &'static str>`.

### endpoints-l2 — Diagnostics-dump endpoint snapshot built in two near-identical places
**Dimension**: Endpoint parsing & classification

The CLI's `blit diagnostics dump` and the TUI's F4 diagnostics-dump build essentially the same JSON shape from the same `blit_app::diagnostics::dump` helpers, but the assembly is duplicated. Both call `parse_transfer_endpoint` for src + dst, `resolve_destination`, `endpoint_snapshot`, `endpoint_display`, `source_is_contents`, `dest_is_container`. The TUI's wraps the snapshot in `serde_json::json!{...}` literally identical to the CLI's. A future field added to one will silently diverge.

**Instances**:
  1. `crates/blit-cli/src/diagnostics.rs:160-197` — `run_diagnostics_dump`: assembles the JSON.
  2. `crates/blit-tui/src/main.rs:4267-4305` — `build_diagnostics_snapshot`: same shape, separate assembly. Doc-comment explicitly says "matches CLI behavior" — i.e. it's a manual copy.

**Canonical**: Neither — both call into the same shared helpers but the assembly stays separate.
**Recommendation**: Lift the JSON-assembly into `blit_app::diagnostics::dump::build_snapshot(src, dst) -> serde_json::Value` so the CLI and TUI both call it. Each retains its own write-target (stdout vs. file).

### endpoints-l3 — `default_remote` config-vs-CLI override precedence not symmetric with empty-string semantics
**Dimension**: Endpoint parsing & classification

Inventory `parse-launch-remote` / `resolve-launch-remote-fn`: "Explicit `--remote` (even empty) wins; blank/whitespace config treated as unset." An empty-string `--remote ""` is explicitly honored (and presumably fails parse), while the same empty-string in `[daemon] default_remote = ""` is treated as unset. Same logical input, two different policies depending on entry point. The trade-off is documented but produces user-visible inconsistency: `blit-tui --remote ""` errors, `blit-tui` with `default_remote = ""` silently uses no remote.

**Instances**:
  1. `crates/blit-tui/src/main.rs:637-647` — `resolve_launch_remote`: explicit (even empty) wins; blank config treated as unset.
  2. `crates/blit-tui/src/main.rs:671-684` — `parse_launch_remote`: bad value surfaces `parse_error_message` → F3 banner + F2 Degraded.

**Canonical**: Probably the config-side policy is right (empty is "no default"), but the CLI-side policy lets a typo'd unquoted `--remote` accidentally clear the launch endpoint.
**Recommendation**: Treat empty-after-trim as "unset" symmetrically — printing a hint when `--remote ""` is explicitly passed so the user knows the flag was ignored. The current asymmetry is small but it's a footgun for shell-templated launches (`blit-tui --remote "$REMOTE"` with unset `$REMOTE`).
