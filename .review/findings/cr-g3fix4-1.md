# cr-g3fix4-1: Unicode symlink pairing still fails for hard links and on Windows

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:616 — multiple identity matches fall back to single_fold; stored_name then returns None for an unpaired symlink at line 539. Reproduced on macOS: src/σ points outside the source, src/alias is a hard link to that symlink, and absolute src/ς resolves to the same inode with nlink=2. Mirror deleted dst/σ/saved.txt and its directory, exited 0, and issued no exclusion warning. The exact-spelling control preserved them. Separately, line 583 retains paired_by_case on Windows; a pure probe confirmed that it cannot pair ς with σ.

## Predicted observable failure
A valid filesystem-equivalent absolute exclude silently loses protection when the symlink has another hard-link name. Mirror deletes explicitly excluded destination data. Windows retains the same failure wherever filesystem equivalence succeeds but Unicode lowercase comparison does not.

## Reviewer's suggested approach
Recover the particular directory entry's stored name through filesystem-aware, non-following lookup on each platform. Do not substitute lowercase comparison when identities have multiple names. Add hard-linked symlink survival coverage and Windows filesystem-equivalence coverage.

## Intake
Admitted; reproduced by the orchestrator with the release binary `blit 1.0.0+548625f5c7cf` on macOS APFS (`scratchpad/g3fix4/repro.out`): `src/σ -> <outside>`, `src/alias` hard-linked to that symlink with `linkat` and no follow (macOS `ln` follows the link; `os.link(follow_symlinks=False)` does not), both `nlink=2`; `blit mirror -y src/ dst/ --exclude <src>/ς` printed no warning and reported "Deleted: 1 file(s), 1 dir(s)" — `dst/σ/saved.txt` gone; the exact spelling `<src>/σ` kept it. This is cr-g3fix3-1's recorded hard-link gap plus its Windows gap, both data-loss paths under `mirror`, so neither can stay a gap. The orchestrator probed the macOS stored-name APIs the reviewer's "filesystem-aware, non-following lookup" points at: `fcntl(F_GETPATH)` on an `O_SYMLINK` descriptor and `getattrlist(ATTR_CMN_NAME, FSOPT_NOFOLLOW)` both report the vnode's cached name — `σ` even for a path typed exactly `alias` — so no macOS API names the directory entry a hard-linked lookup matched, and a text rule cannot either (the filesystem's equivalence is its own). Fix shape, in two parts: (1) **identity everywhere.** On Windows `listed_name` pairs by identity too — volume serial number plus file index from `GetFileInformationByHandle` on a handle opened with `CreateFileW(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)` (the entry itself, never a reparse target; the `windows` crate's `Win32_Storage_FileSystem` feature is already enabled in `crates/blit-core/Cargo.toml`); `nNumberOfLinks` is the Windows `nlink`. `paired_by_case` and `single_fold` are retired: no text folding anywhere in the pairing. (2) **a tie pairs every tied name.** When several listed names share the typed entry's identity and none is spelled exactly as typed, `stored_name` cannot know which one the filesystem matched, so it returns all of them and the pattern expands to one literal pattern per name (each respelled form in `build` and `build_spec`): the entry the user named is excluded whichever directory entry was matched, and the other hard links to the same entry are excluded with it — over-exclusion, the safe direction, in a case that needs a hard-linked entry typed in a spelling only the filesystem pairs. The expansion is named once on stderr the way an unmatchable pattern is (`<pattern>: 2 names share this entry here; excluding σ, alias`) — a note, not a knob. The walk is still at filter build only, once per typed component of an absolute exclude; never on the per-entry path. Guards: the pure pairing test (a tie returns every tied name; exact still wins with no lookup; a single identity match unchanged); the stored-name test with a hard-linked `σ` typed `ς` → both names (Unix, where the filesystem pairs them; make the hard link with `libc::linkat(..., 0)` — `std::fs::hard_link` follows on some platforms — and skip if it fails); the case guard: `build` and `build_spec` carry both `./σ` and `./alias`; a CLI `mirror` survival test with the hard-linked symlink (Unix); on Windows, a `cfg(windows)` test that `listed_name` pairs `CACHE` with a hard-linked `cache` file by identity (`std::fs::hard_link` needs no privilege there) and `ς` with `σ` where NTFS pairs them (probe first) — type-checked by the Windows cross-clippy, run by CI's Windows leg; every guard that runs here mutation-red.

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
