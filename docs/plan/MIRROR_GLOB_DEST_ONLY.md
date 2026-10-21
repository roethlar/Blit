# Mirror pure-glob excludes and destination-only entries on pairing filesystems

**Status**: Draft (r1, 2026-10-21 — owner interview in progress; decisions land here as they are made)
**Created**: 2026-10-21
**Supersedes**: the first bullet of `docs/RELEASE_NOTES_1_0.md` "Known limitations shipped open" (the pure-glob destination-only limitation), once this plan ships; extends `docs/plan/RELEASE_1_0.md` G3b under D-2026-10-19-2
**Decision ref**: pending (activation needs a `docs/DECISIONS.md` entry)

## Goal

A `mirror` exclude written as a pure glob — one with no literal component the destination can look up by name — never lets the deletion pass remove a destination-only entry that the destination filesystem pairs with a spelling the glob names. On APFS, `dst/x/bSS/` survives `--exclude '*ß'`, `dst/x/s/` survives `--exclude '??'`, and NFC `dst/x/É/` survives `--exclude '???'`, each with no source counterpart, exactly as a literal exclude already protects the destination's own spelling. Where exactness is impossible, the pass keeps more; it never deletes on a guess. This closes the first "Known limitations shipped open" item, which D-2026-10-19-2 made a 1.0 blocker, and which the owner's reproduction on 2026-10-19 showed deleting saved data (DEVLOG 2026-10-19 22:55:59Z: three forms, each exit 0, one saved file and its directory deleted).

## The defect, from the code

- `transfers::filter::destination_scope` (`crates/blit-core/src/transfers/filter.rs:816`) builds the deletion scope the destination walk is filtered by. `DestinationNames` (`:1032`) reads every exclude in the destination's own names: a literal name or literal path component through `stored_name` (the filesystem's own pairing: the exact name, else the entry of the same identity), and each path the source's walk pruned through `resolve_pruned` (cr-g3fix6-2).
- A glob rule with no literal component "is not read here: the filter matches it as written" (`:1021-1024`, cr-g3fix6-1). The glob is compiled by globset as typed: byte-oriented, case-sensitive, no normalization (`fs_enum.rs` `CompiledPatterns`; `FileFilter::excluded` at `fs_enum.rs:554`).
- With no source counterpart there is no pruned path for the entry. The destination-only entry matches nothing as written, is in scope, and is extraneous — so it is planned and deleted.
- The existing end-to-end guards (`GLOB_PAIRED` in `crates/blit-cli/tests/exclude_contract.rs:744` and `crates/blit-core/src/transfer_session/mod.rs:9298`) cover only the form WITH a source counterpart, which cr-g3fix6-2 fixed through pruned paths. The destination-only form has no guard; the release notes recorded it as a limitation.

Forms, as the filesystem pairs them. `?` counts bytes in this filter (globset is byte-oriented), so `??` names the two-byte `ſ` and not `s`, and `???` names the three-byte NFD `é` and not the two-byte NFC `É`:

| Exclude | Destination-only entry | Pairs where | Reproduced |
| --- | --- | --- | --- |
| `*ß` | `bSS` | APFS (full case fold: `ß` ↔ `SS`) | 2026-10-19, APFS |
| `??` | `s` | APFS (`ſ` ↔ `s`) | 2026-10-19, APFS |
| `???` | NFC `É` | APFS (NFD ↔ NFC) | 2026-10-19, APFS |
| `*cache` | `aCache` | NTFS and APFS (ASCII case) | **assumption from the code, not yet reproduced**: the glob is compiled case-sensitive, so the ASCII form should fail the same way on Windows; slice 1's guard decides |

Measured 2026-10-21 on this repo's Windows host (NTFS): the filesystem pairs simple case mappings only (`Übung` ↔ `übung`), and does NOT pair `ς` ↔ `σ`, NFD ↔ NFC, `ẞ` ↔ `ß`, `ſ` ↔ `s`, `µ` ↔ `μ`, or any Greek symbol letter to its base letter. The APFS rows are therefore macOS-only in practice; the ASCII row is the Windows-visible one.

## Prior art (owner question 2026-10-21: "how does robocopy do it? how does rsync do it?")

