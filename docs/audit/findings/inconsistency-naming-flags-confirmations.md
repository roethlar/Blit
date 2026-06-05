# Inconsistency Findings: Naming, flags, and destructive-op confirmation prompts
**Generated**: 2026-06-04
**Findings**: 13 (H: 3, M: 6, L: 4)

## High severity

### CFM-01 — `perf history clear` confirms in TUI but fires silently in CLI
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/main.rs:4448-4481` (`handle_profile_clear_confirm_keystroke`), `crates/blit-tui/src/profile.rs:39-46,100-112` (`begin_clear_confirm`), `crates/blit-tui/src/screens/f4.rs:347-349` — TUI F4 `c` to clear performance history opens a modal `clear ALL local performance history? this is permanent · [y / N or Esc]` (d-66). Permanent action, gated like every other TUI destructive op.
2. `crates/blit-cli/src/diagnostics.rs:25-30` — `blit diagnostics perf --clear` fires `perf::clear()` immediately without any prompt. No `--yes`/`-y` flag exists on `PerfArgs` because no prompt exists to skip. `cli.rs:175-184` shows `PerfArgs` has only `--enable`/`--disable`/`--clear`/`--limit`/`--json`.

The same destructive action (wipe `perf_local.jsonl`) is "always-confirm" in the TUI and "never-confirm" in the CLI. A muscle-memory CLI user dropping into the TUI is surprised by a modal; a TUI user dropping into a script is surprised that automation wipes history without a flag. The CLI plan (`docs/audit/inventory/plan-cli.md:386,464`) states "destructive operations prompt unless `--yes` is supplied" — `--clear` here violates that principle.

**Canonical**: TUI's posture is correct under the plan's stated principle (confirm by default, allow opt-out). The CLI should adopt the same posture.
**Recommendation**: Add a confirmation prompt to `blit diagnostics perf --clear` with a `--yes`/`-y` opt-out matching `mirror`/`move`/`rm`. Prompt text could mirror the TUI: `clear all local performance history? this is permanent.`

---

### CFM-02 — `clear-recent` exists only in TUI; CLI has no surface for it (and the TUI clear is unconditional)
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/main.rs:1788-1810` — TUI F2 `E` ClearRecent: arms `F2CancelStatus::ConfirmingClearRecent`, on `y` calls local `transfers.clear_recent` + fans `ClearRecent` RPC to every watched daemon. Hardcoded behavior; no `[transfer]` config knob to skip.
2. `crates/blit-app/src/admin/jobs.rs:107-118` — `clear_recent(&RemoteEndpoint)` library function exists.
3. `crates/blit-cli/src/` — exhaustive search for `clear_recent` / `ClearRecent` returns zero hits. The CLI has no `blit jobs clear-recent` (or similar) verb. The proto `ClearRecentRequest` (`proto/blit.proto:845-849`) is wired up daemon-side but unreachable from the CLI.

The TUI surfaces an irreversible cross-daemon fan-out action (wipes the recent ring on N daemons) with no CLI parallel. Operators automating cleanup must call the library or write their own gRPC client. Worse, the TUI clear is unconditional (no `confirm_cancel`-style opt-out), while every other destructive TUI op has either an unconditional confirm OR a config gate — `clear-recent` doesn't even have the opt-in style of `confirm_cancel`. Either it should match the `--yes` style with a CLI verb, or it should match `confirm_cancel`'s opt-in style (with a default-on confirm), and both should be consistent.

**Canonical**: There should be a `blit jobs clear-recent <REMOTE>` CLI verb with `--yes`. TUI prompt and CLI prompt should both use the same y/N vocabulary.
**Recommendation**: Add `blit jobs clear-recent <REMOTE> [--yes] [--json]` with the same `confirm_destructive_operation` helper used by `mirror`/`move`. Update TUI to call the same display message wording. Plan doc should be updated to list it.

---

