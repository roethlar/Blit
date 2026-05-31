# e-8-config-default-remote: fall back to `[daemon] default_remote`

**Severity**: Feature (Milestone E polish — config)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `bf56a66`

## What

Milestone E lists "JSON config for default endpoints" as polish. Today
the TUI's launch remote comes only from the `--remote` CLI flag; absent
it, the TUI launches mDNS-only. e-8 adds a `[daemon] default_remote`
config key so an operator who always targets the same daemon doesn't
retype it every launch. The CLI flag still wins when present.

## Approach

- `config::DaemonDefaults { default_remote: String }`, added as the
  `daemon` field on `TuiConfig`. `#[serde(default)]` like every other
  section, so older `tui.toml` files (and a fresh install with no
  `[daemon]` section) parse unchanged → empty string → no default.
- `resolve_launch_remote(cli_remote, config_default)` in `main.rs`:
  - CLI flag present → returned verbatim, including a degenerate empty
    string, so the existing parse-error path is unchanged.
  - No flag → the config default when non-blank (trimmed); a
    blank/whitespace value is treated as unset (mDNS-only launch).
- `run_router` resolves the launch remote through this helper, then
  feeds the result through the existing `RemoteEndpoint::parse` path —
  so a bad config-sourced value surfaces the same F2/F3 parse banner as
  a bad CLI flag (no new error surface).

## Files changed

- `crates/blit-tui/src/config.rs`: `DaemonDefaults` + `daemon` field;
  module-doc schema example gains a `[daemon]` section; parse test.
- `crates/blit-tui/src/main.rs`: `resolve_launch_remote`; `run_router`
  uses it for `parsed_remote` / `remote_label`; unit test.

## Tests

593 total (+2):
- `config::daemon_default_remote_parses_and_defaults_empty` — absent
  section → empty; `[daemon] default_remote = "nas:9444:/m/"` parses.
- `resolve_launch_remote_prefers_cli_then_config` — CLI wins (incl.
  empty flag); config fallback when non-blank; blank/whitespace → None;
  config value is trimmed.

The launch wiring itself (config → `parsed_remote`) is exercised at
runtime; the resolution + parse pieces are unit-tested independently.

## Scope

Default *remote endpoint* only. The CLI-flag precedence and the
mDNS-only default are preserved exactly; this only fills the
no-flag gap. Other Milestone E items (key remapping, dark/light theme
presets, Prometheus bridge) remain separate.

## Reviewer comments

(empty — pending grade)
