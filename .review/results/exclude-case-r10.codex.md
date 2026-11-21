I would use the same compiled filter to make the transfer decision and explain it before `mirror` asks for confirmation. I would also give a single-file source one logical name, its source basename, across `copy`, `check`, and retry. The goal is for an operator to know which files a mirror can delete and to trust a subsequent verification or retry.

**Verdict — `acceptable_with_changes`.** Validating the derived exclude rule before confirmation is sound, as is preserving the empty wire path for a single-file transfer. The prompt still makes claims that the complete filter contradicts, and the basename fix stops short of two normal entry points.

**Required changes**

1. **High — preserve warnings when path coverage is uncertain.** With `--exclude a --exclude ./A`, the head printed no case warning. The default mirror deleted `sub/A`, while `--ignore-case` kept it. The reverse risk also occurs. With excludes `Keep`, `[!a]*.txt`, and `Ax.txt`, the prompt advertised additional spellings of `Keep`, yet `--ignore-case` deleted `Ay.txt`, which the default kept. A root-level example or one expansion of `*` cannot establish what happens at every path. When coverage is uncertain, retain a warning that protection may shrink. [Equivalence decision](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:783) · [Loss detection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:708)

2. **High — evaluate includes and excludes together.** With `--exclude '[!a].bin' --exclude '[!b].txt' --include '*.txt'`, the prompt singled out the loss of `A` from `[!a].bin`. Both mirrors kept `A.bin`. The file that `--ignore-case` actually deleted was `B.txt`. Each proposed example should pass through both rule lists in both modes before it gets priority in the prompt. [Filter selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:605)

3. **High — translate the single-file identity on retry.** I blocked `out/Report.txt` with a directory, ran a listed single-file copy, and removed the directory during the retry wait. The head exited 2 with `source: missing at retry` and left the file uncopied. The retry passed the empty wire path into list validation, which classified it as unnamed. Translate that internal root entry back to the source basename before building the retry filter. [List validation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:114) · [Missing-file accounting](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:1457)

4. **Medium — use the source name when verifying a renamed copy.** A listed copy from `Report.txt` to `Saved.bin` exited 0. A checksum check with the same list reported “Missing on destination” and exited 1. The unfiltered checksum check found one match. Use the source basename to select both operands, then compare their actual paths. [Root name selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/enumeration.rs:268) · [Destination enumeration](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/check.rs:72)

5. **Medium — verify the spelling and count actually printed.** With excludes `q[b-~]` and `Q[b-~]`, the prompt offered `Q[b-~]` as new even though the user had supplied that exact glob. `QB` was a genuine new match and would make an accurate example. The count also changed from two additional patterns to zero when I moved `*.Doc` from before `a, A` to after them. Use the verified example directly and apply the same counting rule in either order. [Spelling selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:689) · [Pattern count](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:277)

6. **Medium — bound the cost of generating one warning.** A one-file release `copy --dry-run` with 1,000 `archiveNNN.tmp` exclusions and the covering glob `*.[Tt][Mm][Pp]` rose from **31 ms at base to 443 ms at head**, median of five runs each. Apply a comparison budget across the command and use the general case warning when it is exhausted. [Candidate loop](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:253) · [Spelling enumeration](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:770)

On Linux, formatting and strict workspace Clippy passed. Workspace tests finished with **1,774 passed, 0 failed, 2 ignored**. Windows GNU cross-target Clippy also passed. Runtime checks were Linux-only.

For the independent guard, I restored the old empty-path lookup. `a_root_that_is_one_file_is_filtered_by_its_own_name` failed at “listed by its name” with exit 101. Restoring the pinned source made the same test pass with exit 0. I removed the disposable copy.

```text
base: df682eea76153a4ef6215c568db928a02840c046
head: 41b4bfdda307b3b0caaa911626f223b0ac314ac0
capability_ok: true
guard_confirmed: true
```