### CFM-03 — F2 cancel confirm is config-gated in TUI; CLI cancel has no confirm and no `--yes` flag at all
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/main.rs:1747-1772` — TUI F2 `K` (single) and `1873-1897` (`X` batch) check `tui_config.transfer.confirm_cancel`. If `confirm_cancel = true` in `tui.toml`, K transitions to `Confirming` and shows `cancel <id>? y/N`; otherwise fires Cancel RPC immediately. Default `false` (`config.rs:613`).
2. `crates/blit-cli/src/cli.rs:138-147` (`JobsCancelArgs`) — only `remote`, `transfer_id`, `--json`. No `--yes`. `crates/blit-cli/src/jobs.rs:45-54` (`run_jobs_cancel`) fires `jobs::cancel(...)` unconditionally; no prompt path exists.

The "is cancel destructive enough to confirm?" question got two answers: TUI says "operator can opt in to confirmation"; CLI says "no, ever." A user who set `confirm_cancel = true` in TUI and then runs `blit jobs cancel HOST ID` in a script gets the unguarded behavior — and there's no `--yes` to even document the opt-in/opt-out boundary. This becomes a higher problem when scripted batch cancels are wanted (e.g. `for id in ids; do blit jobs cancel ...`).

**Canonical**: Cancel is recoverable (transfers are resumable) so the no-confirm default is reasonable. But the CLI should offer the SAME opt-in surface the TUI offers, so muscle memory and tooling can align. Either add `--confirm` flag to CLI that mirrors the config knob, or add a `[transfer] confirm_cancel` config CLI also reads.
**Recommendation**: Add `--confirm` to `JobsCancelArgs` (default off) so CLI users can opt in to the same y/N gate, gated by the same `confirm_destructive_operation` helper. Document the symmetry in the plan-cli inventory.

---

## Medium severity

### CFM-04 — F4 confirm phrasing uses `[y / N or Esc]`; F1/F2/F3 and CLI use bare `y/N`
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/screens/f4.rs:244-251,347-349` — F4 destructive prompts render as `mirror will DELETE extraneous files at destination · [y / N or Esc]`, `move will DELETE the SOURCE after copy · [y / N or Esc]`, `clear ALL local performance history? this is permanent · [y / N or Esc]`. Spaces around `/`, square brackets, includes "or Esc".
2. `crates/blit-tui/src/screens/f1.rs:184` — F1 trigger renders `{mode} {src} → {dst}? {detail} y/N`. Bare `y/N`.
3. `crates/blit-tui/src/screens/f2.rs:336,343,350` — F2 renders `cancel {id}? y/N`, `cancel {count} transfers? y/N`, `clear recent? y/N`. Bare `y/N`.
4. `crates/blit-tui/src/screens/f3.rs:416,489` — F3 renders `{verb} → {dest}? {detail} y/N` and `delete {label}? y/N`. Bare `y/N`.
5. `crates/blit-cli/src/transfers/mod.rs:93` — CLI renders `{message} [y/N]: `. Square brackets, NO spaces, NO "or Esc".

Three distinct prompt-text styles for the same y/N question across the same product:
- F4: `[y / N or Esc]` (verbose, hints at Esc, spaces)
- F1/F2/F3: `y/N` (bare)
- CLI: `[y/N]:` (rsync-style)

A user reading docs/screenshots sees inconsistent UI language. The F4 "or Esc" hint is genuinely informative, but only appears on F4 — the other TUI surfaces also accept Esc and don't say so.

**Canonical**: TUI uses `y/N` (with optional `· y/N or Esc` detail for new operators); CLI keeps rsync-style `[y/N]:`.
**Recommendation**: Pick ONE TUI phrasing and apply it to F1/F2/F3/F4 footers. The F4 "or Esc" affordance is operator-friendly — either propagate it to all four screens or drop it for consistency. Bare `y/N` everywhere (matching F1/F2/F3) is the simpler unification.

---

