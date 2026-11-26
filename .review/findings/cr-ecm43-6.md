# cr-ecm43-6: the message-quoting guard ran whatever `du`, `rm` and `df` are installed as those names

**Severity**: MEDIUM — the guard fails where those names are not GNU's (CI's `ubuntu-26.04`: uutils). GitHub CI run 45731868559.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Use the verified GNU reference across the utility comparisons. The `ls` fix selects and names its reference correctly. The existing diagnostic test still invokes bare `du`. On pinned Linux CI it compared two different renderings of byte `FF` and failed. Reuse the GNU selection and canonical invocation name there."

GitHub CI run 45731868559 (`34c77a48`), Linux: `du "bad\u{fffd}.dat"` — blit `du: cannot access 'bad'$'\377''.dat': …`, uutils `du: cannot access $'bad\xFF.dat': …`. The test had not run on CI before: cargo stops at the first failing test binary, and `ls.rs`, before it, had failed every run.

## Intake
Admitted as written, for the three coreutils commands the guard calls (`rm`, `du`, `df`); `find` is findutils.

## What
`a_name_in_a_message_is_written_as_the_system_commands_write_it` calls GNU's `rm`, `du` and `df` wherever they are installed (`common::gnu_coreutils`, cr-ecm42-5's), each called by its own name (`arg0`): GNU names itself by what it was called. Where one of them is not there, it compares nothing and says so.

## Guard proof
- Green with GNU's as `du` (this host), and with CI's layout made here — a `du` of another implementation first on the `PATH`, GNU's as `gnudu`.
- Mutations, restored byte for byte, each → red with CI's layout:
  - the reference taken as installed under its bare name (the run before the fix);
  - GNU's not called by its own name (`gnudu: cannot access …`).
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/rm.rs` — the guard.

## Known gaps
- The other system comparisons (`du_df.rs`, `find.rs`, `rm.rs`'s own `rm` runs) passed against uutils on CI and are unchanged.
