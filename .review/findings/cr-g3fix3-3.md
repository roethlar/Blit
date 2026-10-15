# cr-g3fix3-3: Windows peers reinterpret escaped Unix backslashes as directory separators

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-15), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix3-r1-36b752b6`; codereview re-review g3fix3-r1 over `e9dc99c9..36b752b6` (record `.review/results/g3fix3-r1.codex.json` + `.events.jsonl`); 2026-10-15 20:31–20:40Z; capability_ok=true; guard_confirmed=true (reviewer replaced `globset::escape(name)` with `name.to_string()` in `literal_glob` → `a_name_read_from_the_disk_is_matched_literally` red at `filter.rs:871`, restored by `git checkout`, blob hash verified, green; 1517/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` for blake3 — the reviewer's environment, not the change); SHAs pin-verified; closed: cr-g3fix2-2 true, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:466 — a resolved Unix name back\slash is emitted as ./back[\]slash. On Windows, fs_enum.rs:83 invokes normalize_separators, whose line 147 rewrites this to ./back[/]slash. A temporary test against the actual ExcludeRules::compile branches confirmed that the Unix rule leaves back/slash/saved.txt in scope while the Windows rule excludes it. The remote mirror consumes these rules through transfer_session/mod.rs:4181. This was verified through the parameterized compiler on macOS, not a Windows runtime.

## Predicted observable failure
A Unix-to-Windows mirror excluding an aliased directory containing a literal backslash also excludes the unrelated destination subtree back/slash from deletion, retaining stale files outside the requested exclusion.

## Reviewer's suggested approach
Preserve a canonical wire-pattern dialect and normalize native separators before escaping, without rewriting escaped literals on the receiving peer. Test Unix-generated wire patterns under both platform rules.

## Intake
Admitted on the reviewer's analysis (the parameterized `ExcludeRules::compile` under the Windows rule, run on macOS; no Windows runtime), the same footing as cr-g3fix-3. cr-g3fix2-2's Known gaps noted the rewrite and dismissed it because no Windows name can hold `\`; the consequence it missed is that the rewritten class `[/]` then matches a real separator, so `back/slash` — an unrelated destination subtree — leaves the mirror's deletion scope. Root cause: cr-g3fix-3 put the typed-input rule "on Windows `\` is a separator" into `ExcludeRules::compile` (`fs_enum.rs:83`), which is also where every wire pattern a daemon receives (`filter_from_spec`, consumed at `transfer_session/mod.rs:4181`) is compiled, so a canonical wire pattern is re-read as typed input. Fix shape (the reviewer's): the wire dialect is canonical — `/`-separated, literals escaped — and the receiving peer never rewrites it; native separators are normalized once, where typed input enters, before escaping. Mechanism is the agent's call between (a) moving the normalization out of `compile` to the typed entry points (`resolve_excludes_on` already does it; find every other path a typed pattern takes into a `FileFilter` on Windows, so a Windows user's `.\cache` still anchors through the CLI, and redefine cr-g3fix-3's embedder guard `backslash_forms_match_on_windows` only with the embedder contract stated in the `FileFilter`/`ExcludeRules` doc comments and the finding doc) and (b) keeping `compile`'s normalization but leaving a bracket class untouched (`[\]` produced by `literal_glob`; the class syntax `globset::escape` emits is `[x]` per character) — (a) is cleaner and is the recommendation, (b) is the smaller change; either way record the reasoning. Guards: a Unix-generated wire pattern `./back[\]slash` compiled under the Windows rule must leave `back/slash/saved.txt` in scope and must still not match it under the Unix rule; a Windows-typed `.\cache` through `resolve_excludes_on` under the Windows rule still anchors; cr-g3fix-3's existing guards stay green (or are redefined with the reasoning); all mutation-red; the `cfg(windows)` runtime proof is CI's Windows leg.

## What
(filled by the fix)

## Approach
(filled by the fix)

## Files changed
(filled by the fix)

## Guard proof
(filled by the fix)

## Known gaps
(filled by the fix)

## Reviewer comments
(filled by the re-review)
