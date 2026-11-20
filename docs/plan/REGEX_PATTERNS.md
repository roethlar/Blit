# Regular-expression patterns: `--exclude-regex` and `--include-regex`

**Status**: Draft (r1, 2026-11-20 — drafted from the owner's question of that day and the evidence gathered for it; no owner interview yet, every design choice below is the drafting agent's recommendation)
**Created**: 2026-11-20
**Supersedes**: nothing
**Decision ref**: pending (activation needs a `docs/DECISIONS.md` entry)

## Goal

A user who thinks in regular expressions can say what to leave out, or what alone to take, as a regular expression — on the command line and in a file — beside the glob patterns blit has today. Which language a pattern is in is said by the option that carries it and is never guessed from its text.

## Owner requirements (verbatim)

- 2026-10-21, on pattern files: every entry "needs to be exact or a [regex]" (the plan of that day read this as globs).
- 2026-11-20, asked to confirm that reading: "why can't we support regex in both?" — and, on the answer that nothing prevents it but it is a second pattern language with its own rules: "a" (globs in `docs/plan/EXCLUDE_CASE_MATCHING.md`, regular expressions in a plan of their own; D-2026-11-20-1, that plan's Q7).

## Prior art (measured 2026-11-20 on Linux host `continuum` unless marked)

One tree: `a.jpg`, `axjpg`, `my.a.jpg.bak`, `sub/a.jpg`, `sub/deep/b.JPG`, `Cache/x.tmp`, `notes.txt`.

- **One text, two meanings.** Read as a regular expression and searched for, `a.jpg` matches `a.jpg`, `axjpg` and `my.a.jpg.bak`; `*.jpg` is a parse error (ripgrep, the Rust regex engine family). A pattern's language cannot be told from its text.
- **GNU find 4.11.0 `-regex`** matches the WHOLE path, start point included: `-regex 'a.jpg'` finds nothing; `'.*a.jpg'` finds `./a.jpg`, `./axjpg`, `./sub/a.jpg`; `'.*/a\.jpg'` finds the two real ones. Case is a second test, `-iregex` (`'.*\.jpg'` then also finds `b.JPG`). Its glob test is a separate one (`-name`).
- **rclone 1.76.0** keeps globs and marks a regular expression inside the pattern, `{{…}}`: `--include '{{a.jpg}}'` lists `a.jpg`, `axjpg`, `sub/a.jpg` — tied to the END of the path and to the start of any component, as its manual says (`(^|/)(regexp)$`); a leading `/` ties it to the root; case is ignored by `(?i)` inside the expression, or for globs by its own `--ignore-case`.
- **grep** excludes files by `--exclude=GLOB`: a glob even in the tool whose business is regular expressions.
- **rsync and robocopy** have no regular expressions (rsync: read from its filter source, 2026-10-21; robocopy: its reference).
- Recalled, not installed here and not measured: lftp's `mirror -x RX` beside `-X GLOB`, duplicity's `--exclude-regexp`, borg's `re:` pattern prefix, unison's `Regex` ignore form. All name the language explicitly, by option or by marker.

## Non-goals

- Guessing. No pattern is ever read as a regular expression because of how it looks.
- A marker inside a pattern (`{{…}}`, `re:`): per-pattern syntax was rejected on 2026-10-21 as knowledge most users do not have. The language is said by the option.
- `move`: it takes no pattern of any kind.
- `find`: it follows Unix find (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`), where `-regex` would be its own addition.
- Look-around and back-references: the engine blit already links does not have them, by design — it guarantees time linear in the name's length.

## Constraints

- **SIMPLE**: the owner asked for this capability; it still adds options, so it adds as few as answer the question (Q1).
- **FAST**: one compile per expression; the engine is linear-time; nothing is added to a run that gives no expression.
- **RELIABLE**: an expression that does not parse refuses the run before anything is read. One reading per run: the source walk, the deletion scope and `check` match the same compiled expression against the same text. An expression that cannot be carried to a peer is refused, never dropped.
- **Same build only**: new wire fields, a contract bump.
- **Platform truth**: an expression is matched against the same text on every OS (Q2), so its guards run everywhere.

## Acceptance criteria

- [ ] To be written once Q1 to Q6 are ruled.

## Design (sketch, pending the rulings)

- `FileFilter` gains compiled regular expressions beside its globs; an entry is excluded when a glob OR an expression names it, and passes the include test when a glob OR an expression includes it.
- A matched directory is pruned with everything below it, as with a glob.
- The deletion scope reads an expression as written against the destination's relative paths; what the source's walk pruned still reaches the destination through the pruned-path list, as for a pure glob.
- `--ignore-case` compiles the expression case-insensitively — the engine's own Unicode simple case folding, the rule `--ignore-case` already means (`docs/plan/EXCLUDE_CASE_MATCHING.md` Q1).
- Wire: new repeated fields on `FilterSpec`; contract bump. Records: the job spec keeps them.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop.

1. To be cut after the rulings: local matching and options; the wire; files; records and words.

## Open questions

- **Q1 — how the language is said.** (a) Two options, `--exclude-regex RX` and `--include-regex RX`, repeatable, and two for files, `--exclude-regex-from FILE` and `--include-regex-from FILE` [recommended: a file is all globs or all expressions, said by the option that names it]; (b) the two command-line options only, no file form; (c) a marker inside a pattern, as rclone does — rejected in kind on 2026-10-21. — owner
- **Q2 — what an expression must match.** (a) The WHOLE path below the source root, `/`-separated on every platform, as find's `-regex` does [recommended: an expression that names more than was meant deletes nothing it should not under `mirror`, but leaves files out of a copy in silence, and `cache` as a search names `my_cached_files`; whole-path matching makes the user write `.*cache.*` to mean that]; (b) a search anywhere in the path, as grep does; (c) rclone's rule, tied to the end and to the start of a component. — owner
- **Q3 — names that are not valid text.** An expression is text. (a) Such a name is matched as its bytes, where `.` matches one byte of it [recommended, unmeasured]; (b) such a name is never named by an expression, and the manual says so. — owner
- **Q4 — the line shown before a delete.** For a glob it spells the pattern in other cases; that cannot be derived from an expression. Recommended: `"<expression>" is a regular expression; capital letters matter (--ignore-case ignores them).` — owner
- **Q5 — `check`.** It takes the transfer verbs' pattern options (D-2026-11-20-1); recommended that it take these too. — owner
- **Q6 — before or after the 1.0 tag.** It is additive: nothing decided elsewhere changes when it lands. Recommended: after, unless the owner wants it in 1.0. — owner
