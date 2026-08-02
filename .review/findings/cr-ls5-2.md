# cr-ls5-2 — session-stale absent snapshots re-open the alias overwrite

**Severity**: HIGH
**Status**: OPEN — fix assigned to an opus subagent (owner instruction:
coding via opus/sonnet subagents), queued behind the audit-16 tree lock.
**Source**: `ls6-range` codex dispatch over `7fa18f5b..ba3fc73a`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached,
disposable worktree). **`guard_confirmed: true`** — this round also
VERIFIED-CLOSED cr-ls5-1.
Record: `.review/results/ls6-range.codex.json`.

## The finding, as returned

> The 8.3 fix still trusts session-lifetime AbsentDir and empty-directory
> snapshots. Because local apply runs concurrently while later 128-entry
> chunks are still being diffed, an earlier chunk can create a long-name
> file and its 8.3 alias after the snapshot; a later source entry using
> that alias is still reported Absent rather than sent to authoritative
> stat, so it is planned and can overwrite the earlier file, including
> under --ignore-existing. A Windows probe using a real custom short name
> reproduced the stale Absent verdict.

## Why it is admitted

Correct. cr-ls5-1's fix reasoned about aliases of LISTED entries and kept
the trusted absent where "nothing can be aliased" — absent and empty
directories — but that judgment was made against the sweep-time state,
and the session's own apply invalidates it: verdicts for chunk N are
complete before its payloads write, while chunks N+1… diff concurrently
with those writes. A directory that was absent or empty at sweep time can
hold freshly written files (and their 8.3 aliases, and their case
variants — the folded-name set has the same staleness) by the time a
later chunk's entry is judged. The same-name case is safe on every
platform (each name is judged exactly once, before it is ever written);
the ALIAS and CASE-FOLD channels are not, because a write to one name
creates a different resolvable name.

## Fix (prescribed)

Taint-on-verdict: the moment the diff produces a Transfer verdict for a
path, taint every ancestor directory of that path in the
`DirStatCache`. In a tainted directory, ABSENCE judgments (trusted
absent from a swept listing, `AbsentDir`, or the empty-directory
carve-out) degrade to `Fallback` — the authoritative stat resolves
aliases and case the way the filesystem does. Listed-entry HITS remain
valid: a hit describes the file's own pre-write state, which is the
correct comparison state because every file is judged before it is
written. Verdicts strictly precede writes (chunk N's needs are batched
after its diff completes), so taint-at-verdict is conservative by
construction. All platforms taint — the case-fold channel exists on any
case-insensitive destination, not just Windows.

Perf shape: a converged mirror issues zero Transfer verdicts, so nothing
taints and the 55.57 s path is untouched. A fresh copy pays one stat per
new file in already-written-to directories — bounded by the write count
and dwarfed by the writes themselves.
