# cr-g3fix3-1: Filesystem-equivalent Unicode symlink excludes still permit mirror data loss

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-15), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix3-r1-36b752b6`; codereview re-review g3fix3-r1 over `e9dc99c9..36b752b6` (record `.review/results/g3fix3-r1.codex.json` + `.events.jsonl`); 2026-10-15 20:31–20:40Z; capability_ok=true; guard_confirmed=true (reviewer replaced `globset::escape(name)` with `name.to_string()` in `literal_glob` → `a_name_read_from_the_disk_is_matched_literally` red at `filter.rs:871`, restored by `git checkout`, blob hash verified, green; 1517/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` for blake3 — the reviewer's environment, not the change); SHAs pin-verified; closed: cr-g3fix2-2 true, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:565 — paired_name uses Unicode lowercase rather than filesystem equivalence; when pairing fails, stored_name returns None for symlinks at line 535. Reproduced on case-insensitive macOS with src/σ pointing outside the source and absolute exclude src/ς: both paths had the same symlink inode, but local and wire patterns remained ./ς with no warning. Mirror deleted dst/σ/saved.txt and its directory. An NFD-stored café symlink excluded using uppercase NFC CAFÉ behaved identically. Exact stored-spelling controls preserved both destination subtrees.

## Predicted observable failure
A valid absolute exclude naming an existing symlink through another filesystem-equivalent spelling silently fails to protect its destination counterpart; mirror deletes pre-existing data. This leaves cr-g3fix2-1 and cr-g3fix-1 materially open.

## Reviewer's suggested approach
Recover the entry's stored name using filesystem-aware lookup or candidate identity checked without following symlinks. Cover Unicode case equivalence and normalization with destination-survival tests.

## Intake
Admitted; reproduced by the orchestrator with the release binary `blit 1.0.0+36b752b6ff1e` on macOS APFS: `src/σ -> <outside>`, `blit mirror -y src/ dst/ --exclude <src>/ς` printed no warning and reported "Deleted: 1 file(s), 1 dir(s)" — `dst/σ/saved.txt` and its directory gone. This is the gap cr-g3fix2-1's Known gaps recorded ("case folding is per-character Unicode lowercase, not the filesystem's own table"), and it is a data-loss path under `mirror`, so it cannot stay a gap. Root cause: `paired_name` (`filter.rs:565`) decides equivalence by text (per-character lowercase), but the filesystem's rule is what made the typed path resolve — APFS pairs final sigma with sigma and NFC with NFD; no text rule reproduces every filesystem's table. Fix shape (the reviewer's "candidate identity checked without following symlinks"): let the filesystem decide. `symlink_metadata(typed)` already proves the typed path denotes an entry without following it; pair it with the parent listing's entry that has the same identity under `symlink_metadata` — on Unix `(dev, ino)` via `std::os::unix::fs::MetadataExt` — the link itself, never its target. Order: the entry spelled exactly as typed (no stat) → otherwise walk the listing and `symlink_metadata` each entry until the identity matches (stop at the first hit; a hard-linked identity that matches several entries takes the fold-equal one, else pairs nothing). This replaces text folding on Unix; on Windows std has no stable file identity (`file_index` is unstable), and NTFS equivalence is an upcase table with no normalization rule, so the case-fold pairing stays there and is recorded as the remaining gap. Guards: the stored-name unit test and the case guard gain a `σ` symlink excluded as `ς` and an NFD `café` symlink excluded as NFC `CAFÉ`, each run only where the filesystem pairs them (probe at test time: create `σ`, `symlink_metadata(ς)` succeeds), the pattern becoming `./σ` / `./café` (NFD) in both `build` and `build_spec`; a CLI `mirror` destination-survival test for the sigma case; a pure test of the identity pairing against a stand-in listing (exact wins, identity next, several identities need the fold, none pairs nothing); all mutation-red. Cost note (FAST): the identity walk is one `lstat` per listed entry until the hit, only when no entry is spelled as typed, once per absolute exclude component — not on the transfer's per-entry path.

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
