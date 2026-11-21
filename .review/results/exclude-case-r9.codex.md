The goal is to let an operator predict what a mirror will copy and what it may delete before answering the confirmation prompt. I would compile one effective filter per run, use exact matching by default, and widen literal letters in the glob for `--ignore-case`. That preserves the byte semantics of `?` and bracket classes and keeps `--files-from` names exact. The prompt should show a concrete filename whose treatment changes, giving priority to any file that would lose protection.

**Verdict** `acceptable_with_changes`. I would keep the pattern-widening design, the saved job’s snapshot of pattern-file contents, and the versioned filter sent to daemons. Three changes are required.

1. **High — reject rules that cannot be compiled as executed.** On Linux, the prompt for `--exclude 'keep\/'` advertised `KEEP\/` as an additional match. The `--ignore-case` mirror kept the file `keep\` and deleted `KEEP\`. Supplying `KEEP\/` explicitly protected `KEEP\`. Trimming the trailing slash leaves a dangling escape in the derived rule, and validation admits it into a case-sensitive text fallback. Validate every derived rule and refuse an unrepresentable rule before confirmation. [Rule construction](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/fs_enum.rs:134) · [Compiler fallback](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/fs_enum.rs:281)

2. **High — show a verified loss of protection first.** With `--exclude Keep --exclude '{[!ab].txt,A.txt}'`, the prompt described only the additional spellings of `Keep`. The exact-mode mirror kept both `A.txt` and `B.txt`. The `--ignore-case` mirror deleted `B.txt`. The warning requires every candidate letter to lose its match, so the alternative that preserves `A.txt` hides the loss of `B.txt`. One verified loss should be enough to show it. Check gains against the complete filter too: `--exclude a --exclude A` advertised `A` as an additional match, although both runs already protected it. [Loss check](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:564) · [Single-pattern probe](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:484)

3. **High — use the same filter name for a single-file root.** With two unequal files named `Report.txt`, `copy --ignore-case --exclude './report.txt'` skipped the file, while `check` with those options reported a difference. With `Report.txt` in `--files-from`, `check` reported zero files and success on the unequal pair. `copy` wrote the file and then exited 2, reporting it missing. Use the basename consistently for filtering and missing-file accounting, preserving the empty relative path used on the wire. [Enumeration](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/enumeration.rs:260) · [Transfer filtering](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/source.rs:1225) · [Missing-file accounting](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1425)

Reading `AGENTS.md` and resolving the pinned head both succeeded. Disabling ASCII case widening made the source-walk guard fail on `A*TMP` against `a/b/u.tmp`. Restoring the exact Git blob made the same test pass.

Linux formatting, Clippy, and workspace tests passed, with **1,718 passed and 2 ignored**. Windows cross-Clippy passed. Native Windows and macOS behavior remains unverified, and hosted CI was unavailable through the unauthenticated CLI. I removed the disposable copy and verified the shared tracked tree was clean.

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: ab7331c34710feb88e1aea6aff55ee48529311be
capability_ok: true
guard_confirmed: true
```