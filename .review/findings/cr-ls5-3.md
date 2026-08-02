# cr-ls5-3 — taint matched directory SPELLINGS, not directories

**Severity**: HIGH
**Status**: OPEN — assigned to an opus subagent (with cr-a16-1 as a
second, separate commit).
**Source**: `ls6-range` r2 codex dispatch over `ba3fc73a..6c7f67f0`
(reviewer id `parent-alias-taint`). **`guard_confirmed: true`** for
cr-ls5-2's own guards — this is the next hole out, not a vacuous guard.
Record: `.review/results/ls6-range.codex.r2.json`.

## The finding, as returned

> DirStatCache::taint records lexical PathBuf ancestors, while lookup
> checks tainted.contains(dir) using the exact spelling. On Windows, case
> variants and 8.3 directory aliases resolve to the same directory but
> remain different cache keys. An independent probe cached AbsentDir
> through `...\newdirectory`, tainted and created
> `...\NewDirectory\victim.txt`, then still received Absent rather than
> Fallback through the equivalent spelling. A later chunk using an
> alternate parent spelling can therefore trust a stale absence while the
> earlier chunk writes, reopening the overwrite under --ignore-existing
> that cr-ls5-2 was intended to close.

## Why it is admitted

Correct, and it is the third instance of one root error: reasoning about
destination identity through NAMES. cr-ls5-1 was leaf-name aliases,
cr-ls5-2 was snapshot staleness, this is parent-path aliases — every
per-spelling data structure on a case-insensitive, alias-bearing
filesystem has this hole, and canonicalizing is not an answer at verdict
time (the directory may not exist yet, and it costs the round trips the
cache exists to avoid).

## Fix (prescribed)

Stop matching spellings: ONE session-global write taint. The first
transfer verdict of the session sets a flag on `DirStatCache`; from then
on EVERY absence-class answer (swept miss, `AbsentDir`, empty-directory
carve-out) degrades to `Fallback`, in every directory, on every
platform. Listed-entry hits stay valid (pre-write state, judged-once).

Why global is not a perf regression: a CONVERGED mirror issues zero
transfer verdicts — the flag never sets and the 55.57 s path is
byte-for-byte untouched. After the first verdict, absence answers pay
one authoritative stat each — and absence answers belong almost
exclusively to genuinely-new files, whose own copies dwarf one stat.
Per-directory precision bought nothing measurable and carried the
spelling hole; delete it rather than patch it.
