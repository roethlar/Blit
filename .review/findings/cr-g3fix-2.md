# cr-g3fix-2: Canonicalizing the whole pattern fails for glob paths through symlinks

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (this commit)
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/transfers/filter.rs:232 — canonicalize(pattern) requires the wildcard-containing path itself to exist. Reproduced with alias pointing to src: copying src/ with an absolute exclude alias/*.txt warned that it could not match and copied keep.txt. The corresponding literal exclude alias/keep.txt worked.

## Predicted observable failure
An absolute glob through a symlink alias, including an alternate /tmp spelling on macOS, fails to exclude matching files even though its directory resolves under the source.

## Reviewer's suggested approach
Canonicalize the existing literal prefix, preserve the glob suffix, and then relativize against the source-root forms.

## Intake
Admitted. `relative_to_root` canonicalizes the whole pattern, so an absolute glob through a symlink (`/alias/*.txt`, `/tmp/...` vs `/private/tmp/...` on macOS) fails to canonicalize — the glob path does not exist — and is reported as unmatchable, while the literal `/alias/keep.txt` works. Reproduced by the reviewer. Fix shape: split the pattern at the first component holding a glob metacharacter, canonicalize the existing literal prefix, re-append the glob suffix unchanged, then relativize against the source-root forms (as typed, then resolved). This also supplies cr-g3fix-1's on-disk spelling for the literal prefix. Guard: an absolute glob through a symlinked root excludes its matches.

## What
An absolute `--exclude` glob written through a symlink — to the source (`/alias/*.txt`), to a directory inside it, or through macOS's `/tmp` for `/private/tmp` — is now made source-relative instead of being reported as unable to match. Only the pattern's literal leading part is resolved; the glob is re-appended exactly as typed.

## Approach
- `resolve_excludes` no longer canonicalizes the whole pattern (a glob path never exists, so that always failed for a glob). Its second pattern form now comes from `through_resolved_root` (new): the pattern's leading ancestors, up to the first component holding a glob metacharacter, are resolved shortest first; the first that exists and lands at or below the resolved source root is written resolved, and the rest of the pattern is re-appended unchanged. That form is relativized against the source-root forms (as typed, then resolved) exactly as before, and the typed form is still tried first.
- `has_glob_meta` (new): a component holding `*`, `?`, `[` or `{` — or, off Windows, `\`, which globset reads as an escape there (on Windows `\` is a separator and never reaches a component).
- Decision (within the ruled shape, a narrower "existing literal prefix"): resolution stops at the first ancestor that lands inside the source instead of canonicalizing the whole literal prefix. Canonicalizing all of it would also resolve a symlink *inside* the source, so `/alias/link` would become the link's target (`./sub`); the walk does not follow symlinks, so that names a different entry than the one the user typed. Two consequences, both pinned by the guard: a tail that does not exist yet under an existing ancestor (`/alias/new/*.tmp`) is made relative (`new/*.tmp`) instead of unmatchable, and a `..` after the landing point is folded by a longer ancestor's resolved form rather than accepted.
- The source root is canonicalized once per call and used for both the root forms and the new resolution.

## Files changed
- `crates/blit-core/src/transfers/filter.rs:198-236` — `resolve_excludes`: the root is resolved once; the second pattern form comes from `through_resolved_root` instead of `canonicalize(pattern)`.
- `crates/blit-core/src/transfers/filter.rs:251-300` — `through_resolved_root` and `has_glob_meta` (new).
- `crates/blit-core/src/transfers/filter.rs:577-613` — `an_absolute_glob_through_a_symlink_becomes_relative` (guard).
- `crates/blit-cli/tests/exclude_contract.rs:184-218` — `mirror_exclude_absolute_glob_through_a_symlinked_source` (guard, CLI end to end).

## Guard proof
- Guards: blit-core `transfers::filter::tests::an_absolute_glob_through_a_symlink_becomes_relative` (Unix; with `alias -> src`, `into_sub -> src/sub` and `src/link -> src/sub`, the patterns `alias/*.txt`, `into_sub/*.log`, `alias/new/*.tmp`, `alias/link` must become `./*.txt`, `sub/*.log`, `new/*.tmp`, `./link`, none unmatchable) and blit-transfer `exclude_contract::mirror_exclude_absolute_glob_through_a_symlinked_source` (Unix; `blit mirror src/ dst/ --exclude <tmp>/alias/*.txt` must leave out `keep.txt`, copy `other.dat`, and print no "cannot match anything" line).
- Mutation: in `resolve_excludes`, `through_resolved_root(pattern, root, windows)` replaced with `std::fs::canonicalize(pattern)` mapped to a string — the pre-fix whole-pattern canonicalization.
- Red: `cargo test -p blit-core --lib transfers::filter::tests::an_absolute_glob_through_a_symlink_becomes_relative` → FAILED, "assertion `left == right` failed", `left: ["/var/folders/…/alias/*.txt", "/var/folders/…/into_sub/*.log", "/var/folders/…/alias/new/*.tmp", "./sub"]`, `right: ["./*.txt", "sub/*.log", "new/*.tmp", "./link"]` — the three globs kept as typed (unmatchable) and the link resolved to its target. `cargo test -p blit-transfer --test exclude_contract mirror_exclude_absolute_glob_through_a_symlinked_source` → FAILED at `exclude_contract.rs:208`, "the glob through the alias must exclude keep.txt".
- Restore: the fixed `filter.rs` copied back, md5 `1d72156d8cf4380ae24131b60a5a1742` matched the saved fixed file; both commands → 1 passed, the guard `ok`.

## Known gaps
- Both guards are Unix-only (creating a symlink on Windows needs privilege). The Windows forms through `through_resolved_root` (canonical `\\?\` paths, `Path::ancestors` over either slash) are by reading only.
- Observation, not changed here: a relativized glob keeps globset's semantics, where `*` also matches `/`, so `./*.txt` (and `/src/*.txt`) leaves out `.txt` files below the top level as well. Pre-existing since audit-19; the guard asserts only the top-level file.
- Verified on macOS (aarch64) only, plus Linux cross-clippy at the final gate; Linux and Windows runtime tests were not run.

## Reviewer comments
(filled by the re-review)
