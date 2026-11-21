# cr-uv3-8: Two of the new tests ran only where they were written

**Severity**: MEDIUM — on macOS one test fails before its portable half is asked; on a Linux built against another C library the core crate's tests do not compile
**Status**: Verified — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`, which required no change to it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `d3e6a2ec`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. It completes cr-uv2-5 and cr-uv2-6.

## Evidence
"The Unix round-trip test unconditionally creates `bad\xff.dat`, which APFS rejects before the backslash case runs. Separate the two fixtures and condition the raw-byte case on filesystem support. The Linux character-class test also calls a glibc-only API, preventing test compilation on musl. Scope that comparison to glibc and retain the portable table tests."

Reproduced here for the second half: `cargo check -p blit-core --lib --tests --target x86_64-unknown-linux-musl` failed on `libc::gnu_get_libc_version` (and `libc::LC_ALL_MASK`). The first half cannot be run here: there is no macOS host.

## Intake
Admitted as the reviewer put it.

## What
- `rm.rs::a_name_these_commands_print_is_one_they_take` is two tests over one body (`the_line_find_prints_is_taken`): the name with a `\` in it, which every Unix filesystem takes; and `a_name_that_is_not_text_is_printed_as_one_these_commands_take`, which makes its file first and, where the filesystem refuses the name, says so and asks nothing — as `rm_removes_a_name_that_is_not_text` and the `find` round trip already do.
- `find_name::…::the_classes_are_the_c_librarys_under_c_utf8` is compiled for glibc only (`target_env = "gnu"`). The tables' own guards — `a_class_is_the_reference_locales`, `the_reference_tables_are_whole`, `a_name_matches_as_the_reference_matches_it` — are unchanged and run everywhere.

## Guard proof
- musl: `cargo check --workspace --tests --target x86_64-unknown-linux-musl` — failed before on the two names above, passes after. Compiled, not run.
- Each of the two tests still fails for its own fault, restored byte for byte: the whole word read as an endpoint (cr-uv2-5's fault) → the `\` test red; `df`'s table holding the text nearest the operand → the not-text test red, and the `\` test not.
- This machine: `rm.rs` 16 passed (15 before the split).

## Files changed
- `crates/blit-cli/tests/rm.rs`, `crates/blit-core/src/find_name.rs` — tests only.

## Known gaps
- **Not run on macOS.** That APFS refuses the fixture and the test then asks nothing is the reviewer's observation and this code's reading of it; the macOS target's tests could not even be compiled here (a dependency's build script needs that system's C compiler).
- `find.rs::find_equals_the_system_find`, which makes `bad\xff.dat` without asking, is compiled for Linux only; on a Linux filesystem that refuses such names (ZFS with `utf8only`) it would fail at the fixture.
- Run on Linux only.