**Robocopy** (`robocopy.exe` on this repo's Windows Server 2022 host, measured 2026-10-21 with `/MIR` and a destination-only entry plus an unrelated extra file under a three-character parent):

| Exclude | Destination-only entry | Survived | Unrelated extra deleted |
| --- | --- | --- | --- |
| `/XD *cache` | `aCache\` | yes | yes |
| `/XF *CACHE.TXT` | `aCache.txt` | yes | yes |
| `/XD *ß` | `bSS\` | **no** | yes |
| `/XD ??` | `ab\`, `s\`, `ſ\` | yes, yes, yes | yes |
| `/XD ??` | `abc\` | no | yes |
| `/XF ???` | NFC `É`, NFD `é` | yes, yes | yes |
| `/XF ???` | `abcd` | no | yes |

What that shows: (1) robocopy PROTECTS excluded extras under `/MIR` — an `/XD`/`/XF` match on the destination side keeps the entry out of the deletion; (2) its wildcard engine is Windows' own: case-insensitive by the OS upcase table, UTF-16 code units, and `?` matches zero or one character (DOS semantics — `??` keeps every one- and two-character name, so the earlier run with a one-character parent `x` excluded the parent itself); (3) it therefore agrees with NTFS by construction and knows nothing NTFS does not: `*ß` does not keep `bSS` because NTFS does not pair them. Robocopy has no notion of NFC/NFD or `ß`/`SS` pairing because its platform has none.

**rsync** (read from source, `github.com/rsyncproject/rsync` at `a7404a9a`, cloned by the owner to `D:\source\rsync` 2026-10-21; not run — rsync is not installed on this host and WSL is absent): the receiver's delete pass (`generator.c` `delete_in_dir`) lists each destination directory with `get_dirlist`, which walks it under `ALL_FILTERS` (`flist.c:2339`, `send_directory` → `is_excluded`), so a destination name an exclude matches is never listed and never a deletion candidate — the manual's "an exclude protects a file from being removed on the receiving side" (`rsync.1.md:4361`), overridden only by `--delete-excluded` or an include. Matching (`lib/wildmatch.c` `dowild`) walks `uchar` bytes: `?` matches exactly one byte other than `/` (`:100-103`), literals compare byte for byte, and the only case folding is `iwildmatch`'s ASCII-only `tolower` (`ISUPPER` requires `ISASCII`), used for daemon host tokens (`access.c`, `token.c`) and never for filter rules (`exclude.c:1062` uses `wildmatch_array`/`litmatch_array`). No normalization anywhere in the filter path; `--iconv` converts names in transit only. So rsync protects excluded extras by the receiver's own BYTES: on APFS, `rsync --delete --exclude '*ß'` deletes a destination-only `bSS`, and `--exclude '??'` deletes `s`, exactly as blit does today — the identical hole, unaddressed.

Implication: both tools protect excluded extras; robocopy is exact only because its pattern semantics ARE its platform's name semantics, and rsync does not try. Blit can have robocopy's property on Windows by letting the filesystem match the pattern (option (E) below); APFS exposes no such API, so on macOS the Unicode tables (option (A)) are the only route to the same consistency.

## Non-goals

- Changing what a glob EXCLUDES at the source (its byte-oriented, as-written reading, cr-g3fix6-1). This plan changes only what the destination's deletion scope PROTECTS.
- Literal names and literal path components: already exact through `stored_name` (cr-g3fix5-1/-2) and pruned paths (cr-g3fix6-2).
- Case-insensitive Linux mounts (`ext4` casefold, VFAT): consistent with `AUDIT_2026_10_20`'s non-goal; the destination probe `case_insensitive_at` would in fact detect them, so whatever lands applies there too, but no guard is written for them.
- Wire protocol or `CONTRACT_VERSION` changes: none are needed; the fix is destination-local, where the deletion scope is built.
- Any new user-visible CLI option (SIMPLE; owner ruling 2026-08-01).

## Constraints

- **RELIABLE over complete**: where the destination's pairing rule cannot be decided from text, the pass keeps the entry. Over-keeping a stale destination-only entry is the safe direction; deleting a kept one is the defect.
- **FAST**: zero extra work when no pure-glob exclude is in scope, when the destination compares exactly (`case_insensitive_at` false), or when the walk visits a directory holding no destination-only candidate. On a pairing destination the extra work per destination-only candidate per pure-glob rule must be bounded by the name's length times a small constant (no enumeration of spellings), and any one-time table build must be lazy and happen only on such a run.
- A fold or equivalence is used only to WIDEN protection, never to narrow it: the as-written match stays, and the equivalence reading is OR-ed with it. (cr-g3fix6-1's bug was a fold that narrowed: `[A-z]` folded to `[A-Z]` lost `_`.)
- Platform truth: every guard that can go red only where the filesystem pairs must say so and skip with a message elsewhere, as `GLOB_PAIRED` does; the macOS CI leg is where the APFS rows bite, Windows where the ASCII row does.
- One finding per commit; each slice carries a mutation-proven guard where the platform allows, and names the platform where it cannot.

## Acceptance criteria

- [ ] **AC-1**: on a destination that pairs spellings, a `mirror` with a pure-glob exclude and NO source counterpart exits 0 and leaves every destination-only entry the filesystem pairs with a spelling the glob names, with its contents and directory, in place — the four forms above, each run where the filesystem pairs it (APFS: all four; NTFS: the ASCII form; ext4: skipped with a message), and an unrelated extraneous entry is still deleted.
- [ ] **AC-2**: a destination that compares exactly (ext4, case-sensitive APFS) is unchanged: a destination-only entry a pure glob does not match as written is deleted, and no equivalence table is built.
- [ ] **AC-3**: a run with no pure-glob exclude, or with `--files-from`, performs no additional work (no table build, no extra matching) — asserted by an observable count in tests, not by inspection.
- [ ] **AC-4**: where the equivalence reading cannot decide (a glob whose automaton cannot be built within the existing cap, or a name whose class the tables do not cover), the entry is kept, and the run says so once in its summary and job log, in plain words, without a flag.
- [ ] **AC-5**: `docs/RELEASE_NOTES_1_0.md` no longer lists the limitation; `docs/TRANSFER_SESSION.md` and `README.md` describe what a pure-glob exclude protects at a pairing destination.

## Design

### Why a text rule is admissible here, when cr-g3fix6-1 rejected one

cr-g3fix6-1 removed a fold of the glob's TEXT because it changed what the glob EXCLUDED (narrowed a class). The destination scope asks a different question: "would this filesystem pair the destination's name with SOME spelling the glob names?" No API answers that for a glob (the filesystem pairs names, not patterns), so the only exact answer comes from the filesystem's own pairing tables — Unicode case folding and canonical normalization, which is what APFS and NTFS implement — applied to WIDEN protection only. The as-written match remains; the equivalence reading can only add kept entries.

### Candidate mechanisms (owner decision Q1 selects)

**(A) Exact under the Unicode tables, keep where undecidable — recommended.** For a destination-only entry with name `N` in a directory where a pure-glob rule `G` is in scope, decide "∃ spelling `S` paired with `N` such that `G` matches `S`" without enumerating spellings: walk `G`'s byte DFA (the `regex_automata` dense DFA `DestinationNames` already builds for crossing globs, `filter.rs:1070`, under `MAX_CROSSING_GLOB_DFA_BYTES`) over `N` one position at a time, where each position offers every member of that character's equivalence class (the characters whose full case fold equals this one's, plus the NFD and NFC forms) as alternative byte strings; the state set after the last position accepts iff some paired spelling matches. Linear in `|N|` × class size × member bytes. Classes come from a lazily built inverse full-case-fold table (one pass over Unicode scalar values on first use, only on a pairing destination with a pure-glob rule in scope) and from `icu_normalizer` (already in `Cargo.lock` via `idna`); full case folding needs `icu_casemap` (new direct dependency, same ICU4X family) — owner decision Q2. If `G`'s DFA cannot be built, or `N` holds a character the tables do not fold, the rule keeps every destination-only entry in its scope and the run reports it (AC-4). Covers all four forms exactly (`??` ↔ `s` because `ſ` is in `s`'s class and is two bytes).

**(B) Widening fold of both sides.** Match `G` against `N` as written, and also fold(`G`'s literal text) against fold(`N`) and NFD/NFC forms. Cheap, no DFA walk. Covers `*ß`/`bSS`, `???`/`É`, `*cache`/`aCache`; MISSES `??`/`s` and every form that depends on a paired spelling's byte length, because folding yields one representative, not the class. Not perfect; fails D-2026-10-19-2 unless the owner rules the byte-length forms out of scope.

**(C) Keep every destination-only entry in a pure-glob rule's scope on a pairing destination.** No tables. Perfectly safe, but a `mirror --exclude '*.o'` on macOS would stop deleting any stale destination-only file anywhere the rule reaches (an any-depth rule reaches everywhere): the mirror no longer mirrors. Acceptable only as (A)'s fallback for the undecidable case.

**(D) Refuse** a `mirror` whose excludes include a pure glob when the destination pairs spellings. Safe and simple to state; removes a working feature for every macOS and Windows user. Not recommended.

**(E) Let the filesystem match the pattern, where it can — Windows only.** `FindFirstFileW(dir\pattern)` has NTFS apply a wildcard under its own upcase table (robocopy's property, by construction). Its dialect is `*` and `?` only, with `?` matching zero or one UTF-16 unit; a globset component holding only `*`, `?` and literals translates (the lenient `?` only widens protection), classes and alternations do not and fall back to (A) or (C). Adds a platform-specific path for a platform where the Unicode tables already over-approximate safely; worth it only if the owner wants Windows to behave exactly as robocopy does rather than slightly more conservatively.

**(B′) Widening fold with robocopy's lenient `?`.** (B) plus NFC/NFD forms and a destination reading of `?` as zero or one character. Covers all four reproduced forms (`??` keeps `s` as it keeps every one-character name) at (B)'s cost, with robocopy-style over-protection of short names. A pragmatic middle if (A)'s table and automaton walk are judged too heavy; still needs full case folding for `ß`/`SS`.

### Where the change lives

- `crates/blit-core/src/transfers/filter.rs`: `DestinationNames::new` compiles the pure-glob rules it currently skips (`:1173`) into a new step kind read only on pairing destinations (`case_insensitive` already carried, `:1041`); `protects` consults it for entries the walk visits that no source path, pruned path, or literal rule already protects. Any-depth pure globs apply in every directory; path-shaped pure globs (`a*/b?`) follow the existing `Step` machinery with each glob component read by the equivalence walk.
- `crates/blit-core/src/transfers/name_equivalence.rs` (new): the inverse full-case-fold table (lazy), NFD/NFC forms, `class(c)`, and `glob_names_some_spelling(dfa, name) -> Decision { Yes, No, Undecidable }`. Pure, testable on every OS.
- `crates/blit-core/src/transfer_session/mod.rs` `mirror_delete_pass` and the summary: the AC-4 count and sentence.
- Tests: `exclude_contract.rs` and `transfer_session/mod.rs` gain the destination-ONLY table beside `GLOB_PAIRED`; `filter.rs` gains stand-in-destination unit tests that run on every OS; `name_equivalence.rs` unit tests pin `ß`/`SS`, `ſ`/`s`, NFD/NFC, Kelvin `K`/`k`, final sigma, and an undecidable case.

### Risks

- A new direct dependency (`icu_casemap`) for full case folding; `icu_normalizer` is transitively present already. Owner decision Q2; the alternative is a vendored CaseFolding.txt table (status C+F), generated once and checked in, no new crate.
- The inverse fold table's first-use cost (one pass over ~1.1 M scalar values) on a pairing destination with a pure-glob rule: estimated tens of milliseconds, once per process; to be measured in slice 2 before acceptance, and restricted to the Basic Multilingual Plane if the measurement says so.
- NTFS folds less than the tables (simple upcase only): the tables over-keep there (`ſ`-paired names kept that NTFS would not pair). Safe direction; noted in docs.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop.

1. **Guard first**: the destination-only table (`GLOB_DEST_ONLY`, four forms) as end-to-end guards in `exclude_contract.rs` and `transfer_session/mod.rs`, each run where the filesystem pairs and skipped with a message elsewhere, and required not to pass vacuously; expected RED on macOS CI (and, if the assumption holds, on Windows for the ASCII row). Lands as a failing guard only after the owner's go; no production change.
2. `name_equivalence`: tables, classes, forms, the DFA walk, unit tests on every OS, the first-use cost measured and recorded.
3. `DestinationNames` reads pure-glob rules on pairing destinations through the equivalence walk; undecidable → keep; stand-in-destination unit tests; slice 1's guards go green on macOS.
4. AC-4 reporting (summary sentence and job log note) and the docs changes (AC-5); release notes limitation removed.

## Open questions

- **Q1 — guarantee level**: (A) exact under the Unicode tables with keep-when-undecidable [recommended], (B) widening fold only (misses byte-length forms such as `??`/`s`), (B′) widening fold with robocopy's lenient `?`, (C) keep every destination-only entry in a pure glob's scope on pairing destinations, (D) refuse, (E) filesystem-matched patterns on Windows as an addition to (A) or (B′). — owner
- **Q2 — full case folding source**: `icu_casemap` as a new direct dependency, or a generated, checked-in CaseFolding table with no new crate. — owner (asked after Q1 if (A) or (B) is chosen)
- **Q3 — AC-4 surfacing**: one summary sentence plus a job-log note when a rule was kept conservatively, or job log only. — owner
- **Q4 — the ASCII row**: if slice 1 shows `*cache`/`aCache` is deleted on NTFS today, it is covered by the same mechanism; confirm it is in scope rather than a separate finding. — owner
