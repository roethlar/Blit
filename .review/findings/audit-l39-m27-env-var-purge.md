# audit-l39 + audit-m27 — Env-var purge for app + diagnostic config

**Source**: 2026-06-04 audit chain, R3 findings **L39** + **M27**.
**Owner directive (2026-06-04)**: env vars are out for app and diagnostic configuration.
CLI options only, sparingly added. Diagnostics are fine when explicitly flagged as
diagnostics-only in help.

## What

Three env-var violations existed in the shipped binary, plus two documented-but-
nonexistent env-var overrides in `greenfield_plan_v6.md`. All purged in this slice.

**Removed from shipped binary**:
1. `BLIT_TUI_INPUT_TRACE` (`crates/blit-tui/src/main.rs:4917`) — was reading the env
   var and writing to a hardcoded `/tmp/blit-tui-input.log` path. Now a CLI flag
   `--trace-input PATH` with caller-supplied path, `hide_short_help=true`.
2. `BLIT_TEST_COUNTER_FILE` (`crates/blit-core/src/remote/instrumentation.rs:10-22`) —
   was env-gated diagnostics that the integration tests and the production bench
   script depended on. Now a global CLI flag `--diagnostics-counter-file PATH`,
   `hide_short_help=true`, explicitly marked diagnostics-only in help. The
   instrumentation module is now `pub` (was `pub(crate)`) so `blit-cli`'s `main`
   can call `set_counter_path`.
3. (no third runtime env var existed; R3 listed two)

**Removed from plan docs** (R3-M27):
4. `greenfield_v6 §1.2 line 161`: `BLIT_FORCE_GRPC_DATA=1` mention struck. Text now
   says any future locked-down override will be a CLI flag (e.g. `--force-grpc-data`),
   sparingly added, diagnostics-only, with explicit "no env-var form" citing
   audit-l39.
5. `greenfield_v6 §1.3 line 167`: `BLIT_DISABLE_LOCAL_TELEMETRY=1` mention struck.
   Same treatment.
6. `REMOTE_REMOTE_DELEGATION_PLAN.md:898-909`: updated to reference the new
   `--diagnostics-counter-file` flag instead of the env var.

## Approach

Mechanical swap, no behavior change:
- `BLIT_TUI_INPUT_TRACE` → `--trace-input PATH`. Args struct gains `trace_input:
  Option<PathBuf>`; `spawn_input_task` takes the path as a parameter rather than
  reading env.
- `BLIT_TEST_COUNTER_FILE` → `--diagnostics-counter-file PATH`. `Cli` struct gains
  the global flag. `main` calls `blit_core::remote::instrumentation::set_counter_path`
  at startup before any RPC. Instrumentation module promoted to `pub`; a `OnceLock
  <PathBuf>` replaces the `std::env::var` read. The instrumentation crate's `record`
  is unchanged except for the path source.
- Tests at `crates/blit-cli/tests/remote_remote.rs` swap the env-var setup for the
  global CLI flag (inline test + the `run_blit` helper). The flag must appear
  **before** the subcommand because it's `#[arg(global = true)]`.
- Bench script `scripts/bench_remote_remote.sh:81` same swap.

The flag's hidden status (`hide_short_help = true`) keeps it off the standard
`--help` output. Operators reading `blit --help` won't see it; integration tests
and the bench script know it exists.

## Files changed

- `crates/blit-cli/src/cli.rs` — add `--diagnostics-counter-file` global flag with
  hidden-from-short-help.
- `crates/blit-cli/src/main.rs` — destructure + install via `set_counter_path`.
- `crates/blit-cli/tests/remote_remote.rs` — replace `.env(...)` with CLI flag
  in both the inline test and `run_blit` helper.
- `crates/blit-core/src/remote/instrumentation.rs` — full rewrite; `OnceLock` +
  `set_counter_path`; module doc updated.
- `crates/blit-core/src/remote/mod.rs` — `pub(crate) mod` → `pub mod`.
- `crates/blit-tui/src/main.rs` — `--trace-input` flag, `spawn_input_task` takes
  the path arg, doc-comment updated.
- `docs/plan/greenfield_plan_v6.md` — strike both env-var overrides + add the
  "no env-var form" prohibition.
- `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` — update observable-mechanism
  reference.
- `scripts/bench_remote_remote.sh` — swap env for `--diagnostics-counter-file`.

No new code paths, no new dependencies.

## Tests added

None (mechanical swap). The existing integration tests at
`crates/blit-cli/tests/remote_remote.rs` exercise both the test counter path
and the relay-vs-direct byte assertions; they continue to pass with the CLI flag
swapped in. The integration suite's coverage is the regression test for this
slice — any path that constructs `DataPlaneSession` outbound bytes or the
`RemoteTransferSource` relay primitive still records into the counter file when
the flag is present.

Workspace validation suite green: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`.

## Known gaps

- **DEVLOG.md / REVIEW.md history** mentioning `BLIT_TEST_COUNTER_FILE` and
  `BLIT_TUI_INPUT_TRACE` is left as-is — historical record of when the env vars
  existed. No follow-up needed.
- **The `tui-key-dispatch-press-only-filter` finding doc** at
  `.review/findings/tui-key-dispatch-press-only-filter.md` references the old
  env var in its prose. Historical artifact; superseded by this slice's CLI
  flag. Not edited to avoid touching settled review records.
- **No tests** that directly exercise `--trace-input` itself. The flag is a
  diagnostics-only path; adding a test would require either a fake terminal
  source (heavy) or trusting the env-swap mechanical equivalence. The pre-
  existing `tui-key-dispatch-press-only-filter` test still validates the input
  pipeline's correctness end-to-end.

## Cross-references

- R3 finding L39 (TUI input trace + test counter file env vars).
- R3 finding M27 (documented-but-nonexistent override env vars).
- Owner directive 2026-06-04: "I abhor env vars for app settings. NO. if we
  need these options, they need to be cli options. probably noted as
  troubleshooting only in the help. that was the standard I tried to set
  before drift. cli options only. SPARINGLY ADDED. diagnostics are fine, but
  they need to be explicitly flagged as for diagnostics only."
- Memory `audit-owner-decisions` — extends with the no-env-vars rule.
