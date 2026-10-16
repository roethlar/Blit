# cr-g3fix4-3: Typed Unix escapes still change deletion scope on a Windows peer

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:204 — a bare backslash becomes '/', including in received patterns. transfers/filter.rs:228 leaves Unix relative escapes unchanged before serialization. A parameterized compiler probe confirmed that wire pattern ./a\b excludes ab/saved.txt under the Unix rule but allows it under the Windows rule; Windows instead excludes a/b/saved.txt. Both destination paths are valid Windows names. This was tested through the actual rule compiler on macOS, not a Windows runtime.

## Predicted observable failure
A Unix-to-Windows mirror using --exclude './a\b' omits source ab while leaving destination ab inside deletion scope, allowing existing excluded data to be deleted. It also incorrectly protects a/b.

## Reviewer's suggested approach
Translate typed Unix escapes into a portable wire representation before serialization and compile received patterns consistently. Preserve native Windows separator handling for locally supplied patterns, including .\cache.

## Intake
Admitted on the reviewer's parameterized-compiler analysis (the Windows rule as a parameter, run on macOS; no Windows runtime), the Unix half reproduced by the orchestrator with `blit 1.0.0+548625f5c7cf`: `blit copy src/ dst/ --exclude './a\b'` leaves `ab` out and copies `a/b`, so the source side excludes `ab` while, per the analysis, a Windows peer compiling the same wire text (`fs_enum.rs:204`, and globset's own Windows reading of a bare `\`) keeps `ab` inside its deletion scope and shields `a/b` instead. cr-g3fix3-3's Known gaps recorded exactly this and called it a wider dialect change; it is a mirror data-loss path, so it cannot stay a gap. Fix shape (the reviewer's): a typed Unix escape never leaves the typed entry. Under the Unix rule `resolve_excludes_on` rewrites every bare `\c` outside a bracket class into the class dialect — `globset::escape(c)`, i.e. `[c]` for a glob metacharacter and `c` itself otherwise, `\\` → `[\]` — bracket-aware through `bracket_class_len`, a trailing lone `\` left for validation to refuse as today. The rewrite applies to relative patterns and to the typed parts of a resolved absolute pattern (`written`), so `build` (the local filter) and `build_spec` (the wire form) both carry the canonical dialect, which holds no bare `\`. Under the Windows rule `\` is already a separator and nothing changes; a Windows user's `.\cache` still anchors. The receiving peer's `compile` is untouched: its reading of a bare `\` no longer matters because none arrives from a 1.0 peer. `--include` is not rewritten (it takes no part in the deletion scope; record it). Guards: unit — `resolve_excludes_on(["./a\\b", "x\\*y", "[\\]z", "p\\\\q", "./cache"], None, false)` gives `["./ab", "x[*]y", "[\\]z", "p[\\]q", "./cache"]`, and the output compiled under the Windows rule excludes `ab/saved.txt` and leaves `a/b/saved.txt` in scope (parameterized, every OS); `build_spec` carries the rewritten form; the CLI `copy --exclude './a\b'` still leaves `ab` out and copies `a/b` (Unix); cr-g3fix-3's and cr-g3fix3-3's guards stay green; mutation-red.

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