### CFM-05 — `confirm_destructive_operation` is duplicated inline in `rm.rs`
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:87-99` — helper `confirm_destructive_operation(msg, skip_prompt)`: prints `{msg} [y/N]: ` to stdout, reads stdin, accepts `y` / `yes` after trim+lowercase. Used by `mirror` (line 186) and `move` (line 415).
2. `crates/blit-cli/src/rm.rs:48-58` — reimplements identical logic inline: `print!("Delete {} on {}? [y/N]: ", ...)`, stdout flush, stdin read, trim+lowercase, accepts `y`/`yes`. Same vocabulary as the helper, but a separate code path.

Risk: divergence over time. If `confirm_destructive_operation` is later changed to accept `Y` only (or to route prompts to `/dev/tty` instead of stdout), `rm.rs` doesn't track. Already the prompt text format diverges slightly: helper formats as `{msg} [y/N]: ` (caller supplies the message ending with `?`); `rm.rs` interleaves variables differently but ends in the same `? [y/N]: ` shape.

**Canonical**: Single shared helper.
**Recommendation**: Have `rm::run_rm` call `confirm_destructive_operation` (move it to `blit_cli::shared` or `blit_app::common`).

---

### CFM-06 — Confirm prompt writes to stdout, not stderr/`/dev/tty`
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:93-94` — `print!("{} [y/N]: ", message); io::stdout().flush()?;` writes prompt to stdout.
2. `crates/blit-cli/src/rm.rs:49-50` — same pattern.
3. `crates/blit-cli/src/transfers/mod.rs:194-203, 420-428` — meanwhile, the "starting copy SRC -> DST" banner is sent to stderr (line 196: `eprintln!`). So the CLI has the policy "info goes to stderr, summary/JSON goes to stdout" but the confirm prompt sits on stdout, contradicting it.

