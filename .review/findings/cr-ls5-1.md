# cr-ls5-1 — the sweep trusted absence where Windows resolves 8.3 aliases

**Severity**: HIGH
**Status**: FIXED, awaiting re-review (dispatch itself awaits owner go).
**Source**: `ls5-range` codex dispatch over `32486690..7fa18f5b`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree). **`guard_confirmed: true`** for the slice's own guards — this
finding is a hole in the trust boundary, not a vacuous guard.
Record: `.review/results/ls5-range.codex.json`.

## The finding, as returned

> The cache treats every non-exact, non-case-folded name as absent, but
> Windows directory enumeration returns long names while path lookup also
> resolves 8.3 aliases. On this host, `C:\Windows\PROFES~1.XML` resolves to
> `Professional.xml`, yet the cache returned Absent and `destination_needs`
> returned Transfer instead of Skip with `ignore_existing` enabled. A
> cross-platform source file whose legitimate name collides with an existing
> destination short alias can therefore overwrite that file despite
> `--ignore-existing`.

## Why it is admitted

Correct, and reproduced live by the reviewer. ls-5's trust boundary named
case-folding as the one channel by which a sweep-missing name could still
resolve; 8.3 short names are a second channel I did not model. The
consequence is a literal stomp: writing to the alias name replaces the
aliased file, and `--ignore-existing` promised not to. A custom alias set
via `fsutil` need not even contain `~`, so no name-shape heuristic can
whitelist safely.

## Fix

On Windows, a miss beside LISTED entries is the authoritative stat's to
judge (`DirStatLookup::Fallback`), never a trusted absent — only the stat
resolves aliases. The trusted absent survives exactly where no alias can
exist: absent directories (every platform), empty swept directories (an
alias can only resolve to a listed entry), and listed directories on
non-Windows destinations. Cost: one stat per genuinely-new file inside an
existing non-empty destination directory on Windows — noise against
copying that file; the converged-mirror fast path (all names present, all
exact hits) is untouched, confirmed by the unchanged
`a_converged_mirror_answers_from_directory_sweeps` counters.

Tests: `a_windows_miss_beside_listed_entries_is_the_stats_to_judge` pins
the DECISION (auto 8.3 generation is per-volume config a test cannot
assume, so it pins the fallback rule, not the OS alias table);
`an_empty_directory_keeps_the_trusted_absent_on_every_platform` pins the
carve-out; the platform-split rule replaces the old unconditional
trusted-absent assertion.

**Guard proof**: mutating the Windows miss arm back to `Absent` reds both
the new pin and the platform-rule test with `left: Absent / right:
Fallback`; restore verified byte-identical by SHA-256
(`E244BDED…1C59`).

Verified after the change: fmt, clippy `-D warnings` on
x86_64-pc-windows-msvc AND x86_64-unknown-linux-gnu, workspace **1763
passed / 0 failed** (baseline 1754; count grew).
