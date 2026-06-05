# Reopened: audit-l39-m27-env-var-purge

**Reviewer**: gpt (relayed via owner), 2026-06-05.

## Findings

Two issues — both small but stale operator-facing prose, exactly the class of
drift this slice exists to prevent.

### 1. `scripts/bench_remote_remote.sh:20` still says `BLIT_TEST_COUNTER_FILE`

The script's prose comment block above the helper still reads "uses
`BLIT_TEST_COUNTER_FILE`, the same env-gated instrumentation," contradicting
the code change at line 81 that swapped to the CLI flag. Operators reading
the script for context will see the env-var guidance and assume the env-gated
mechanism is current.

**Fix**: update the comment to describe the `--diagnostics-counter-file`
global CLI flag, mentioning the audit-l39 rationale.

### 2. `crates/blit-core/src/remote/instrumentation.rs:13` misdescribes Clap's `hide_short_help`

The module doc says:
> The CLI flag is marked `hide_short_help = true` so it doesn't pollute the
> ordinary `--help` output — diagnostics only.

That's wrong. Clap's `hide_short_help = true` hides the option only from the
**short** help (`-h`), not from the **long** help (`--help`). The flag
**will** appear in `blit --help`. To actually hide from both, use `hide = true`.

**Fix**: pick one:
- (a) Keep the current Clap setting (`hide_short_help = true`) and correct
  the comment to say "hidden from `-h` short help; still appears in
  `--help` so operators can discover it but it's de-emphasized."
- (b) Switch to `hide = true` and keep the comment intent (fully hidden
  from both help levels).

(a) is probably right — fully hiding a diagnostics flag makes it
undiscoverable for operators who need it for troubleshooting; the short-help
hide is the right level of de-emphasis. But the owner's stated principle
("CLI options... probably noted as troubleshooting only in the help") leans
toward (a). Either way, the comment must match the actual Clap behavior.

## Required fixes

1. `scripts/bench_remote_remote.sh:20` — replace `BLIT_TEST_COUNTER_FILE`
   reference in the prose with the new CLI flag.
2. `crates/blit-core/src/remote/instrumentation.rs:13` — correct the
   description of `hide_short_help` (or change to `hide` if full hiding was
   intended). Apply the same fix in `crates/blit-cli/src/cli.rs` if a similar
   comment exists there.

## Validation expected after fix

- `grep -n BLIT_TEST_COUNTER_FILE scripts/ crates/` returns only historical
  references in `.review/findings/` + DEVLOG (acceptable historical record).
- The doc comment in `instrumentation.rs` matches Clap's actual behavior;
  `blit --help` either shows the flag (with the de-emphasized framing) or
  fully hides it, per whichever option is chosen.

## Scope

Same finding, fix-up. Original analysis at
`.review/findings/audit-l39-m27-env-var-purge.md` still applies.