If a user accidentally pipes `blit mirror src dst | tee log`, the y/N prompt is interleaved with summary on stdout (and the read-from-stdin would block on tee's input). Most CLIs (git, rsync) route prompts to stderr or `/dev/tty`.

**Canonical**: Prompt → stderr (or `/dev/tty`), summary/JSON → stdout.
**Recommendation**: Switch `print!` in `confirm_destructive_operation` and `rm.rs` to `eprint!` (mirror the banner pattern), or open `/dev/tty` directly.

---

### CFM-07 — TUI `ConfirmingMirror`/`ConfirmingMove` split vs F3's single `Confirm { kind }` variant
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/transfer.rs:46-81` — F4 local transfer state machine: two distinct enum variants `TransferStatus::ConfirmingMirror` and `TransferStatus::ConfirmingMove`. Accessors: `is_confirming_mirror()` / `is_confirming_move()` / `begin_confirm_mirror()` / `begin_confirm_move()`.
2. `crates/blit-tui/src/f3pull.rs:88-97` — F3 remote pull state machine: ONE variant `PullStatus::Confirm { dest, dest_root, kind }` where `kind: PullKind`. One accessor `is_confirming_destructive()` / `confirm_destructive()` / `cancel_destructive()`.
3. `crates/blit-tui/src/f1trigger.rs:48-72` — F1 trigger uses a `confirming: bool` flag inside `Editing` (yet another shape — a flag, not a variant).

Three different state-machine shapes for "I'm awaiting y/N for a destructive transfer kind." If a new kind (e.g. `Sync`) is added, F3 needs no shape change (just add to PullKind), but F4 needs a 3rd variant + 3rd accessor + 3rd dispatcher arm. The asymmetric design accumulates maintenance debt; the `code-tui-state.md` inventory already calls it out as a smell.

**Canonical**: Either F3's `Confirm { kind }` or a sibling-trait abstraction. Single-variant + `kind` is the right shape; it matches PullKind/TransferKind enum design.
**Recommendation**: Refactor `TransferStatus` to `Confirming { kind: TransferKind }` and unify accessors. Same applies to `F1TriggerStatus::Editing { confirming }` — promote `confirming` to a proper Confirm variant or to a `TransferStatus::Confirm` reuse.

---

### CFM-08 — TUI `TransferMirrorConfirm` user-action is overloaded across F4/F2 paths (and the name is wrong for some)
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/main.rs:5042` — `UserAction::TransferMirrorConfirm` declared.
2. `main.rs:5359-5360` — `Y`/`y` keystroke maps to `TransferMirrorConfirm`. The action is the "yes" answer regardless of what's pending.
3. `main.rs:1797-1856` — F2 dispatcher: `TransferMirrorConfirm if app.cancel_status.is_confirming()` handles `y` for cancel (single + batch + clear-recent). The action name "MirrorConfirm" has nothing to do with mirror; it's pressed during a CANCEL confirm.
4. `main.rs:2172-2207` — F4 dispatcher: `TransferMirrorConfirm if app.transfer.is_confirming_mirror()` (line 2172) AND `TransferMirrorConfirm if app.transfer.is_confirming_move()` (line 2196). Same action key both for mirror and for MOVE confirm — name is wrong for `move`.

The action name was tied to the FIRST destructive prompt ever added (d-4 mirror). Subsequent prompts (move, cancel, batch cancel, clear recent) reused the same action because semantically all of them want "y means yes." But the type-level name lies about what the action means.

**Canonical**: Rename to `ConfirmYes` or `DestructiveYes`.
**Recommendation**: Rename `UserAction::TransferMirrorConfirm` → `UserAction::ConfirmYes` (and the partner `TransferCancel` → `ConfirmNo` would also help). The bind in `key_action` doesn't change, only the type. Cleanly separates "y/N answer" from "what's being confirmed."

---

### CFM-09 — `--delete-scope` is stringly-typed; clap is case-sensitive, internal compare is case-insensitive
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-cli/src/cli.rs:247-248` — clap definition: `value_parser = ["subset", "all"]`. Case-sensitive — `blit copy --delete-scope ALL` is rejected by clap.
2. `crates/blit-cli/src/cli.rs:386-388` — `delete_scope_all()`: `self.delete_scope.eq_ignore_ascii_case("all")` — case-insensitive consumer.

A future change that lifts the clap value_parser (e.g. enum migration) could accept `All` from a config file or env, and the code would happily consume it — but today it can't reach the consumer because clap blocks at parse. Today's behavior: `--delete-scope all` works; `--delete-scope All` errors at clap; `--delete-scope ALL` errors at clap. The internal accept-anything-case is dead but misleading.

**Canonical**: Use a proper enum (`#[derive(ValueEnum)]`) so case handling is uniform.
**Recommendation**: Define `enum DeleteScope { Subset, All }` with `ValueEnum`; drop the `String` field and the case-insensitive helper.

---

## Low severity

### CFM-10 — Module-name validation is non-empty-only; nothing rejects `/` or control chars
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-daemon/src/runtime.rs:243-245` — `module.name.trim().is_empty()` is the only validation. Whitespace-trimmed empty is rejected; anything else is accepted (including `foo/bar`, `..`, unicode control chars, spaces).
2. `crates/blit-core/src/remote/endpoint.rs:73-76` — `RemoteEndpoint::parse` splits on the first `/` after the module name. A module named `foo/bar` in the daemon config would be impossible to address from the CLI (the parser would see `foo` as module, `bar/...` as rel_path). The daemon would advertise the module name; the CLI would silently address the wrong path.

The daemon and the parser have different implicit assumptions about valid module names. The daemon accepts anything non-empty; the wire protocol assumes the module name doesn't contain `/`.

**Canonical**: Reject `/`, `..`, control chars, whitespace inside module names at daemon-config-load time.
**Recommendation**: Add a `validate_module_name(&str)` helper to `blit_core` (alongside `validate_wire_path`); use it both at daemon config load and at any callsite that constructs a module spec.

---

### CFM-11 — TUI `[keys]` field naming uses `pane_f1`/`pane_f4` but plan doc says "pane_f1"/"pane_f2"; movement keys use directional names (`move_down`)
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-tui/src/config.rs:107-110,120-123` — `pane_f1`, `pane_f2`, `pane_f3`, `pane_f4` (F-key-indexed); `move_down`, `move_up`, `move_top`, `move_bottom` (direction-indexed).
2. `crates/blit-tui/src/config.rs:50-58` (comment block in module doc) — agrees with the field names, lists both styles. So self-consistent within `config.rs`.

Each name is reasonable in isolation, but they don't share a discipline. `pane_f1` mixes "what's the destination" (pane) with "what's the key it traditionally maps to" (F1). If F-keys F1-F4 are remapped to different panes later, the key name `pane_f1` would no longer mean "the key that goes to F1" — it'd mean "key 1 of 4." Cleanest would be `pane_1`/`pane_2`/... (parallel to movement directions) or `f1_pane`/`f2_pane` (parallel to "F1 maps to pane 1").

**Canonical**: No clear canonical. Either `pane_1/2/3/4` (numeric) or `f1_pane/f2_pane` (key-named).
**Recommendation**: If breaking config compatibility is on the table, rename to `pane_1`/`pane_2`/.../`pane_4` for parallel structure with movement. Otherwise document the asymmetry in a `[keys]` schema comment.

---

### CFM-12 — `list-modules` uses `RemoteEndpoint::parse` directly; every other admin verb uses `parse_endpoint_or_local`
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-cli/src/list_modules.rs:7-9` — `RemoteEndpoint::parse(&args.remote)?` direct call.
2. `crates/blit-cli/src/du.rs:9-17`, `df.rs:8-16`, `find.rs:9-17`, `rm.rs:14-22`, `ls.rs:57-59`, `completions.rs:50-58` — all use `parse_endpoint_or_local` and branch on `Endpoint::Local` to emit a friendly "this verb is remote-only" message.
3. `crates/blit-cli/src/jobs.rs:33-34,46-47,175-176` — also use `RemoteEndpoint::parse` direct. Documented as intentional (verbs are remote-only) but inconsistent with the friendlier admin-verb pattern.

The error messages differ between the two patterns. `list-modules` shows the lower-level "remote location cannot be empty"-style parser error; `du`/`df`/`find`/`rm` show `\`blit du\` only supports remote paths (received local path: X)`. A user typing `blit list-modules /tmp` sees a less helpful error than `blit du /tmp`.

**Canonical**: All admin verbs use `parse_endpoint_or_local` and branch on Local with a per-verb message. Apply consistently.
**Recommendation**: Convert `list-modules` and `jobs list/cancel/watch` to `parse_endpoint_or_local`. Standardize the "this verb is remote-only" message format.

---

### CFM-13 — `--null` rejection cites "remote" in error but actually rejects ANY non-local endpoint and ANY mirror mode in subtle text
**Dimension**: Naming, flags, and destructive-op confirmation prompts
**Instances**:
1. `crates/blit-cli/src/transfers/mod.rs:131-142` — `--null` with `mirror` mode bails: long verbose message about destination-purge.
2. `crates/blit-cli/src/transfers/mod.rs:143-154` — `--null` with any remote endpoint bails: long verbose message about silently-ignored writes.
3. `crates/blit-cli/src/transfers/mod.rs:251-253` — `move --dry-run` bails with a TERSE single-line `"move does not support --dry-run"`.
4. `crates/blit-cli/src/transfers/mod.rs:323-331` — `move --null` bails with a LONG paragraph about null-sink + source-delete.

Reject-flag messages have wildly different verbosity (single-line terse vs 7-line essay). The data-loss-class rejections all bail with long explanations; the non-data-loss rejections (like `move --dry-run`) bail tersely. The plan doc principles don't prescribe a length, but a user faced with bumpy verbosity reads it as a quality signal.

**Canonical**: A consistent "verbose rejection message with example remediation" or "terse rejection with one-line reason." Pick one for data-loss-class rejections; either is defensible but mixed is not.
**Recommendation**: Adopt the data-loss-class verbose style for ALL reject-gates in `run_transfer` / `run_move` (currently it's used for ~6 of 8 gates). Specifically lengthen the `move --dry-run` message to explain why dry-run+move can't make sense (the source-delete step needs the transfer to actually happen).
