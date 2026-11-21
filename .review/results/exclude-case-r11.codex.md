The goal is to tell users what case matching will change before `copy` overwrites or `mirror` deletes, and to make `check` and retries recognize the same single-file source. I would echo the user’s pattern and show a concrete filename whose treatment changes under the full include and exclude rules. When that example is too costly to establish, I would state the matching rule. For retries, I would preserve the distinction between “the source file itself” and “a child of the source directory” through enumeration and failure accounting.

**Verdict — `acceptable_with_changes`.** The renamed-destination `check` fix is sound. Using both pattern lists and bounding the work spent on the note are also sound choices. Six changes are required.

1. **High — Preserve the root-file identity during retry.** After a failed `Report.txt` became a directory containing another `Report.txt`, `jobs retry` overwrote `out/Report.txt/Report.txt` and exited 0. The base retry selector left that destination intact and exited 2. Resolve the root marker only when the source enumerator sees a root file. This also matters to `move`, whose successful retry can authorize recursive removal of the replacement directory. [retry.rs:587](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/retry.rs:587) · [move cleanup:1196](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:1196)

2. **High — Split saved-job leftovers before translating their names.** I marked the root as an incomplete output in a failed `--ignore-existing` job record. `jobs retry` looked for the empty path on its repair pass, skipped `Report.txt` on its other pass, and exited 2 with the partial bytes still present. Both sets need to use the same identity when the split is made. [jobs.rs:308](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:308) · [retry split:362](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/main.rs:362)

3. **High — Print the filename that demonstrated the change.** With `--include '*a' --exclude '[Aa]'`, the mirror prompt said `--ignore-case would also match A`. Both mirrors preserved `A`. The file deleted only with `--ignore-case` was `xA`. The code finds a changed name, reduces that result to a boolean, and prints a different spelling. Return and print the verified name. [case_fold.rs:825](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:825)

4. **Medium — Suppress examples when the alternate matcher cannot compile.** An `s` inside 249 nested brace pairs was accepted as typed and produced an uppercase example. The same `copy --dry-run --ignore-case` exited 1 during filter validation. A failed probe should select the general wording. [case_fold.rs:569](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:569)

5. **Medium — Make the count independent of search order.** In a 33-pattern file containing `Visible.txt`, 31 case-equivalent alternatives, and `{z,2}`, moving `{z,2}` from last to second changed “1 more pattern” to “2 more patterns.” Use a neutral count of the remaining configured patterns that can be computed independently of the example-search budget. [case_fold.rs:339](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:339)

6. **Medium — Apply the budget before compiling the full alternate include list.** A one-file release dry run with 10,000 literal includes took a median **217 ms** with the head note, **64 ms** with the base note implementation, and **52 ms** with explicit `--match-case`, over five runs each. One candidate triggers compilation of the widened list. Use the general wording when that compilation would exceed the budget. [case_fold.rs:780](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:780)

**Verification.** Reading `AGENTS.md` and resolving the head SHA both succeeded. Removing the source-name override made the guard report one missing destination. Restoring the pinned file made it report one match. Linux formatting and strict Clippy passed, and the workspace suite finished with **1,803 passed, 0 failed, 2 ignored** after setting Git’s discovery ceiling for the disposable temp directory. Windows cross-target Clippy passed. Origin CI still reported the Windows and macOS runtime checks pending. The disposable copy was removed.

```text
base: d3e33acc2b3b399365bee911a8dba22773e5d8ff
head: bfa537f9884dd6bca4b15ceb4585f2ea3afc5717
capability_ok: true
guard_confirmed: true
```